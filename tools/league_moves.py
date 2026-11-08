# SPDX-License-Identifier: AGPL-3.0-or-later
"""Test, and if it holds up fit, a per-move price adjustment from logged random leagues (CR7,
20 §9.3, D71).

    python tools/league_moves.py out/fit1 out/fit2=out/league2.txt out/fit3 ...

Each argument is a league's `--log` prefix; its champions' moves come from
`<prefix>.champions.csv` (the `moves` and `passive` columns), or, for leagues logged before those
columns existed, from the table the league printed (`prefix=table.txt`).

League champions all cost the same, so a move whose team wins more than its price says would be
priced too cheap. The model: a logistic regression of "blue won" on, for every ability and
passive, how many of blue's champions carry it minus how many of red's, with an L2 pull toward
zero. **First it checks that this predicts**: leaving each league out in turn, fitting on the
others and scoring the left-out one (log-loss; a coin flip is 0.693). Only if some strength of
pull beats the coin flip does it fit all the leagues and print the points each move should cost
more (or less), for a table in `mftr_sim::balance`. If none does, the moves' effects don't carry
from one league to the next, and no per-move price would help.

(2026-11-08, three leagues of 200: no strength beat the coin flip; see 08 CR7.)
"""

import csv
import math
import re
import sys

POINT = 0.025  # log-odds per price point (the parts regressions' per-point coefficients, 0.02–0.03)
LIMIT = 12.0  # at most this many points either way
PULLS = (2, 20, 60, 150, 400)


def champions(prefix, table):
    out = {}
    with open(f"{prefix}.champions.csv", newline="") as f:
        rows = list(csv.DictReader(f))
    if rows and "moves" in rows[0]:
        for r in rows:
            out[int(r["champion"])] = r["moves"].split("|") + ["passive:" + r["passive"]]
        return out
    # Older logs: the league's printed table, "League 05  90.8  92  87%  A / B / C / D · Passive".
    pat = re.compile(r"^League (\d+)\s+[\d.]+\s+\d+\s+\S+\s+(.*?) · (.*)$")
    with open(table, encoding="utf-8") as f:
        for line in f:
            m = pat.match(line.strip())
            if m:
                out[int(m.group(1))] = [s.strip() for s in m.group(2).split(" / ")] + ["passive:" + m.group(3).strip()]
    return out


def league(arg):
    # `prefix=table.txt` (not a colon: Windows paths have one after the drive).
    prefix, _, table = arg.partition("=")
    champs = champions(prefix, table)
    rows = []
    with open(f"{prefix}.matches.csv", newline="") as f:
        for r in csv.DictReader(f):
            x = {}
            for side, sign in (("blue", 1), ("red", -1)):
                for c in r[side].split():
                    for m in champs.get(int(c), []):
                        x[m] = x.get(m, 0) + sign
            rows.append(({k: v for k, v in x.items() if v}, 1.0 if r["winner"] == "blue" else 0.0))
    return rows


def fit(rows, l2, steps=1500):
    feats = sorted({k for x, _ in rows for k in x})
    idx = {k: i for i, k in enumerate(feats)}
    w, b, n = [0.0] * len(feats), 0.0, len(rows)
    for _ in range(steps):
        gw, gb = [0.0] * len(feats), 0.0
        for x, y in rows:
            z = b + sum(w[idx[k]] * v for k, v in x.items())
            p = 1.0 / (1.0 + math.exp(-max(-30.0, min(30.0, z))))
            for k, v in x.items():
                gw[idx[k]] += (p - y) * v
            gb += p - y
        for i in range(len(feats)):
            w[i] -= 0.5 * (gw[i] + l2 * w[i]) / n
        b -= 0.5 * gb / n
    return {k: w[idx[k]] for k in feats}, b


def log_loss(model, rows):
    w, b = model
    s = 0.0
    for x, y in rows:
        z = b + sum(w.get(k, 0.0) * v for k, v in x.items())
        p = min(max(1.0 / (1.0 + math.exp(-max(-30.0, min(30.0, z)))), 1e-6), 1 - 1e-6)
        s -= y * math.log(p) + (1 - y) * math.log(1 - p)
    return s / len(rows)


def main():
    leagues = [league(a) for a in sys.argv[1:]]
    if len(leagues) < 2:
        sys.exit("give two leagues or more: the check leaves each out in turn")
    coin = math.log(2)
    best = None
    for l2 in PULLS:
        held = sum(log_loss(fit([r for j, lg in enumerate(leagues) if j != i for r in lg], l2), lg) for i, lg in enumerate(leagues))
        held /= len(leagues)
        print(f"pull {l2:>4}: held-out log-loss {held:.4f} (coin flip {coin:.4f})")
        if held < coin - 0.002 and (best is None or held < best[1]):
            best = (l2, held)
    if best is None:
        print("No per-move model predicts a league it wasn't fitted on: the moves' effects don't carry")
        print("from league to league, so no per-move price would help. Nothing to suggest.")
        return
    w, _ = fit([r for lg in leagues for r in lg], best[0])
    print(f"// Per-move adjustments (pull {best[0]}): positive = priced too cheap by the leagues.")
    for k in sorted(w, key=lambda k: -w[k]):
        pts = max(-LIMIT, min(LIMIT, w[k] / POINT))
        if abs(pts) >= 1.0:
            print(f'    ("{k}", {pts:.1f}),')


if __name__ == "__main__":
    main()
