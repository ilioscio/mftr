# SPDX-License-Identifier: AGPL-3.0-or-later
"""Natural wrists for weapons (14 §3.3): a pass run once a model and its clips are built.

Weapons were laid along the hand that holds them, so a clip aimed a weapon by turning the whole
hand: a maul's swing bent the wrist wherever the maul had to point, often 60–90° off the
forearm. Here:

1. **Weapons move to the prop bones.** Every piece skinned to a hand that is long or stands off
   from the hand (a haft, a blade, a bow; whole mesh islands, so nothing tears at the fist), and
   any small piece touching one (a crossguard, a ring on the haft), is rebound to that side's
   `prop_l` / `prop_r` (at the palm), which become deforming bones. The palm, the fingers and
   close-fitting gear (gauntlets, knuckle guards) stay on the hand.
2. **Every clip's wrists are limited.** At each key the hand may bend at most `MAX_BEND` off
   where the forearm would carry it at rest. The weapon stays exactly where the clip put it (the
   prop bone is keyed to hold it there) and the arm is re-solved so the fist (the prop bone's
   head) still closes on it: the elbow and shoulder take up the turn, as a person's do. The off
   hand's grips (`clips.grip`) were solved against that same weapon, so they still hold it.

The clips' authoring (`clips.aim` on the hand) stays as it is: this is a correction after it.
"""

import math

import bmesh
import bpy
from mathutils import Matrix, Quaternion, Vector

MAX_BEND = math.radians(35.0)
# A hand-skinned island is a weapon when it reaches this far from the hand's bones, or is this
# long; a smaller one is part of the weapon when it comes this close to a weapon island.
FAR = 0.11
LONG = 0.22
TOUCH = 0.012
# How far the fist may sit from the weapon's grip.
GAP = 0.015


def _segment_distance(p, a, b):
    ab = b - a
    t = max(0.0, min(1.0, (p - a).dot(ab) / max(ab.length_squared, 1e-9)))
    return (p - (a + ab * t)).length


def _islands(verts):
    """Connected groups among `verts` (BMVerts)."""
    todo = set(verts)
    out = []
    while todo:
        v0 = todo.pop()
        island, stack = [v0], [v0]
        while stack:
            v = stack.pop()
            for e in v.link_edges:
                w = e.other_vert(v)
                if w in todo:
                    todo.discard(w)
                    island.append(w)
                    stack.append(w)
        out.append(island)
    return out


def _bounds(pts):
    lo = Vector(tuple(min(p[i] for p in pts) for i in range(3)))
    hi = Vector(tuple(max(p[i] for p in pts) for i in range(3)))
    return lo, hi


def _near(a, b, gap):
    return all(a[0][i] - gap <= b[1][i] and b[0][i] - gap <= a[1][i] for i in range(3))


def rebind_weapons(armature, meshes):
    """Move weapon islands from `hand_<s>` to `prop_<s>`. Returns {side: islands moved}."""
    moved = {"l": 0, "r": 0}
    bones = armature.data.bones
    mw = armature.matrix_world
    for side in "lr":
        hand, fingers, prop = f"hand_{side}", f"fingers_{side}", f"prop_{side}"
        if hand not in bones or prop not in bones:
            continue
        segs = [(mw @ bones[b].head_local, mw @ bones[b].tail_local) for b in (hand, fingers) if b in bones]
        for obj in meshes:
            g_hand = obj.vertex_groups.get(hand)
            if g_hand is None:
                continue
            bm = bmesh.new()
            bm.from_mesh(obj.data)
            deform = bm.verts.layers.deform.verify()
            on_hand = [v for v in bm.verts if v[deform].get(g_hand.index, 0.0) > 0.5]
            islands = []
            for island in _islands(on_hand):
                pts = [obj.matrix_world @ v.co for v in island]
                far = max(min(_segment_distance(p, a, b) for a, b in segs) for p in pts)
                box = _bounds(pts)
                islands.append((island, box, far >= FAR or (box[1] - box[0]).length >= LONG))
            weapon = [i for i in islands if i[2]]
            # Small pieces touching the weapon (a crossguard, collars), but not the fist itself.
            weapon += [i for i in islands if not i[2] and any(_near(i[1], w[1], TOUCH) for w in weapon)
                       and min(_segment_distance(p, *segs[0]) for p in (i[1][0], i[1][1])) > 0.03]
            if not weapon:
                bm.free()
                continue
            g_prop = obj.vertex_groups.get(prop) or obj.vertex_groups.new(name=prop)
            for island, _, _ in weapon:
                for v in island:
                    w = v[deform][g_hand.index]
                    del v[deform][g_hand.index]
                    v[deform][g_prop.index] = w
            moved[side] += len(weapon)
            bm.to_mesh(obj.data)
            bm.free()
    if any(moved.values()):
        prev = bpy.context.view_layer.objects.active
        bpy.context.view_layer.objects.active = armature
        bpy.ops.object.mode_set(mode="EDIT")
        for side in "lr":
            eb = armature.data.edit_bones.get(f"prop_{side}")
            if eb is not None:
                eb.use_deform = True
                eb.use_connect = False
        bpy.ops.object.mode_set(mode="OBJECT")
        bpy.context.view_layer.objects.active = prev
    return moved


