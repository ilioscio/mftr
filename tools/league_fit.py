# SPDX-License-Identifier: AGPL-3.0-or-later
"""Refit the balance price's weights from a random league (CR7, 20 §9.3, D71).

    mftr-tools league --champions 24 --matches 200 --seed 21 --log out/fit
    python tools/league_fit.py out/fit

Every league champion spends the same budget (`balance::fit_to`), so if the price were right,
which team wins wouldn't depend on how its points are split between durability, damage,
control and the rest. This fits a logistic regression of "blue won" on the two teams' summed
price parts (blue minus red), with a blue-side intercept: a part whose coefficient is above the
parts' mean wins more per point than the others (priced too cheap), one below wins less (too
dear). It prints each part's suggested weight multiplier, shrunk toward 1 by how sure the fit is
(a bootstrap), and clamped to ±25% a round: refit in small steps and run the league again.

Standard library only (no numpy), so it runs anywhere the tools do.
"""

import csv
import math
import random
import sys

PARTS = ["durability", "attack", "speed", "damage", "control", "mobility", "protection"]


def load(prefix):
    champs = {}
    with open(f"{prefix}.champions.csv", newline="") as f:
        for row in csv.DictReader(f):
            champs[int(row["champion"])] = [float(row[p]) for p in PARTS]
    matches = []
    with open(f"{prefix}.matches.csv", newline="") as f:
        for row in csv.DictReader(f):
            blue = [int(x) for x in row["blue"].split()]
            red = [int(x) for x in row["red"].split()]
            if not blue or not red:
                continue
            x = [sum(champs[c][j] for c in blue) - sum(champs[c][j] for c in red) for j in range(len(PARTS))]
            matches.append((x, 1.0 if row["winner"] == "blue" else 0.0))
    return champs, matches


def fit(matches, l2=0.02, steps=3000, rate=0.05):
    """Logistic regression by gradient descent on standardized features."""
    n, k = len(matches), len(PARTS)
    scale = [max(1e-6, math.sqrt(sum(x[j] ** 2 for x, _ in matches) / n)) for j in range(k)]
    w, b = [0.0] * k, 0.0
    for _ in range(steps):
        gw, gb = [0.0] * k, 0.0
        for x, y in matches:
            z = b + sum(w[j] * x[j] / scale[j] for j in range(k))
            p = 1.0 / (1.0 + math.exp(-max(-30.0, min(30.0, z))))
            for j in range(k):
                gw[j] += (p - y) * x[j] / scale[j]
            gb += p - y
        for j in range(k):
            w[j] -= rate * (gw[j] / n + l2 * w[j])
        b -= rate * gb / n
    # Back to "per point" coefficients.
    return [w[j] / scale[j] for j in range(k)], b


def main():
    prefix = sys.argv[1]
    champs, matches = load(prefix)
    print(f"{len(matches)} decided matches, {len(champs)} champions")
    w, b = fit(matches)
    rng = random.Random(1)
    boots = []
    for _ in range(30):
        sample = [matches[rng.randrange(len(matches))] for _ in matches]
        boots.append(fit(sample, steps=800)[0])
    mean_w = sum(w) / len(w)
    print(f"blue side: {b:+.2f} (log-odds)")
    print(f"{'part':<11} {'per point':>10} {'± (boot)':>9} {'suggest ×':>10}")
    for j, part in enumerate(PARTS):
        vals = [bw[j] for bw in boots]
        mu = sum(vals) / len(vals)
        sd = math.sqrt(sum((v - mu) ** 2 for v in vals) / len(vals))
        raw = w[j] / mean_w if mean_w > 0 else 1.0
        # Shrink toward 1 by the signal-to-noise of the coefficient.
        sure = abs(w[j]) / (abs(w[j]) + sd) if sd > 0 else 1.0
        m = 1.0 + (raw - 1.0) * sure * 0.5
        m = max(0.75, min(1.25, m))
        print(f"{part:<11} {w[j]:>10.4f} {sd:>9.4f} {m:>10.2f}")


if __name__ == "__main__":
    main()
