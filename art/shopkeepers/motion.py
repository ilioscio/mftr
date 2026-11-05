# SPDX-License-Identifier: AGPL-3.0-or-later
"""How the four shopkeepers move (19 §3, §4): their bones, idles, fidgets and reactions, as
functions of time. Shared by the animated previews (previews.py, the block-outs on a quick rig)
and the real packs (build.py, the `biped` bones), so what the owner approved in the previews is
what ships.

A pose is a turn, an offset and a scale per bone, in the armature's axes (X = the keeper's left,
Y = back, Z = up, facing -Y), relative to the parent. Arms are two-bone IK chains reaching for
`ik_<side>` (a bone of our own, not the armature's); bones on `springs` lag and settle behind
whatever carries them (hats, bows, coat tails, beards, packs, lanterns).
"""

import math

from mathutils import Euler, Matrix, Quaternion, Vector

import concepts as C

FPS = 30


# --- Timing helpers ------------------------------------------------------------------------

def ss(x):
    x = min(1.0, max(0.0, x))
    return x * x * (3 - 2 * x)


def _v(x):
    return Vector(x) if isinstance(x, (tuple, list, Vector)) else x


def keys(t, ks):
    """Eased interpolation through [(time, value)], holding the ends; values numbers or tuples."""
    if t <= ks[0][0]:
        return _v(ks[0][1])
    for (t0, v0), (t1, v1) in zip(ks, ks[1:]):
        if t <= t1:
            f = ss((t - t0) / (t1 - t0)) if t1 > t0 else 1.0
            a, b = _v(v0), _v(v1)
            return a + (b - a) * f
    return _v(ks[-1][1])


def sn(t, period, phase=0.0):
    return math.sin(2 * math.pi * (t / period + phase))


def E(x=0.0, y=0.0, z=0.0):
    """A turn in degrees about the armature axes (X = the keeper's left, Y = back, Z = up)."""
    return Euler((math.radians(x), math.radians(y), math.radians(z))).to_quaternion()


def path(u, pts):
    """A point along [(time, Vector)] (eased between, held at the ends)."""
    return keys(u, [(t, tuple(p)) for t, p in pts])


def env(u, a, b, ramp=0.2):
    """1 between `a` and `b`, easing in and out over `ramp` seconds."""
    return min(ss((u - a) / ramp), ss((b - u) / ramp))


# --- Pose and rig --------------------------------------------------------------------------

class Pose(dict):
    """bone -> [turn (Quaternion, armature axes, relative to the parent), offset, scale]."""

    def __init__(self, names):
        super().__init__()
        for n in names:
            self[n] = [Quaternion(), Vector(), Vector((1.0, 1.0, 1.0))]

    def rot(self, n, x=0.0, y=0.0, z=0.0, q=None):
        self[n][0] = (q if q is not None else E(x, y, z)) @ self[n][0]

    def mv(self, n, v):
        self[n][1] = self[n][1] + Vector(v)

    def sc(self, n, s):
        s = Vector(s) if isinstance(s, (tuple, list, Vector)) else Vector((s, s, s))
        self[n][2] = Vector([a * b for a, b in zip(self[n][2], s)])

    def size(self, n, s):
        self[n][2] = Vector((s, s, s))


class Rig:
    """Bones with heads in the keeper's rest space. `arms`: side -> (shoulder, elbow, hand,
    parent), two-bone chains whose target is the bone `ik_<side>`. `springs`: bone -> (Hz, damping
    ratio, +1 upright / -1 hanging, how much the parent's acceleration swings it, the largest
    swing in degrees). Bones under an arm (hands, fingers) are keyed but outside our FK."""

    def __init__(self, bones, arms, springs):
        self.head = {"world": Vector()}
        self.parent = {}
        self.names = []
        self.deco = set()
        for name, head, parent in bones:
            self.head[name] = Vector(head)
            self.parent[name] = parent
            self.names.append(name)
            if parent.startswith(("up_", "fore_", "hand_")) or parent in self.deco:
                self.deco.add(name)
        self.arms = arms
        self.springs = springs

    def fk(self, P):
        W = {"world": Matrix.Identity(4)}
        for n in self.names:
            if n in self.deco:
                continue
            p = self.parent[n]
            r, l, _ = P[n]
            W[n] = W[p] @ Matrix.Translation(self.head[n] - self.head[p] + l) @ r.to_matrix().to_4x4()
        return W

    def at(self, W, n, p):
        """Where the rest-space point `p`, carried by bone `n`, is now."""
        return W[n] @ (Vector(p) - self.head[n])

    def place(self, P, W, n, point, w=1.0):
        """Move bone `n`'s head toward `point` (posed space) by `w`."""
        p = self.parent[n]
        rel = self.head[n] - self.head[p]
        cur = W[p] @ (rel + P[n][1])
        new = cur.lerp(Vector(point), w)
        P[n][1] = W[p].inverted() @ new - rel


class Springs:
    def __init__(self, rig):
        self.rig = rig
        self.st = {n: None for n in rig.springs}

    def apply(self, P, dt):
        rig = self.rig
        for n in rig.names:
            if n not in rig.springs:
                continue
            hz, zeta, up, gain, most = rig.springs[n]
            W = rig.fk(P)
            Qp = W[rig.parent[n]].to_quaternion()
            pos = W[n].translation.copy()
            st = self.st[n]
            if st is None:
                st = self.st[n] = {"th": Vector(), "v": Vector(), "q": Qp.copy(), "p1": pos.copy(), "p2": pos.copy()}
            dq = Qp @ st["q"].inverted()
            if dq.w < 0:
                dq.negate()
            axis, ang = dq.to_axis_angle()
            inv = Qp.inverted()
            om = inv @ (Vector(axis) * ang)
            acc = inv @ ((pos - 2 * st["p1"] + st["p2"]) / (dt * dt))
            k = (2 * math.pi * hz) ** 2
            c = 2 * zeta * math.sqrt(k)
            st["th"] = st["th"] - om
            tau = -Vector((0, 0, up)).cross(acc) * gain
            st["v"] = st["v"] + (tau - k * st["th"] - c * st["v"]) * dt
            st["th"] = st["th"] + st["v"] * dt
            lim = math.radians(most)
            if st["th"].length > lim:
                st["th"] = st["th"].normalized() * lim
            if st["th"].length > 1e-7:
                P.rot(n, q=Quaternion(st["th"].normalized(), st["th"].length))
            st["q"], st["p2"], st["p1"] = Qp.copy(), st["p1"], pos.copy()


def limbs(arms, legs):
    """The bones every keeper has besides its own: a hand at the end of each arm, and each leg's
    shin and foot (static in the previews; the real rig plants them with IK)."""
    out = []
    for side, (sh, el, hd, par) in arms.items():
        out.append(("hand_" + side, hd, "fore_" + side))
    for side, (hip, knee, ankle, toe) in legs.items():
        out.append(("leg_" + side, knee, "root"))
        out.append(("foot_" + side, ankle, "root"))
    return out


# --- Goldwick: pompous, shrewd, delighted --------------------------------------------------