def _rot(m):
    return m.to_3x3().normalized().to_quaternion()


def _fcurves(action):
    from bpy_extras import anim_utils
    for slot in action.slots:
        bag = anim_utils.action_get_channelbag_for_slot(action, slot)
        if bag is not None:
            yield from bag.fcurves


def _bone_of(fc):
    return fc.data_path.split('"')[1] if '"' in fc.data_path else ""


def _key_frames(action, names):
    frames = set()
    for fc in _fcurves(action):
        if _bone_of(fc) in names:
            frames.update(int(round(k.co[0])) for k in fc.keyframe_points)
    return sorted(frames)


def _set(pb, m):
    pb.matrix = m
    bpy.context.view_layer.update()


def _turned(pb, new_dir):
    """`pb`'s armature matrix turned (shortest arc) so it points along `new_dir`."""
    cur = (pb.tail - pb.head).normalized()
    q = cur.rotation_difference(new_dir.normalized()) @ _rot(pb.matrix)
    return Matrix.Translation(pb.head) @ q.to_matrix().to_4x4()


def _reach(upper, lower, wrist, pole_dir):
    """Turn `upper` and `lower` so `lower`'s tail lands on `wrist` (or as near as it reaches),
    the elbow toward `pole_dir`."""
    s = upper.head.copy()
    l1, l2 = upper.bone.length, lower.bone.length
    dv = wrist - s
    d = max(min(dv.length, (l1 + l2) * 0.999), abs(l1 - l2) + 1e-4)
    dirn = dv.normalized()
    p = pole_dir - dirn * pole_dir.dot(dirn)
    p = p.normalized() if p.length > 1e-6 else Vector((0, 0, -1))
    a = (l1 * l1 - l2 * l2 + d * d) / (2 * d)
    h = math.sqrt(max(l1 * l1 - a * a, 0.0))
    mid = s + dirn * a + p * h
    _set(upper, _turned(upper, mid - s))
    _set(lower, _turned(lower, s + dirn * d - mid))


def _limit(natural, authored, max_bend):
    d = natural.inverted() @ authored
    if d.w < 0:
        d.negate()
    angle = 2.0 * math.acos(min(1.0, d.w))
    if angle <= max_bend:
        return authored, angle
    return natural @ Quaternion().slerp(d, max_bend / angle), angle


