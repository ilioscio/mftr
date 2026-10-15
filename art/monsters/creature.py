# SPDX-License-Identifier: AGPL-3.0-or-later
"""Helpers for `creature` skeletons (16 §3): bone tables for beasts, and gaits keyed by phase.

A table entry is (name, parent, head, tail, deform, roll axis), as `rig.bone_table`'s. `bone`
picks the roll axis from the bone's direction (up-pointing bones roll to forward, lying ones to
up), `table` mirrors the left side and scales the whole thing (a pup is its alpha, smaller).
"""

import math

from mathutils import Vector

from mftr_blender import rig

FORWARD = rig.FORWARD
UP = rig.UP


def bone(name, parent, head, tail, deform=True):
    d = Vector(tail) - Vector(head)
    axis = FORWARD if abs(d.z) > max(abs(d.x), abs(d.y)) else UP
    return (name, parent, tuple(head), tuple(tail), deform, axis)


def socket(name, parent, at):
    return (name, parent, tuple(at), (at[0], at[1] - 0.1, at[2]), False, UP)


def table(center, left, sockets, scale=1.0):
    """`center` + `left` (mirrored to the right) + `sockets`, scaled about the origin."""
    full = list(center) + rig.mirrored(left) + list(sockets)
    s = scale
    return [(n, p, tuple(x * s for x in h), tuple(x * s for x in t), d, a) for n, p, h, t, d, a in full]


def point(tab, name, t, off=(0.0, 0.0, 0.0)):
    """A point `t` of the way along `name` in `tab`, offset (meters, armature axes)."""
    for n, _, h, tl, _, _ in tab:
        if n == name:
            return tuple(h[i] + (tl[i] - h[i]) * t + off[i] for i in range(3))
    raise KeyError(name)


def wave(phase, lo, hi, sharp=1.0):
    """A value swinging between `lo` and `hi` over a cycle (phase in [0, 1)), `sharp` > 1
    lingering at the ends."""
    s = math.sin(2 * math.pi * phase)
    s = math.copysign(abs(s) ** (1.0 / sharp), s)
    return lo + (hi - lo) * (0.5 + 0.5 * s)
