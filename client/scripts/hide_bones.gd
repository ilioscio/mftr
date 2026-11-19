extends SkeletonModifier3D
## Hides bones (and what they carry) by scaling them to nothing after the clips have posed the
## skeleton: a grapple's hand while it flies out on the stretched arm (14 §4.4). Each `hold`
## is matched by a `release`; a bone shows again when nothing holds it.

var _holds := {}                        # bone index -> holds


func hold(bone: String) -> void:
	var i := get_skeleton().find_bone(bone)
	if i >= 0:
		_holds[i] = _holds.get(i, 0) + 1


func release(bone: String) -> void:
	var i := get_skeleton().find_bone(bone)
	if _holds.has(i):
		_holds[i] -= 1
		if _holds[i] <= 0:
			_holds.erase(i)


func _process_modification() -> void:
	var skel := get_skeleton()
	if skel == null:
		return
	for i in _holds:
		skel.set_bone_pose_scale(i, Vector3.ONE * 0.0001)


## The hider on `skel`, made the first time it's asked for.
static func on(skel: Skeleton3D) -> Node:
	var h := skel.get_node_or_null("HideBones")
	if h == null:
		h = load("res://scripts/hide_bones.gd").new()
		h.name = "HideBones"
		skel.add_child(h)
	return h