def _hold(upper, fore, hand, h_old, fist, fist_local, pole, natural, max_bend):
    """Pose the arm so the fist (`fist_local` in the hand) is on `fist` with the wrist bent at
    most `max_bend`, as near the authored hand `h_old` as that allows: the wrist slides on the
    sphere round the fist between where a straight wrist puts it and where the clip had it."""
    r = fist_local.length
    authored = _rot(h_old)

    def place(wrist):
        _reach(upper, fore, wrist, pole)
        # The authored hand turned (shortest arc) to close on the fist, its roll kept inside the
        # limit, and turned onto the fist again.
        q = (authored @ fist_local).rotation_difference(fist - fore.tail) @ authored
        q, _ = _limit(natural(), q, max_bend)
        q = (q @ fist_local).rotation_difference(fist - fore.tail) @ q
        _set(hand, Matrix.Translation(fore.tail) @ q.to_matrix().to_4x4())
        d = natural().inverted() @ q
        return 2.0 * math.acos(min(1.0, abs(d.w)))

    # The straight wrist: the fist on the forearm's line (a two-bone reach with the hand added
    # to the forearm).
    sh = upper.head.copy()
    l1, l2 = upper.bone.length, fore.bone.length + r
    dv = fist - sh
    d = max(min(dv.length, (l1 + l2) * 0.999), abs(l1 - l2) + 1e-4)
    dirn = dv.normalized()
    p = pole - dirn * pole.dot(dirn)
    p = p.normalized() if p.length > 1e-6 else Vector((0, 0, -1))
    a = (l1 * l1 - l2 * l2 + d * d) / (2 * d)
    mid = sh + dirn * a + p * math.sqrt(max(l1 * l1 - a * a, 0.0))
    straight = mid + (fist - mid).normalized() * fore.bone.length
    old = h_old.to_translation()

    def wrist_at(t):
        w = straight.lerp(old, t)
        v = fist - w
        return fist - (v.normalized() if v.length > 1e-6 else dirn) * r

    lo, hi = 0.0, 1.0
    for _ in range(14):
        mid_t = (lo + hi) / 2
        if place(wrist_at(mid_t)) <= max_bend + 1e-3:
            lo = mid_t
        else:
            hi = mid_t
    return place(wrist_at(lo))


def correct_clips(armature, max_bend=MAX_BEND):
    """Limit both wrists in every clip on `armature`'s NLA tracks (see the module). A clip with
    a wrist past the limit has that arm and its prop baked on every frame (linear keys: the
    exporter samples every frame anyway), so the fist never drifts off the weapon between keys.
    Returns (clips touched, frames corrected, the worst bend found in degrees)."""
    ad = armature.animation_data
    if ad is None:
        return 0, 0, 0.0
    pb = armature.pose.bones
    bones = armature.data.bones
    sides = [s for s in "lr" if all(f"{b}_{s}" in pb for b in ("upperarm", "forearm", "hand", "prop"))]
    if not sides:
        return 0, 0, 0.0
    chain = {s: [f"{b}_{s}" for b in ("upperarm", "forearm", "hand", "prop")] for s in sides}
    for s in sides:
        for n in chain[s]:
            pb[n].rotation_mode = "QUATERNION"
    rest = {n: bones[n].matrix_local.copy() for s in sides for n in chain[s]}
    scene = bpy.context.scene
    actions = [st.action for tr in ad.nla_tracks for st in tr.strips if st.action is not None]
    touched, fixed, worst = 0, 0, 0.0
    stuck = {}
    saved_action = ad.action

    def natural_of(s):
        return _rot(pb[f"forearm_{s}"].matrix) @ _rot(rest[f"forearm_{s}"]).inverted() @ _rot(rest[f"hand_{s}"])

    for act in actions:
        if not _key_frames(act, {n for s in sides for n in chain[s][:3]}):
            continue
        ad.action = act
        if getattr(ad, "action_slot", None) is None and act.slots:
            ad.action_slot = act.slots[0]
        start, end = (int(round(v)) for v in act.frame_range)
        frames = list(range(start, end + 1))
        # Every frame's authored pose first (re-keying one frame would bend the curves under
        # the next).
        authored = {}
        over = set()
        for f in frames:
            scene.frame_set(f)
            for s in sides:
                hand = pb[f"hand_{s}"]
                _, angle = _limit(natural_of(s), _rot(hand.matrix), max_bend)
                worst = max(worst, math.degrees(angle))
                if angle > max_bend + 1e-3:
                    over.add(s)
                up, fore = pb[f"upperarm_{s}"], pb[f"forearm_{s}"]
                authored[f, s] = ([pb[n].rotation_quaternion.copy() for n in chain[s][:3]],
                                  [pb[n].location.copy() for n in chain[s][:3]],
                                  hand.matrix.copy(), fore.head - up.head, fore.tail - up.head)
        if not over:
            continue
        for s in over:
            _clear(act, set(chain[s]))
        for f in frames:
            scene.frame_set(f)
            for s in over:
                quats, locs, h_old, elbow, reach_to = authored[f, s]
                upper, fore, hand, prop = (pb[n] for n in chain[s])
                for n, q, loc in zip(chain[s][:3], quats, locs):
                    pb[n].rotation_quaternion = q
                    pb[n].location = loc
                prop.rotation_quaternion = Quaternion()
                prop.location = Vector()
                bpy.context.view_layer.update()
                if _limit(natural_of(s), _rot(h_old), max_bend)[1] > max_bend + 1e-3:
                    weapon = h_old @ rest[f"hand_{s}"].inverted() @ rest[f"prop_{s}"]
                    fist = weapon.to_translation()
                    fist_local = (rest[f"hand_{s}"].inverted() @ rest[f"prop_{s}"]).to_translation()
                    # The elbow keeps bending the way it did.
                    d = reach_to.normalized()
                    pole = elbow - d * elbow.dot(d)
                    bend = _hold(upper, fore, hand, h_old, fist, fist_local, pole, lambda: natural_of(s), max_bend)
                    if bend > max_bend + math.radians(5):
                        # The fist is too close to the shoulder for a straight wrist (a pose to
                        # fix where it's authored).
                        stuck.setdefault(act.name, []).append(f)
                    _set(prop, weapon)
                    fixed += 1
                for b in (upper, fore, hand, prop):
                    b.keyframe_insert("rotation_quaternion", frame=f, group=b.name)
                    b.keyframe_insert("location", frame=f, group=b.name)
        names = {n for s in over for n in chain[s]}
        _continuous(act, names)
        for fc in _fcurves(act):
            if _bone_of(fc) in names:
                for kp in fc.keyframe_points:
                    kp.interpolation = "LINEAR"
        touched += 1
    ad.action = saved_action
    for s in sides:
        for n in chain[s]:
            pb[n].rotation_quaternion = Quaternion()
            pb[n].location = Vector()
    for name, fs in stuck.items():
        print(f"wrists: {name}: the fist is too close to the shoulder for a straight wrist on frames {fs}")
    return touched, fixed, worst


