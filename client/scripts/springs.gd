extends RefCounted
## Secondary motion (22 §4.5, BW6): a forged champion's hanging hair and cloth swing on Godot's
## spring bones. The forge keeps the worn parts' chains (`extra_<part>_<chain>_<n>`, laid at
## build time by `mftr_blender.chains`); no clip keys them, so they are simulated here, visual
## only, after the animator's pose each frame. The body is their colliders: the head, the chest,
## the hips, the upper arms and the legs, a little inside the skin so hair lying on the
## shoulders and a skirt on the hips rest where they were built.
##
## They swing in the champion's own space (its body node's): a champion running at 3.5 m/s
## would otherwise stream everything out flat behind it, and a blink would yank it. What moving
## does to them instead is a headwind (`wind`), from the champion's ground velocity: the stride,
## the turns and the clips' own motion swing them; the run blows them back.

## How each kind of chain moves: [stiffness, drag, gravity, joint radius (m)].
const FEEL := {
	# Hair: loose, settles quickly.
	"hair": [0.5, 0.4, 0.3, 0.012],
	"beard": [0.8, 0.5, 0.2, 0.012],
	# Cloth: heavier, holds its drape, swings with the stride.
	"cloth": [0.9, 0.5, 0.2, 0.02],
}

## The headwind for each m/s of ground speed.
const WIND := 0.1

## Beyond this far from the camera (m) a champion's chains rest as built: too small to see swing.
const FAR := 45.0

## Colliders: [bone, toward the child bone (a capsule along the bone), radius (m) on the slim
## build], spheres where there is no child.
const BODY := [
	["head", "", 0.085],
	["spine_02", "chest", 0.11],
	["chest", "neck", 0.12],
	["pelvis", "spine_01", 0.13],
	["upperarm_l", "forearm_l", 0.045],
	["upperarm_r", "forearm_r", 0.045],
	["thigh_l", "calf_l", 0.07],
	["thigh_r", "calf_r", 0.07],
	["calf_l", "foot_l", 0.05],
	["calf_r", "foot_r", 0.05],
]

## The slim build's hips (between the thigh joints, m): colliders scale with a broader build.
const HIPS := 0.18


## Give `model` (from `source.instantiate()`) its spring bones, if it has chains to swing.
## Returns the simulator, or null.
static func attach(model: Node3D, source: MftrModel) -> SpringBoneSimulator3D:
	var chains: Array = source.spring_chains()
	# `--no-springs`: everything hangs as built (a comparison, a slow machine).
	if chains.is_empty() or "--no-springs" in OS.get_cmdline_user_args():
		return null
	var skeleton: Skeleton3D = model.get_node("Skeleton")
	var sim := SpringBoneSimulator3D.new()
	sim.name = "Springs"
	skeleton.add_child(sim)
	sim.setting_count = chains.size()
	# The champion's body node (the model's parent): its travel isn't the chains' motion.
	var center := sim.get_path_to(model.get_parent()) if model.get_parent() != null else NodePath()
	for i in chains.size():
		if not center.is_empty():
			sim.set_center_from(i, SpringBoneSimulator3D.CENTER_FROM_NODE)
			sim.set_center_node(i, center)
		var chain: String = chains[i][0]
		var feel: Array = FEEL["hair"] if chain.begins_with("hair_") else (
			FEEL["beard"] if chain.begins_with("beard_") else FEEL["cloth"])
		sim.set_root_bone_name(i, chains[i][1])
		sim.set_end_bone_name(i, chains[i][2])
		sim.set_extend_end_bone(i, true)
		sim.set_end_bone_direction(i, SpringBoneSimulator3D.BONE_DIRECTION_FROM_PARENT)
		sim.set_end_bone_length(i, _bone_length(skeleton, chains[i][2]))
		sim.set_stiffness(i, feel[0])
		sim.set_drag(i, feel[1])
		sim.set_gravity(i, feel[2])
		sim.set_gravity_direction(i, Vector3.DOWN)
		sim.set_radius(i, feel[3])
	_colliders(sim, skeleton)
	return sim


## The headwind of moving at `velocity` (world, m/s) on `sim` (from `attach`; null is fine).
static func wind(sim: SpringBoneSimulator3D, velocity: Vector3) -> void:
	if sim == null:
		return
	var v := Vector3(velocity.x, 0.0, velocity.z)
	var was: Vector3 = sim.get_meta("wind", Vector3.ZERO)
	var now := was.lerp(-v * WIND, 0.2)
	sim.set_meta("wind", now)
	sim.external_force = now


## Swing only what can be seen (22 §4.5): `sim` pauses while its champion at `at` (world) is out
## of `camera`'s view or past `FAR`, its chains resting as built, and starts again from rest.
static func detail(sim: SpringBoneSimulator3D, camera: Camera3D, at: Vector3) -> void:
	if sim == null or camera == null:
		return
	var seen := camera.global_position.distance_to(at) < FAR and (
		camera.is_position_in_frustum(at) or camera.is_position_in_frustum(at + Vector3(0, 1.8, 0)))
	if seen != sim.active:
		sim.active = seen
		if seen:
			sim.reset()


## The last bone's own length: as long as the bone before it.
static func _bone_length(skeleton: Skeleton3D, bone_name: String) -> float:
	var b := skeleton.find_bone(bone_name)
	if b < 0:
		return 0.05
	return maxf(skeleton.get_bone_rest(b).origin.length(), 0.02)


static func _colliders(sim: SpringBoneSimulator3D, skeleton: Skeleton3D) -> void:
	var k := 1.0
	var l := skeleton.find_bone("thigh_l")
	var r := skeleton.find_bone("thigh_r")
	if l >= 0 and r >= 0:
		var gap := skeleton.get_bone_global_rest(l).origin.distance_to(skeleton.get_bone_global_rest(r).origin)
		k = clampf(gap / HIPS, 0.8, 1.6)
	for c in BODY:
		var b := skeleton.find_bone(c[0])
		if b < 0:
			continue
		var radius: float = c[2] * k
		var to := skeleton.find_bone(c[1]) if c[1] != "" else -1
		var shape: SpringBoneCollision3D
		if to >= 0:
			# Along the bone to its child, in the bone's own frame.
			var a := skeleton.get_bone_global_rest(b)
			var end := a.affine_inverse() * skeleton.get_bone_global_rest(to).origin
			var cap := SpringBoneCollisionCapsule3D.new()
			cap.radius = radius
			cap.height = end.length() + radius * 2.0
			cap.position_offset = end * 0.5
			cap.rotation_offset = Quaternion(Vector3.UP, end.normalized()) if end.length() > 1e-4 else Quaternion()
			shape = cap
		else:
			# The skull: a sphere above the head's joint.
			var sphere := SpringBoneCollisionSphere3D.new()
			sphere.radius = radius
			var up := skeleton.get_bone_global_rest(b).basis.inverse() * Vector3.UP
			sphere.position_offset = up.normalized() * radius * 1.1
			shape = sphere
		shape.name = "Collide_%s" % c[0]
		sim.add_child(shape)
		shape.bone_name = c[0]