class Goldwick:
    key = "goldwick"
    MON = (0.042, -0.178, 0.993)
    WATCH = (0.17, -0.36, 0.585)
    COIN = (0.13, -0.26, 0.62)
    COINS = (0.0, -0.30, 0.80)
    arms = {"R": ((-0.30, 0, 0.78), (-0.43, -0.04, 0.61), (-0.43, -0.12, 0.48), "body"),
            "L": ((0.30, 0, 0.78), (0.37, -0.16, 0.66), (0.19, -0.32, 0.68), "body")}
    legs = {"L": ((0.12, 0, 0.30), (0.12, 0, 0.17), (0.12, 0, 0.06), (0.12, -0.16, 0.02)),
            "R": ((-0.12, 0, 0.30), (-0.12, 0, 0.17), (-0.12, 0, 0.06), (-0.12, -0.16, 0.02))}
    bones = [
        ("root", (0, 0, 0), "world"),
        ("body", (0, 0, 0.22), "root"),
        ("head", (0, -0.03, 0.88), "body"),
        ("hat", (0, 0, 1.08), "head"),
        ("monocle", MON, "head"),
        ("watch", WATCH, "body"),
        ("coin", COIN, "body"),
        ("coins", COINS, "world"),
        ("ik_R", (-0.43, -0.12, 0.48), "body"),
        ("ik_L", (0.19, -0.32, 0.68), "body"),
    ] + limbs(arms, legs)
    springs = {"hat": (2.6, 0.3, 1, 2.5, 25)}
    look = 0.78
    dist = 4.6
    loop = 6.4
    detail = 1.45

    @staticmethod
    def props(b):
        b.part("watch")
        C.disc(b, Goldwick.WATCH, 0.042, 0.016, C.GOLD, sides=12, rot=(90, 0, 0))
        C.disc(b, (0.17, -0.37, 0.585), 0.033, 0.004, C.CREAM, sides=12, rot=(90, 0, 0))
        b.part("coin")
        C.disc(b, Goldwick.COIN, 0.035, 0.012, C.GOLD, sides=10, rot=(90, 0, 0))
        b.part("coins")
        x, y, z = Goldwick.COINS
        for k in range(10):
            a = k * 2 * math.pi / 10
            C.disc(b, (x + 0.12 * math.cos(a), y + 0.12 * math.sin(a), z + 0.05 * math.sin(3 * a)), 0.035, 0.01, C.GOLD,
                   sides=8, rot=(30 * math.sin(a), 40 * math.cos(2 * a), 0))

    @staticmethod
    def idle(t, P, rig, q):
        rock = sn(t, 3.2)
        P.rot("body", x=3.0 * rock)
        P.mv("body", (0, 0, 0.012 * max(0.0, rock)))
        br = sn(t, 6.4 / 3)
        P.sc("body", (1 + 0.025 * br, 1 + 0.025 * br, 1))
        P.rot("head", x=-1.5 * sn(t, 3.2, -0.08) + 4 * sn(t, 6.4, 0.25) * (1 - q), z=10 * sn(t, 6.4) * (1 - q))
        P.mv("ik_L", (0, -0.008 * br, 0.006 * br))
        P.size("coins", 0.0)

    @staticmethod
    def monocle(u, P, rig):
        """Pops the monocle out, polishes it on his coat, screws it back in with a squint."""
        P.rot("head", x=keys(u, [(0.5, 0), (0.8, -6), (1.3, 12), (2.8, 12), (3.2, 0), (3.75, 0), (3.9, -5), (4.4, 4), (4.9, 0)]),
              y=keys(u, [(3.7, 0), (3.9, 9), (4.5, 9), (4.9, 0)]))
        P.rot("body", x=keys(u, [(1.3, 0), (1.6, 5), (2.8, 5), (3.1, 0)]))
        W = rig.fk(P)
        eye = rig.at(W, "head", Goldwick.MON)
        front = rig.at(W, "head", (0.06, -0.27, 0.99))
        coat = rig.at(W, "body", (0.10, -0.45, 0.66))
        if u < 1.4:
            m = path(u, [(0.6, eye), (0.9, front), (1.4, coat)])
        elif u < 2.8:
            a = (u - 1.4) * 2 * math.pi * 2.2
            e = min(1.0, (u - 1.4) / 0.2, (2.8 - u) / 0.2)
            m = coat + Vector((0.03 * math.cos(a), 0, 0.025 * math.sin(a))) * e
        else:
            m = path(u, [(2.8, coat), (3.35, front), (3.65, eye)])
        rig.place(P, W, "monocle", m)
        P.rot("monocle", y=keys(u, [(1.4, 0), (2.8, 720), (3.35, 720), (3.65, 1080)]))
        reach = keys(u, [(0, 0), (0.55, 1), (3.7, 1), (4.4, 0)])
        rig.place(P, W, "ik_L", m + Vector((0.035, -0.03, -0.02)), reach)

    @staticmethod
    def watch(u, P, rig):
        """Pulls his pocket watch, gives it a shake, holds it to his ear, a satisfied nod."""
        P.rot("head", y=keys(u, [(2.4, 0), (2.7, 16), (3.5, 16), (3.8, 0)]),
              x=keys(u, [(1.1, 0), (1.4, 10), (2.4, 10), (2.7, 0), (3.5, 0), (3.65, 9), (3.8, 0), (3.95, 9), (4.1, 0)]))
        P.rot("body", y=keys(u, [(2.4, 0), (2.7, 4), (3.5, 4), (3.8, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        w = path(u, [(0.5, at(Goldwick.WATCH)), (1.0, at((0.30, -0.50, 0.80))), (1.4, at((0.06, -0.48, 0.98))),
                     (2.4, at((0.06, -0.48, 0.98))), (2.75, at((0.25, -0.13, 1.02))), (3.5, at((0.25, -0.13, 1.02))),
                     (3.85, at((0.30, -0.50, 0.80))), (4.25, at(Goldwick.WATCH))])
        if 1.5 < u < 2.3:
            w = w + Vector((0.03 * math.sin(2 * math.pi * 6 * (u - 1.5)), 0, 0)) * min(1.0, (u - 1.5) / 0.1, (2.3 - u) / 0.1)
        rig.place(P, W, "watch", w)
        P.rot("watch", z=keys(u, [(0.6, 0), (1.3, 180), (2.4, 180), (2.75, 90), (3.5, 90), (4.1, 0)]))
        rig.place(P, W, "ik_L", w + Vector((0.01, 0.0, -0.05)), keys(u, [(0, 0), (0.5, 1), (4.2, 1), (4.8, 0)]))

    @staticmethod
    def coin(u, P, rig):
        """Bites a coin to test it, a pleased nod, a tap of the cane."""
        P.rot("head", x=keys(u, [(1.0, 0), (1.3, 6), (1.45, 10), (1.75, 16), (1.85, 9), (2.05, 16), (2.15, 9), (2.3, 0),
                                 (2.55, 0), (2.65, 9), (2.75, 0), (2.85, 9), (2.95, 0)]),
              y=keys(u, [(2.2, 0), (2.4, 10), (2.55, 10), (2.9, 0)]))
        P.rot("body", x=keys(u, [(2.45, 0), (2.7, -5), (3.1, -5), (3.4, 0)]))
        P.sc("body", 1 + keys(u, [(2.45, 0), (2.7, 0.04), (3.1, 0.04), (3.4, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        mouth = rig.at(W, "head", (0.0, -0.205, 0.93))
        c = path(u, [(0.45, at(Goldwick.COIN)), (0.6, at((0.16, -0.39, 0.64))), (1.1, at((0.10, -0.48, 0.96)))])
        if u > 1.1:
            c = path(u, [(1.35, at((0.10, -0.48, 0.96))), (1.65, mouth), (2.2, mouth), (2.5, at((0.09, -0.46, 0.98))),
                         (2.95, at((0.09, -0.46, 0.98))), (3.4, at((0.16, -0.39, 0.64))), (3.55, at(Goldwick.COIN))])
        rig.place(P, W, "coin", c)
        P.rot("coin", z=keys(u, [(1.1, 0), (1.35, 25), (2.2, 25), (2.5, 0)]))
        rig.place(P, W, "ik_L", c + Vector((0.035, 0.0, -0.04)), keys(u, [(0, 0), (0.45, 1), (3.55, 1), (4.2, 0)]))
        P.mv("ik_R", (0, 0, keys(u, [(2.5, 0), (2.6, 0.07), (2.7, 0), (2.85, 0.07), (2.95, 0)])))

    @staticmethod
    def greet(u, P, rig):
        """Tips his hat with a little bow, the cane flourishing."""
        P.rot("body", x=keys(u, [(0.4, 0), (0.9, 14), (1.4, 14), (1.9, 0)]))
        P.rot("head", x=keys(u, [(0.4, 0), (0.9, 8), (1.4, 8), (1.9, 0)]))
        P.mv("hat", keys(u, [(0.35, (0, 0, 0)), (0.65, (0, -0.04, 0.10)), (1.45, (0, -0.04, 0.10)), (1.75, (0, 0, 0))]))
        P.rot("hat", x=keys(u, [(0.35, 0), (0.65, -22), (1.45, -22), (1.75, 0)]))
        P.rot("hand_R", z=keys(u, [(0.5, 0), (1.3, 360)]))
        W = rig.fk(P)
        brim = rig.at(W, "hat", (0.12, -0.10, 1.10))
        rig.place(P, W, "ik_L", brim + Vector((0.03, -0.03, 0.0)), keys(u, [(0, 0), (0.35, 1), (1.75, 1), (2.3, 0)]))
        P.mv("ik_R", keys(u, [(0.4, (0, 0, 0)), (0.8, (-0.10, -0.12, 0.14)), (1.3, (-0.10, -0.12, 0.14)), (1.7, (0, 0, 0))]))

    @staticmethod
    def purchase(u, P, rig):
        """Rubs his gloved hands together, flips a coin and catches it on the back of his hand."""
        P.rot("head", x=keys(u, [(0, 0), (0.3, 6), (1.1, 6), (1.5, -10), (2.2, -10), (2.5, 0)]))
        P.mv("body", (0, 0, 0.012 * abs(math.sin(2 * math.pi * 2 * u)) * env(u, 0.1, 1.1)))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        rub = 0.03 * math.sin(2 * math.pi * 4 * u) * env(u, 0.3, 1.1, 0.1)
        rig.place(P, W, "ik_R", at((-0.04 + rub, -0.42, 0.72)), keys(u, [(0, 0), (0.3, 1), (1.1, 1), (1.4, 0)]))
        hand = at((0.10, -0.46, 0.84))
        if u < 1.2:
            c = at(Goldwick.COIN)
            goal = at((0.04 - rub, -0.42, 0.72))
        else:
            c = path(u, [(1.2, at(Goldwick.COIN)), (1.35, at((0.16, -0.39, 0.64))), (1.6, hand)])
            if 1.6 <= u < 2.15:
                s = (u - 1.6) / 0.55
                c = hand + Vector((0, 0, 4 * 0.32 * s * (1 - s)))
            elif u >= 2.15:
                c = path(u, [(2.3, hand), (2.6, at((0.16, -0.39, 0.64))), (2.75, at(Goldwick.COIN))])
            goal = c if not (1.6 < u < 2.15) else hand
        dip = keys(u, [(1.5, 0), (1.6, -0.04), (1.7, 0.02), (1.8, 0), (2.1, 0), (2.2, -0.04), (2.35, 0)])
        rig.place(P, W, "coin", c)
        P.rot("coin", x=keys(u, [(1.6, 0), (2.15, 1080)]))
        rig.place(P, W, "ik_L", goal + Vector((0.02, 0, dip - 0.03)), keys(u, [(0, 0), (0.3, 1), (2.75, 1), (3.2, 0)]))

    @staticmethod
    def farewell(u, P, rig):
        """A two-finger salute off the hat brim."""
        P.rot("head", y=keys(u, [(0.4, 0), (0.7, 8), (1.3, 8), (1.7, 0)]))
        W = rig.fk(P)
        brim = rig.at(W, "hat", (0.10, -0.20, 1.12))
        out = rig.at(W, "body", (0.34, -0.42, 1.18))
        goal = path(u, [(0.4, brim), (0.55, brim), (0.8, out), (1.2, out)])
        rig.place(P, W, "ik_L", goal, keys(u, [(0, 0), (0.4, 1), (1.2, 1), (1.8, 0)]))
        P.rot("hand_L", z=keys(u, [(0.55, 0), (0.8, -25), (1.2, -25), (1.6, 0)]))

    @staticmethod
    def victory(u, P, rig):
        """Throws a handful of coins in the air; they rain down around him."""
        hop = sum(0.06 * max(0.0, math.sin(math.pi * (u - t0) / 0.35)) for t0 in (0.7, 1.3, 1.9) if t0 <= u <= t0 + 0.35)
        P.mv("body", (0, 0, hop))
        P.rot("head", x=keys(u, [(0.5, 0), (0.8, -14), (2.4, -14), (2.8, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        wave = 0.04 * math.sin(2 * math.pi * 2 * u) * env(u, 0.9, 2.4)
        for side, sgn in (("L", 1), ("R", -1)):
            low, high = at((sgn * 0.12, -0.42, 0.66)), at((sgn * 0.26, -0.36, 1.32 + sgn * wave))
            rig.place(P, W, "ik_" + side, path(u, [(0.3, low), (0.45, low), (0.7, high), (2.4, high), (2.9, low)]),
                      keys(u, [(0, 0), (0.3, 1), (2.9, 1), (3.3, 0)]))
        x, y, z = Goldwick.COINS
        if 0.6 < u < 2.5:
            s = (u - 0.6) / 1.9
            h = keys(u, [(0.6, 0.0), (1.15, 1.0), (2.35, -0.70)])
            rig.place(P, W, "coins", (x, y, z + h))
            spread = (0.6 + 1.8 * s) * (1.0 if u < 2.35 else max(0.0, (2.5 - u) / 0.15))
            P["coins"][2] = Vector((spread, spread, spread))
            P.rot("coins", z=200 * s)

    @staticmethod
    def defeat(u, P, rig):
        """Hugs himself, trembling, his hat sliding down over his eyes."""
        tremble = 1.6 * math.sin(2 * math.pi * 9 * u) * ss((u - 0.9) / 0.3)
        P.rot("body", x=keys(u, [(0, 0), (0.7, 10)]), y=tremble)
        P.mv("body", (0, 0, keys(u, [(0, 0), (0.7, -0.04)])))
        P.rot("head", x=keys(u, [(0.2, 0), (0.9, 20)]))
        P.mv("hat", keys(u, [(0.6, (0, 0, 0)), (1.2, (0, -0.02, -0.10))]))
        P.rot("hat", x=keys(u, [(0.6, 0), (1.2, 12)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        rig.place(P, W, "ik_L", at((-0.14, -0.40, 0.64)), keys(u, [(0, 0), (0.6, 1)]))
        rig.place(P, W, "ik_R", at((0.14, -0.41, 0.58)), keys(u, [(0.1, 0), (0.7, 1)]))

    fidgets = [("Fidget 1: polishes his monocle", 5.0, monocle),
               ("Fidget 2: checks his pocket watch", 4.9, watch),
               ("Fidget 3: bites a coin to test it", 4.3, coin)]
    reactions = [("greet", 2.4, greet), ("purchase", 3.3, purchase), ("farewell", 2.0, farewell),
                 ("victory", 3.4, victory), ("defeat", 2.6, defeat)]


# --- Madame Rosette: flirtatious, theatrical, vain -----------------------------------------

def _heart(rig, P, W, u, start, at):
    """The kiss she blows: a little heart floating off from `at`, born at `start`."""
    if u > start - 0.05:
        s = (u - start) / 1.6
        h = Vector(at) + Vector((0.25 * s + 0.05 * math.sin(2 * math.pi * 2 * s), -0.5 * s, 0.45 * s))
        rig.place(P, W, "heart", h)
        P.size("heart", keys(u, [(start - 0.05, 0), (start + 0.15, 1.15), (start + 0.25, 1.0), (start + 1.2, 1.0), (start + 1.6, 0)]))
        P.rot("heart", z=15 * math.sin(2 * math.pi * 1.5 * s))


class Rosette:
    key = "rosette"
    MIRROR = (0.20, -0.22, 0.60)
    PERFUME = (0.27, -0.20, 0.56)
    SHAFT = (Vector((-0.36, 0.24, 1.60)) - Vector((-0.17, -0.18, 0.78))).normalized()
    MIST = Vector((0.06, -0.46, 1.26))
    arms = {"R": ((-0.22, 0, 0.86), (-0.28, -0.08, 0.74), (-0.17, -0.17, 0.80), "body"),
            "L": ((0.22, 0, 0.86), (0.31, 0.02, 0.74), (0.17, -0.06, 0.72), "body")}
    legs = {"L": ((0.08, 0, 0.50), (0.08, 0, 0.28), (0.08, 0, 0.06), (0.08, -0.14, 0.02)),
            "R": ((-0.08, 0, 0.50), (-0.08, 0, 0.28), (-0.08, 0, 0.06), (-0.08, -0.14, 0.02))}
    bones = [
        ("root", (0, 0, 0), "world"),
        ("body", (0, 0, 0.55), "root"),
        ("skirt", (0, 0, 0.55), "body"),
        ("head", (0, 0, 0.96), "body"),
        ("bow", (0, 0.03, 1.26), "head"),
        ("parasol", (-0.17, -0.18, 0.78), "body"),
        ("ik_R", (-0.17, -0.17, 0.80), "parasol"),
        ("ik_L", (0.17, -0.06, 0.72), "body"),
        ("mirror", MIRROR, "body"),
        ("perfume", PERFUME, "body"),
        ("heart", (0, 0, 0.75), "world"),
        ("mist", (0, 0, 0.80), "world"),
    ] + limbs(arms, legs) + [("fingers_L", (0.18, -0.08, 0.70), "hand_L")]
    springs = {"skirt": (1.6, 0.35, -1, 1.2, 35), "bow": (2.2, 0.18, 1, 3.0, 30)}
    look = 0.78
    dist = 5.0
    loop = 6.4
    detail = 1.15

    @staticmethod
    def props(b):
        b.part("perfume")
        b.push((0.27, -0.20, 0.51), (0, 0, 0), 0.55)
        C.bottle(b, (0, 0, 0), 1, C.emi((0.95, 0.25, 0.55)))
        b.pop()
        b.part("heart")
        for s in (-1, 1):
            C.sphere(b, (s * 0.035, 0, 0.775), 0.045, (1.0, 0.22, 0.50), sides=8, rings=5, s=(1, 0.45, 1))
        C.lathe(b, [(0.70, 0.0), (0.775, 0.075)], 8, (1.0, 0.22, 0.50), cap=False, sy=0.45)
        b.part("mist")
        for i, (x, y, z, r) in enumerate(((0, 0, 0.80, 0.09), (0.09, 0.02, 0.83, 0.07), (-0.08, -0.03, 0.78, 0.075),
                                          (0.03, -0.06, 0.88, 0.06), (-0.04, 0.05, 0.86, 0.065), (0.07, -0.04, 0.74, 0.055),
                                          (-0.11, 0.03, 0.85, 0.05))):
            C.sphere(b, (x, y, z), r, (0.98, 0.62, 0.78) if i % 2 else (0.92, 0.52, 0.80), sides=8, rings=5)

    @staticmethod
    def idle(t, P, rig, q):
        sway = sn(t, 3.2)
        P.rot("body", y=3.5 * sway, x=-2 + 1.0 * sn(t, 1.6))
        P.mv("body", (0.02 * sway, 0, 0.006 * sn(t, 6.4 / 3)))
        P.rot("head", y=-2.5 * sway + 5 * sn(t, 6.4, 0.6) * (1 - q), z=12 * sn(t, 6.4) * (1 - q), x=-4)
        P.rot("parasol", q=Quaternion(Rosette.SHAFT, math.radians(360 / 6.4 * t)))
        P.rot("parasol", x=2 * sn(t, 3.2, 0.25))
        for n in ("heart", "mist"):
            P.size(n, 0.0)

    @staticmethod
    def mirror(u, P, rig):
        """Admires herself in her hand mirror, then blows a kiss (a little heart floats off)."""
        P.rot("head", y=keys(u, [(1.1, 0), (1.4, 12), (1.9, 12), (2.2, -10), (2.5, -10), (2.8, 0)]),
              x=keys(u, [(1.0, 0), (1.3, 6), (2.5, 6), (2.8, 0), (3.2, 0), (3.4, 6), (3.6, -6), (4.3, 0)]))
        P.rot("body", x=keys(u, [(3.4, 0), (3.7, -5), (4.2, -5), (4.6, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        m = path(u, [(0.5, at(Rosette.MIRROR)), (1.1, at((0.05, -0.40, 1.06))), (2.5, at((0.05, -0.40, 1.06))),
                     (3.0, at(Rosette.MIRROR))])
        rig.place(P, W, "mirror", m)
        P.rot("mirror", z=keys(u, [(0.5, 0), (1.1, 170), (2.5, 170), (3.0, 0)]))
        lips = rig.at(W, "head", (0.02, -0.22, 1.05))
        fling = at((0.30, -0.62, 1.18))
        if u < 3.0:
            goal, reach = m + Vector((0.0, 0.0, -0.05)), keys(u, [(0, 0), (0.45, 1)])
        else:
            goal = path(u, [(3.0, m + Vector((0.0, 0.0, -0.05))), (3.35, lips), (3.55, lips), (3.8, fling)])
            reach = keys(u, [(4.1, 1), (4.7, 0)])
        rig.place(P, W, "ik_L", goal, reach)
        _heart(rig, P, W, u, 3.7, (0.30, -0.62, 1.18))

    @staticmethod
    def fan(u, P, rig):
        """Fans herself with a gloved hand, then a long dramatic sigh."""
        P.rot("head", x=keys(u, [(0.3, 0), (0.6, -10), (2.6, -10), (2.9, -14), (3.3, 8), (3.9, 0)]),
              y=keys(u, [(0.3, 0), (0.6, 8), (2.6, 8), (3.0, 0)]))
        P.rot("body", x=keys(u, [(0.3, 0), (0.6, -5), (2.6, -5), (2.95, -8), (3.3, 5), (3.9, 0)]))
        P.mv("body", (0, 0, keys(u, [(2.5, 0), (2.9, 0.035), (3.35, -0.015), (3.9, 0)])))
        P.rot("parasol", x=keys(u, [(2.9, 0), (3.3, -10), (3.9, 0)]))
        W = rig.fk(P)
        e = min(1.0, max(0.0, (u - 0.4) / 0.2), max(0.0, (2.6 - u) / 0.2))
        a = 2 * math.pi * 3.5 * u
        goal = rig.at(W, "body", (0.16, -0.24, 1.0)) + Vector((0.04 * math.sin(a), -0.02 * math.cos(a), 0.01 * math.sin(2 * a))) * e
        rig.place(P, W, "ik_L", goal, keys(u, [(0, 0), (0.4, 1), (2.7, 1), (3.4, 0)]))

    @staticmethod
    def spritz(u, P, rig):
        """Spritzes perfume into the air and twirls through it."""
        P.rot("root", z=360 * ss((u - 2.0) / 1.6))
        P.mv("body", (keys(u, [(1.8, 0), (2.4, 0.05), (3.6, 0.05), (4.6, 0)]), keys(u, [(1.8, 0), (2.4, -0.42), (3.6, -0.42), (4.6, 0)]), 0))
        P.rot("head", x=keys(u, [(0.6, 0), (1.0, -8), (1.6, -8), (2.0, -14), (3.8, -14), (4.3, 0)]))
        P.rot("body", x=keys(u, [(1.9, 0), (2.2, -4), (3.6, -4), (4.0, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        spray = at((0.20, -0.44, 1.28))
        p = path(u, [(0.45, at(Rosette.PERFUME)), (1.0, spray), (1.5, spray), (1.9, at(Rosette.PERFUME))])
        if 1.0 < u < 1.5:
            p = p + Vector((0, 0, -0.012 * abs(math.sin(2 * math.pi * 4 * (u - 1.0)))))
        rig.place(P, W, "perfume", p)
        P.rot("perfume", x=keys(u, [(0.5, 0), (1.0, 25), (1.5, 25), (1.9, 0)]))
        rig.place(P, W, "ik_L", p + Vector((0.0, 0.0, -0.035)), keys(u, [(0, 0), (0.45, 1), (1.9, 1), (2.3, 0)]))
        if u > 1.0:
            rig.place(P, W, "mist", Rosette.MIST + Vector((0.0, 0.0, -0.12 * ss((u - 1.5) / 2.5))))
            P.size("mist", keys(u, [(1.05, 0), (1.3, 0.8), (1.5, 1.0), (2.8, 1.3), (3.7, 0)]))
            P.rot("mist", z=20 * (u - 1.0))

    @staticmethod
    def greet(u, P, rig):
        """A deep curtsy, the gown pooling."""
        P.mv("body", (0, 0, keys(u, [(0.2, 0), (0.9, -0.12), (1.5, -0.12), (2.2, 0)])))
        P.rot("body", x=keys(u, [(0.2, 0), (0.9, 10), (1.5, 10), (2.2, 0)]))
        P.rot("head", x=keys(u, [(0.3, 0), (0.9, 14), (1.5, 14), (2.1, 0)]), y=keys(u, [(0.9, 0), (1.2, 8), (1.6, 0)]))
        P.sc("skirt", 1 + keys(u, [(0.3, 0), (0.9, 0.12), (1.5, 0.12), (2.2, 0)]))
        P.rot("parasol", x=keys(u, [(0.3, 0), (0.9, 12), (1.5, 12), (2.2, 0)]))
        W = rig.fk(P)
        rig.place(P, W, "ik_L", rig.at(W, "body", (0.34, -0.22, 0.46)), keys(u, [(0, 0), (0.5, 1), (1.6, 1), (2.3, 0)]))

    @staticmethod
    def purchase(u, P, rig):
        """Claps her hands under her chin, then blows the buyer a kiss (a little heart pops)."""
        P.rot("head", y=keys(u, [(0.2, 0), (0.5, 10), (1.4, 10), (1.7, 0)]), x=keys(u, [(1.6, 0), (1.8, 6), (2.0, -6), (2.6, 0)]))
        P.mv("body", (0, 0, 0.01 * abs(math.sin(2 * math.pi * 3 * u)) * env(u, 0.4, 1.4)))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        gap = 0.025 + 0.035 * (0.5 + 0.5 * math.cos(2 * math.pi * 3 * (u - 0.4))) if 0.4 < u < 1.4 else 0.06
        rig.place(P, W, "ik_R", at((-gap, -0.27, 0.98)), keys(u, [(0, 0), (0.4, 1), (1.4, 1), (1.9, 0)]))
        lips = rig.at(W, "head", (0.02, -0.22, 1.05))
        fling = at((0.30, -0.62, 1.18))
        goal = path(u, [(0.4, at((gap, -0.27, 0.98))), (1.4, at((gap, -0.27, 0.98))), (1.65, lips), (1.8, lips), (2.05, fling)]) \
            if u >= 1.4 else at((gap, -0.27, 0.98))
        rig.place(P, W, "ik_L", goal, keys(u, [(0, 0), (0.4, 1), (2.3, 1), (2.9, 0)]))
        _heart(rig, P, W, u, 1.95, (0.30, -0.62, 1.18))

    @staticmethod
    def farewell(u, P, rig):
        """A wave with the fingers only: toodle-oo."""
        P.rot("head", y=keys(u, [(0.2, 0), (0.5, -8), (1.6, -8), (2.0, 0)]))
        P.rot("fingers_L", x=30 * math.sin(2 * math.pi * 5 * u) * env(u, 0.45, 1.6, 0.1))
        W = rig.fk(P)
        rig.place(P, W, "ik_L", rig.at(W, "body", (0.26, -0.30, 1.08)), keys(u, [(0, 0), (0.4, 1), (1.6, 1), (2.1, 0)]))

    @staticmethod
    def victory(u, P, rig):
        """Twirls with the parasol open overhead."""
        up = keys(u, [(0.2, 0), (0.7, 1), (2.6, 1), (3.2, 0)])
        tilt = Rosette.SHAFT.rotation_difference(Vector((0.05, 0.05, 1)).normalized())
        P.rot("parasol", q=Quaternion().slerp(tilt, up))
        P.mv("parasol", Vector((0.05, 0.08, 0.45)) * up)
        P.rot("root", z=360 * ss((u - 0.8) / 1.6))
        P.rot("head", x=-10 * up)
        P.mv("body", (0, 0, 0.02 * abs(math.sin(2 * math.pi * 1.5 * u)) * env(u, 0.8, 2.4)))
        W = rig.fk(P)
        rig.place(P, W, "ik_L", rig.at(W, "body", (0.30, -0.10, 1.00)), keys(u, [(0.3, 0), (0.8, 1), (2.5, 1), (3.1, 0)]))

    @staticmethod
    def defeat(u, P, rig):
        """Swoons backward, the back of her hand to her forehead; the parasol droops."""
        P.rot("body", x=keys(u, [(0.2, 0), (1.0, -22)]), y=keys(u, [(0.2, 0), (1.0, 6)]))
        P.mv("body", (0, 0, keys(u, [(0.2, 0), (1.0, -0.10)])))
        P.rot("head", x=keys(u, [(0.3, 0), (1.0, -16)]), y=keys(u, [(0.3, 0), (1.0, 12)]))
        P.rot("skirt", x=keys(u, [(0.2, 0), (1.0, 18)]))
        P.rot("parasol", y=keys(u, [(0.5, 0), (1.4, -55)]), x=keys(u, [(0.5, 0), (1.4, -15)]))
        P.mv("parasol", keys(u, [(0.5, (0, 0, 0)), (1.4, (-0.04, 0.02, -0.20))]))
        W = rig.fk(P)
        brow = rig.at(W, "head", (0.07, -0.20, 1.17))
        rig.place(P, W, "ik_L", brow + Vector((0.02, -0.03, 0.0)), keys(u, [(0.1, 0), (0.8, 1)]))

    fidgets = [("Fidget 1: admires herself, blows a kiss", 5.6, mirror),
               ("Fidget 2: fans herself, a dramatic sigh", 4.2, fan),
               ("Fidget 3: spritzes perfume and twirls through it", 4.9, spritz)]
    reactions = [("greet", 2.5, greet), ("purchase", 3.4, purchase), ("farewell", 2.2, farewell),
                 ("victory", 3.4, victory), ("defeat", 2.2, defeat)]


# --- Slink: shifty, conspiratorial, opportunistic -------------------------------------------

class Slink:
    key = "slink"
    DAGGER = (0.05, -0.10, 0.86)
    KNOT = (0.14, 0.10, 1.25)
    arms = {"R": ((-0.22, -0.28, 1.24), (-0.36, -0.33, 0.98), (-0.20, -0.52, 0.93), "body"),
            "L": ((0.22, -0.28, 1.24), (0.42, -0.36, 1.08), (0.33, -0.60, 1.26), "body")}
    legs = {"L": ((0.11, 0, 0.55), (0.11, -0.01, 0.30), (0.11, 0, 0.07), (0.11, -0.19, 0.02)),
            "R": ((-0.11, 0, 0.55), (-0.11, -0.01, 0.30), (-0.11, 0, 0.07), (-0.11, -0.19, 0.02))}
    bones = [
        ("root", (0, 0, 0), "world"),
        ("body", (0, -0.02, 0.50), "root"),
        ("hem", (0, -0.03, 0.55), "body"),
        ("flap", (0.19, -0.12, 0.90), "body"),
        ("sack", KNOT, "body"),
        ("loot", (0.14, 0.10, 1.30), "sack"),
        ("head", (0, -0.36, 1.30), "body"),
        ("brim", (0, -0.40, 1.50), "head"),
        ("scarf", (0.09, -0.46, 1.30), "head"),
        ("dagger", DAGGER, "body"),
        ("ik_R", (-0.20, -0.52, 0.93), "body"),
        ("ik_L", (0.33, -0.60, 1.26), "body"),
    ] + limbs(arms, legs) + [("fingers_L", (0.33, -0.62, 1.28), "hand_L")]
    springs = {"hem": (1.5, 0.35, -1, 1.2, 25), "sack": (1.3, 0.3, -1, 3.0, 35),
               "brim": (2.6, 0.3, 1, 2.0, 20), "scarf": (1.7, 0.25, -1, 4.0, 40)}
    look = 0.95
    dist = 5.0
    loop = 5.6
    detail = 1.75

    @staticmethod
    def props(b):
        b.part("dagger")
        x, y, z = Slink.DAGGER
        C.box(b, (x, y, z + 0.19), (0.05, 0.012, 0.27), C.met((0.72, 0.74, 0.78)), taper=0.25)
        C.box(b, (x, y, z + 0.05), (0.13, 0.03, 0.025), C.GOLD_D)
        C.cyl(b, (x, y), z - 0.04, z + 0.04, 0.014, (0.10, 0.05, 0.03), sides=6)
        C.sphere(b, (x, y, z - 0.05), 0.02, C.LINING, sides=6, rings=4)
        # A rag in his team's color knotted round his left arm (the team mark, 19 §2).
        sh, el = Vector(Slink.arms["L"][0]), Vector(Slink.arms["L"][1])
        d = (el - sh).normalized()
        rot = tuple(math.degrees(x) for x in d.to_track_quat("Z", "Y").to_euler())
        b.part("up_L")
        C.lathe(b, [(-0.035, 0.078), (0.035, 0.078)], 10, C.team(), at=tuple(sh.lerp(el, 0.55)), rot=rot)
        C.tube(b, [tuple(sh.lerp(el, 0.55) + Vector((0.0, -0.07, 0.0))), tuple(sh.lerp(el, 0.55) + Vector((0.03, -0.14, -0.08)))],
               [0.02, 0.008], 4, C.team(0.85))
        b.part("loot")
        C.sphere(b, (0.14, 0.10, 1.32), 0.10, C.emi((0.55, 1.0, 0.10)), sides=10, rings=6)
        for dx, dz in ((-0.07, 0.07), (0.08, 0.05), (0.0, 0.12)):
            C.sphere(b, (0.14 + dx, 0.08, 1.32 + dz), 0.03, C.emi((0.9, 1.0, 0.5)), sides=6, rings=4)

    @staticmethod
    def idle(t, P, rig, q):
        rock = sn(t, 2.8)
        P.rot("body", x=2.0 * rock, y=1.5 * sn(t, 5.6))
        P.mv("body", (0, 0, 0.006 * sn(t, 1.4)))
        c = t % 5.6
        hz = keys(c, [(0, 0), (1.6, 0), (1.72, 38), (2.4, 38), (2.52, -38), (3.2, -38), (3.4, 0)])
        P.rot("head", z=hz * (1 - q), x=4 * sn(t, 2.8, -0.1))
        cb = t % 2.8
        curl = keys(cb, [(0, 0), (0.15, 75), (0.4, 0), (0.55, 75), (0.8, 0)]) * (1 - q)
        P.rot("fingers_L", x=-curl)
        P.mv("ik_L", (0, 0.02 * curl / 75, 0))
        P.size("loot", 0.0)

    @staticmethod
    def coat(u, P, rig):
        """Opens one side of his coat to show the vials, a look around, snaps it shut."""
        P.rot("flap", z=keys(u, [(0.4, 0), (1.0, 75), (2.9, 75), (3.1, 0)]))
        P.rot("body", z=keys(u, [(0.6, 0), (1.1, -12), (2.6, -12), (3.0, 0)]))
        P.rot("head", x=keys(u, [(1.0, 0), (1.3, 18), (2.0, 18), (2.3, 0)]),
              z=keys(u, [(2.3, 0), (2.4, 40), (2.6, 40), (2.7, -40), (2.85, -40), (2.95, 0)]))
        W = rig.fk(P)
        edge = rig.at(W, "flap", (0.045, -0.27, 1.0)) + Vector((0.02, -0.04, 0))
        rig.place(P, W, "ik_L", edge, keys(u, [(0, 0), (0.4, 1), (3.1, 1), (3.7, 0)]))

    @staticmethod
    def dagger(u, P, rig):
        """Flips a dagger, catches it, cleans a nail with it."""
        P.rot("head", x=keys(u, [(0.8, 0), (1.0, -10), (1.6, -6), (2.2, 16), (3.7, 16), (4.0, 0)]),
              z=keys(u, [(1.8, 0), (2.2, -8), (3.7, -8), (4.0, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        show = at((0.26, -0.52, 1.06))
        nail_hand = at((-0.06, -0.58, 1.06))
        nail = nail_hand + Vector((0.25, 0.0, -0.02))
        if 1.8 < u < 3.8:
            nail = nail + Vector((0.015 * math.sin(2 * math.pi * 3 * (u - 2.2)), 0, 0)) * min(1.0, max(0.0, (u - 2.2) / 0.2), max(0.0, (3.6 - u) / 0.2))
        if u < 0.9:
            d = path(u, [(0.4, at(Slink.DAGGER)), (0.55, at((0.10, -0.32, 0.92))), (0.9, show)])
        elif u < 1.6:
            s = (u - 0.9) / 0.7
            d = show + Vector((0, 0, 4 * 0.38 * s * (1 - s)))
        else:
            d = path(u, [(1.8, show), (2.2, nail), (3.7, nail), (4.0, show), (4.4, at((0.10, -0.32, 0.92))), (4.55, at(Slink.DAGGER))])
        rig.place(P, W, "dagger", d)
        P.rot("dagger", x=keys(u, [(0.9, 0), (1.6, 720)]))
        P.rot("dagger", y=keys(u, [(1.8, 0), (2.2, -90), (3.7, -90), (4.0, 0)]))
        hand = d if not (0.95 < u < 1.55) else show
        dip = keys(u, [(0.8, 0), (0.9, -0.04), (1.0, 0.03), (1.1, 0), (1.55, 0), (1.65, -0.05), (1.8, 0)])
        rig.place(P, W, "ik_L", hand + Vector((0, 0, dip - 0.01)), keys(u, [(0, 0), (0.4, 1), (4.55, 1), (5.1, 0)]))
        rig.place(P, W, "ik_R", nail_hand, keys(u, [(1.6, 0), (2.1, 1), (3.7, 1), (4.2, 0)]))

    @staticmethod
    def sack(u, P, rig):
        """Swings the stolen sack off his back, peeks in (something glows), nods, slings it back."""
        P.rot("head", x=keys(u, [(1.4, 0), (1.8, 28), (3.5, 28), (3.6, 20), (3.7, 30), (3.8, 20), (3.9, 30), (4.0, 0)]),
              z=keys(u, [(4.0, 0), (4.1, 35), (4.3, 35), (4.4, -35), (4.6, -35), (4.7, 0)]))
        P.rot("body", x=keys(u, [(1.4, 0), (1.8, 10), (3.6, 10), (4.0, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        over, front = at((0.20, -0.12, 1.55)), at((0.02, -0.58, 1.12))
        k = path(u, [(0.5, at(Slink.KNOT)), (0.9, over), (1.3, front), (4.3, front), (4.7, over), (5.1, at(Slink.KNOT))])
        rig.place(P, W, "sack", k)
        P.rot("sack", x=keys(u, [(0.5, 0), (0.9, -30), (1.3, 0), (4.3, 0), (4.7, -30), (5.1, 0)]))
        rig.place(P, W, "ik_L", k + Vector((0.03, 0.0, 0.02)), keys(u, [(0, 0), (0.5, 1), (5.1, 1), (5.7, 0)]))
        W = rig.fk(P)
        rig.place(P, W, "ik_R", rig.at(W, "sack", (-0.06, 0.0, 1.05)), keys(u, [(1.0, 0), (1.4, 1), (4.2, 1), (4.6, 0)]))
        P.size("loot", keys(u, [(1.6, 0), (1.9, 1.0), (3.4, 1.0), (3.6, 0)]) * (1 + 0.15 * math.sin(2 * math.pi * 5 * u)))

    @staticmethod
    def greet(u, P, rig):
        """Leans way in, a beckoning finger: psst."""
        P.rot("body", x=keys(u, [(0.1, 0), (0.6, 18), (1.6, 18), (2.1, 0)]))
        P.rot("head", x=keys(u, [(0.2, 0), (0.6, -12), (1.6, -12), (2.0, 0)]), y=keys(u, [(0.6, 0), (0.9, 10), (1.6, 10), (2.0, 0)]))
        P.rot("fingers_L", x=-keys(u, [(0.7, 0), (0.85, 80), (1.05, 0), (1.2, 80), (1.4, 0)]))
        W = rig.fk(P)
        rig.place(P, W, "ik_L", rig.at(W, "body", (0.22, -0.78, 1.18)), keys(u, [(0, 0), (0.6, 1), (1.6, 1), (2.2, 0)]))

    @staticmethod
    def purchase(u, P, rig):
        """A quick look both ways, the item slid over, the gold vanishing into his coat."""
        P.rot("head", z=keys(u, [(0, 0), (0.12, 40), (0.35, 40), (0.47, -40), (0.7, -40), (0.82, 0)]))
        P.rot("body", x=keys(u, [(0.7, 0), (1.0, 12), (1.5, 12), (1.9, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        rig.place(P, W, "ik_R", at((-0.08, -0.82, 0.98)), keys(u, [(0.7, 0), (1.1, 1), (1.3, 1), (1.7, 0)]))
        goal = path(u, [(1.2, at((0.22, -0.74, 1.02))), (1.5, at((0.22, -0.74, 1.02))), (1.9, at((0.06, -0.32, 0.95)))])
        rig.place(P, W, "ik_L", goal, keys(u, [(1.0, 0), (1.25, 1), (2.0, 1), (2.6, 0)]))
        P.rot("fingers_L", x=-keys(u, [(1.4, 0), (1.5, 70), (2.0, 70), (2.3, 0)]))

    @staticmethod
    def farewell(u, P, rig):
        """A finger to where his lips would be: shh."""
        P.rot("head", y=keys(u, [(0.3, 0), (0.6, 10), (1.6, 10), (2.0, 0)]), x=keys(u, [(0.3, 0), (0.6, 6), (1.6, 6), (2.0, 0)]))
        W = rig.fk(P)
        lips = rig.at(W, "head", (0.03, -0.58, 1.31))
        rig.place(P, W, "ik_L", lips + Vector((0.0, -0.02, -0.07)), keys(u, [(0, 0), (0.5, 1), (1.6, 1), (2.2, 0)]))

    @staticmethod
    def victory(u, P, rig):
        """A silent, shoulder-shaking laugh."""
        e = env(u, 0.3, 2.5, 0.3)
        P.rot("body", x=-6 * e + 3.0 * math.sin(2 * math.pi * 7 * u) * e)
        P.mv("body", (0, 0, 0.012 * abs(math.sin(2 * math.pi * 7 * u)) * e))
        P.rot("head", x=-14 * e)
        W = rig.fk(P)
        rig.place(P, W, "ik_L", rig.at(W, "body", (0.10, -0.32, 0.88)), keys(u, [(0.1, 0), (0.5, 1), (2.4, 1), (2.9, 0)]))

    @staticmethod
    def defeat(u, P, rig):
        """Crouches low, arms over his head, trembling."""
        tremble = 2.0 * math.sin(2 * math.pi * 8 * u) * ss((u - 0.8) / 0.3)
        P.mv("body", (0, 0, keys(u, [(0, 0), (0.7, -0.24)])))
        P.rot("body", x=keys(u, [(0, 0), (0.7, 24)]), y=tremble)
        P.rot("head", x=keys(u, [(0.1, 0), (0.7, 22)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "head", p)
        rig.place(P, W, "ik_L", at((0.10, -0.38, 1.50)), keys(u, [(0, 0), (0.6, 1)]))
        rig.place(P, W, "ik_R", at((-0.10, -0.38, 1.50)), keys(u, [(0.05, 0), (0.65, 1)]))

    fidgets = [("Fidget 1: flashes the vials in his coat", 3.8, coat),
               ("Fidget 2: flips a dagger, cleans a nail", 5.2, dagger),
               ("Fidget 3: peeks into the stolen sack", 5.8, sack)]
    reactions = [("greet", 2.3, greet), ("purchase", 2.7, purchase), ("farewell", 2.3, farewell),
                 ("victory", 3.0, victory), ("defeat", 1.8, defeat)]


# --- Old Folio: doddering, kindly, absent-minded ---------------------------------------------

class Folio:
    key = "folio"
    SPECS = (0, -0.185, 0.626)
    SCROLL = (0.0, -0.02, 0.46)
    arms = {"R": ((-0.14, -0.03, 0.48), (-0.22, -0.08, 0.38), (-0.22, -0.16, 0.44), "body"),
            "L": ((0.14, -0.03, 0.48), (0.19, -0.11, 0.36), (0.06, -0.22, 0.38), "body")}
    legs = {"L": ((0.07, 0, 0.14), (0.07, -0.01, 0.09), (0.07, 0, 0.045), (0.07, -0.13, 0.015)),
            "R": ((-0.07, 0, 0.14), (-0.07, -0.01, 0.09), (-0.07, 0, 0.045), (-0.07, -0.13, 0.015))}
    bones = [
        ("root", (0, 0, 0), "world"),
        ("body", (0, 0, 0.12), "root"),
        ("head", (0, -0.04, 0.50), "body"),
        ("hoodtip", (0, 0.03, 0.84), "head"),
        ("specs", SPECS, "head"),
        ("beard1", (0, -0.17, 0.55), "head"),
        ("beard2", (0, -0.21, 0.40), "beard1"),
        ("bubble", (0.05, -0.27, 0.57), "head"),
        ("pack1", (0, 0.18, 0.27), "body"),
        ("pack2", (0.01, 0.20, 0.72), "pack1"),
        ("pack3", (0.04, 0.20, 1.17), "pack2"),
        ("staff", (-0.22, -0.16, 0.44), "root"),
        ("lantern", (-0.40, -0.22, 1.08), "staff"),
        ("ik_R", (-0.22, -0.16, 0.44), "staff"),
        ("ik_L", (0.06, -0.22, 0.38), "body"),
        ("scroll", SCROLL, "body"),
        ("sheet", SCROLL, "scroll"),
    ] + limbs(arms, legs)
    springs = {"hoodtip": (2.0, 0.3, 1, 2.0, 30), "beard1": (1.8, 0.45, -1, 1.0, 9), "beard2": (2.2, 0.45, -1, 1.0, 10),
               "pack2": (1.6, 0.22, 1, 1.5, 12), "pack3": (1.3, 0.2, 1, 2.5, 18), "lantern": (1.4, 0.08, -1, 8.0, 45)}
    look = 0.85
    dist = 4.5
    loop = 8.4
    detail = 1.7

    @staticmethod
    def props(b):
        b.part("bubble")
        C.sphere(b, (0.05, -0.27, 0.57), 0.065, (0.78, 0.90, 1.0), sides=10, rings=6)
        b.part("scroll")
        x, y, z = Folio.SCROLL
        b.push((x, y, z), (0, 90, 0))
        C.lathe(b, [(-0.13, 0.035), (0.13, 0.035)], 8, C.PARCH)
        for s in (-1, 1):
            C.disc(b, (0, 0, s * 0.14), 0.042, 0.02, C.OAK_D, sides=8)
        b.pop()
        b.part("sheet")
        C.box(b, (x, y - 0.035, z - 0.225), (0.24, 0.006, 0.45), C.CREAM)
        for k in range(7):
            C.box(b, (x - 0.02 * (k % 2), y - 0.04, z - 0.06 - k * 0.05), (0.18 - 0.03 * (k % 3), 0.004, 0.012), (0.20, 0.13, 0.08))
        C.box(b, (x, y - 0.04, z - 0.41), (0.06, 0.004, 0.06), C.RUNE)

    @staticmethod
    def idle(t, P, rig, q):
        br = sn(t, 4.2)
        P.rot("body", x=3 + 1.5 * br)
        P.mv("body", (0, 0, 0.006 * br))
        P.rot("head", x=-2 * br + 5 * sn(t, 8.4, 0.25) * (1 - q), z=8 * sn(t, 8.4) * (1 - q))
        P.size("bubble", 0.0)
        P.sc("sheet", (1, 1, 0))
        c = t % 4.2
        stroke = keys(c, [(0, 0), (1.3, 1), (1.8, 1), (3.0, 0)])
        W = rig.fk(P)
        goal = rig.at(W, "head", (0.05, -0.24, 0.44)).lerp(rig.at(W, "head", (0.035, -0.26, 0.28)), stroke)
        rig.place(P, W, "ik_L", goal, 1 - q)

    @staticmethod
    def specs(u, P, rig):
        """Pushes his spectacles up, pats himself all over for them, finds them on his hood."""
        P.rot("head", x=keys(u, [(1.2, 0), (1.6, 22), (2.4, 22), (2.8, 5), (3.6, 5), (3.9, 0), (5.6, 0), (5.75, 8), (5.9, 0), (6.05, 8), (6.2, 0)]),
              z=keys(u, [(2.4, 0), (2.7, 25), (3.1, 25), (3.4, -25), (3.7, -25), (3.9, 0)]),
              y=keys(u, [(3.9, 0), (4.2, 12), (4.6, 12), (4.9, 0)]))
        P.rot("body", x=keys(u, [(1.2, 0), (1.6, 8), (3.6, 8), (3.9, 0)]))
        W = rig.fk(P)
        hd = lambda p: rig.at(W, "head", p)
        s = path(u, [(0.5, hd(Folio.SPECS)), (1.0, hd((0.0, -0.09, 0.76))), (4.6, hd((0.0, -0.09, 0.76))),
                     (5.2, hd((0.03, -0.30, 0.64))), (5.6, hd(Folio.SPECS))])
        rig.place(P, W, "specs", s)
        P.rot("specs", x=keys(u, [(0.5, 0), (1.0, -50), (4.6, -50), (5.2, 0)]))
        grip = s + Vector((0.04, -0.02, -0.02))
        if u < 1.3:
            goal = grip
        elif u < 3.8:
            pats = [rig.at(W, "body", (0.17, -0.22, 0.30)), rig.at(W, "body", (0.10, -0.23, 0.44)),
                    rig.at(W, "body", (0.19, -0.16, 0.40))]
            ph = (u - 1.3) / 0.42
            i, f = int(ph), ph - int(ph)
            a, b = pats[i % 3], pats[(i + 1) % 3]
            goal = a.lerp(b, ss(f)) + Vector((0, -0.06 * math.sin(math.pi * f), 0))
            goal = grip.lerp(goal, ss((u - 1.3) / 0.25))
        elif u < 4.3:
            goal = rig.at(W, "body", (0.10, -0.25, 0.42))
        else:
            goal = grip.lerp(rig.at(W, "body", (0.10, -0.25, 0.42)), 1 - ss((u - 4.3) / 0.3))
        rig.place(P, W, "ik_L", goal, keys(u, [(0, 0), (0.5, 1), (5.6, 1), (6.3, 0)]))

    @staticmethod
    def scroll(u, P, rig):
        """Unrolls a scroll to the floor, reads it, rolls it back up."""
        P.rot("head", x=keys(u, [(1.1, 0), (1.6, 20), (5.0, 20), (5.4, 0)]),
              z=keys(u, [(1.9, 0), (2.2, 12), (2.9, -12), (3.1, 12), (3.8, -12), (4.1, 0), (4.2, 0)]))
        P.rot("head", x=keys(u, [(4.2, 0), (4.35, 9), (4.5, 0), (4.65, 9), (4.8, 0)]))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        sc = path(u, [(0.4, at(Folio.SCROLL)), (0.7, at((0.0, -0.22, 0.40))), (1.1, at((0.0, -0.34, 0.47))),
                      (5.8, at((0.0, -0.34, 0.47))), (6.2, at((0.0, -0.22, 0.40))), (6.4, at(Folio.SCROLL))])
        rig.place(P, W, "scroll", sc)
        P["sheet"][2] = Vector((1.0, 1.0, keys(u, [(1.1, 0), (1.9, 1), (5.0, 1), (5.8, 0)])))
        rig.place(P, W, "ik_L", sc + Vector((0.155, 0.0, 0.0)), keys(u, [(0, 0), (0.45, 1), (6.4, 1), (7.0, 0)]))
        rig.place(P, W, "ik_R", sc + Vector((-0.155, 0.0, 0.0)), keys(u, [(0.2, 0), (0.7, 1), (6.0, 1), (6.6, 0)]))

    @staticmethod
    def doze(u, P, rig):
        """Nods off standing up, a bubble at his nose, jerks awake."""
        P.rot("head", x=keys(u, [(0, 0), (2.4, 34), (3.85, 36), (3.98, -8), (4.4, -4), (5.4, 0)]),
              z=keys(u, [(4.3, 0), (4.6, 22), (4.9, 22), (5.2, -18), (5.5, -18), (5.9, 0)]))
        P.rot("body", x=keys(u, [(0, 0), (2.4, 8), (3.85, 9), (3.98, -4), (4.5, 0)]))
        P.mv("body", (0, 0, keys(u, [(0, 0), (2.4, -0.02), (3.85, -0.02), (3.95, 0.035), (4.2, 0)])))
        if 1.6 < u < 3.85:
            sz = ss((u - 1.6) / 0.4) * (0.65 + 0.35 * math.sin(2 * math.pi * (u - 1.6) / 1.1 - math.pi / 2) + 0.2 * (u - 1.6) / 2.2)
        elif 3.85 <= u < 3.92:
            sz = 1.3 * (1 - (u - 3.85) / 0.07)
        else:
            sz = 0.0
        P.size("bubble", sz)

    @staticmethod
    def greet(u, P, rig):
        """Squints, leans forward, then lights up with a wave of the staff."""
        P.rot("body", x=keys(u, [(0, 0), (0.6, 12), (1.1, 12), (1.4, 0)]))
        P.rot("head", x=keys(u, [(0, 0), (0.6, 16), (1.1, 16), (1.35, -8), (2.4, -8), (2.8, 0)]))
        P.mv("staff", keys(u, [(1.2, (0, 0, 0)), (1.5, (0, 0, 0.10)), (2.4, (0, 0, 0.10)), (2.8, (0, 0, 0))]))
        P.rot("staff", y=16 * math.sin(2 * math.pi * 2 * (u - 1.5)) * env(u, 1.5, 2.4))

    @staticmethod
    def purchase(u, P, rig):
        """Licks a finger, writes it down, stamps it with a wax seal."""
        P.rot("head", x=keys(u, [(0.6, 0), (0.9, 20), (2.5, 20), (2.9, 0)]))
        P.mv("body", (0, 0, keys(u, [(2.3, 0), (2.42, -0.02), (2.6, 0)])))
        W = rig.fk(P)
        at = lambda p: rig.at(W, "body", p)
        mouth = rig.at(W, "head", (0.02, -0.21, 0.56))
        desk = at((0.05, -0.32, 0.40))
        if u < 0.9:
            goal = path(u, [(0.4, mouth), (0.7, mouth), (0.9, desk)])
        elif u < 2.0:
            goal = desk + Vector((0.04 * math.sin(2 * math.pi * 3 * u), 0, 0.012 * math.sin(2 * math.pi * 9 * u)))
        else:
            goal = path(u, [(2.0, desk), (2.25, desk + Vector((0, 0, 0.12))), (2.4, desk - Vector((0, 0, 0.03))), (2.6, desk)])
        rig.place(P, W, "ik_L", goal, keys(u, [(0, 0), (0.4, 1), (2.6, 1), (3.1, 0)]))

    @staticmethod
    def farewell(u, P, rig):
        """Raises his staff, the lantern swinging."""
        P.mv("staff", keys(u, [(0.1, (0, 0, 0)), (0.6, (0.03, -0.02, 0.18)), (1.5, (0.03, -0.02, 0.18)), (2.1, (0, 0, 0))]))
        P.rot("staff", y=keys(u, [(0.1, 0), (0.6, -14), (1.5, -14), (2.1, 0)]))
        P.rot("head", x=keys(u, [(0.2, 0), (0.6, -6), (1.5, -6), (2.0, 0)]), y=keys(u, [(0.6, 0), (0.9, -8), (1.5, -8), (1.9, 0)]))

    @staticmethod
    def victory(u, P, rig):
        """A creaky little jig, the tower swaying wildly."""
        e = env(u, 0.2, 2.8, 0.3)
        P.mv("body", (0, 0, 0.035 * abs(math.sin(2 * math.pi * 1.5 * u)) * e))
        P.rot("body", z=12 * math.sin(2 * math.pi * 0.75 * u) * e, y=6 * math.sin(2 * math.pi * 1.5 * u) * e)
        P.rot("head", x=-6 * e, y=-8 * math.sin(2 * math.pi * 0.75 * u) * e)
        P.mv("staff", (0, 0, 0.04 * abs(math.sin(2 * math.pi * 1.5 * u)) * e))

    @staticmethod
    def defeat(u, P, rig):
        """The top scrolls tumble off his pack, and he sits down on the plinth."""
        P.mv("body", (0, 0, keys(u, [(0.5, 0), (1.1, -0.06)])))
        P.rot("body", x=keys(u, [(0.5, 3), (1.1, 14)]))
        P.rot("head", x=keys(u, [(0.6, 0), (1.3, 22)]))
        fall = ss((u - 0.1) / 0.7)
        P.rot("pack3", y=85 * fall, x=-15 * fall)
        P.mv("pack3", Vector((0.42, -0.08, -1.00)) * (fall ** 1.6))

    fidgets = [("Fidget 1: loses his spectacles (on his own hood)", 6.6, specs),
               ("Fidget 2: unrolls a scroll and reads it", 7.0, scroll),
               ("Fidget 3: nods off standing, jerks awake", 6.2, doze)]
    reactions = [("greet", 2.9, greet), ("purchase", 3.2, purchase), ("farewell", 2.2, farewell),
                 ("victory", 3.2, victory), ("defeat", 1.6, defeat)]


KEEPERS = {k.key: k for k in (Goldwick, Rosette, Slink, Folio)}