def _clear(action, names):
    """Remove `names`' rotation and location keys from `action` (they are baked anew)."""
    from bpy_extras import anim_utils
    for slot in action.slots:
        bag = anim_utils.action_get_channelbag_for_slot(action, slot)
        if bag is None:
            continue
        for fc in [fc for fc in bag.fcurves if _bone_of(fc) in names
                   and fc.data_path.rsplit(".", 1)[-1] in ("rotation_quaternion", "location")]:
            bag.fcurves.remove(fc)


def _continuous(action, names):
    """Keep each bone's quaternion keys on one hemisphere, key to key, so they interpolate the
    short way (re-keyed bones come back from matrices with arbitrary signs)."""
    curves = {}
    for fc in _fcurves(action):
        if fc.data_path.endswith("rotation_quaternion") and _bone_of(fc) in names:
            curves.setdefault(_bone_of(fc), [None] * 4)[fc.array_index] = fc
    for fcs in curves.values():
        if any(c is None for c in fcs) or len({len(c.keyframe_points) for c in fcs}) != 1:
            continue
        prev = None
        for i in range(len(fcs[0].keyframe_points)):
            q = Vector([c.keyframe_points[i].co[1] for c in fcs])
            if prev is not None and q.dot(prev) < 0:
                for c in fcs:
                    kp = c.keyframe_points[i]
                    kp.co[1] = -kp.co[1]
                    kp.handle_left[1] = -kp.handle_left[1]
                    kp.handle_right[1] = -kp.handle_right[1]
                q = -q
            prev = q
        for c in fcs:
            c.update()


def fix(armature, meshes):
    """Both steps, for a model whose clips are all on its NLA tracks."""
    moved = rebind_weapons(armature, meshes)
    touched, fixed, worst = correct_clips(armature)
    print(f"wrists: weapon pieces on the prop bones {moved}; {fixed} wrist frames limited in {touched} clips "
          f"(the worst bend was {worst:.0f}°)")
    return moved, touched, fixed
