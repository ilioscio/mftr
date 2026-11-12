## Face life (22 §2.4): the eyes of a champion with eye bones look about and blink. Visual only:
## six bone poses a frame, never in the sim, never in the clips. `tick` after the animator
## drives the skeleton.
##
## The eyes look where they're told (`look_at`, model space) with small saccades about it; the
## upper lids follow the eyes' pitch; a blink every 2–6 s, now and then a double one, closing
## faster than it opens.
extends RefCounted

## How far a blink turns each lid (radians, about the eye's left–right axis): the upper lid
## down from where it rests over the iris to meet the lower, which rises a little.
const UPPER_CLOSE := 0.86
const LOWER_CLOSE := 0.12
## How far the eyes turn at most (radians): left–right, up–down.
const YAW_MAX := 0.35
const PITCH_MAX := 0.22
## The upper lid follows this much of the eyes' pitch.
const LID_FOLLOW := 0.7
## A blink's length (s): closing, then opening.
const CLOSE_TIME := 0.06
const OPEN_TIME := 0.1

var _look := {}       # side -> bone index (eye_look_l, eye_look_r)
var _upper := {}      # side -> bone index
var _lower := {}
var _rest := {}       # bone index -> its rest rotation (local)
var _rng := RandomNumberGenerator.new()
var _gaze := Vector2.ZERO       # (yaw, pitch) now
var _gaze_to := Vector2.ZERO    # where the next saccade lands
var _saccade_in := 0.8
var _blink_in := 2.5
var _blink_t := -1.0            # time into a blink, -1 when not blinking
var _again := false             # a double blink's second
## Set to hold a pose for captures: (blink 0–1, yaw, pitch); `null` lives.
var fixed = null


## Find the eye bones of `skeleton`; false when it has none (`tick` then does nothing).
func setup(skeleton: Skeleton3D, seed := 0) -> bool:
	_rng.seed = seed if seed != 0 else randi()
	for side in ["l", "r"]:
		var look := skeleton.find_bone("eye_look_" + side)
		var upper := skeleton.find_bone("lid_upper_" + side)
		var lower := skeleton.find_bone("lid_lower_" + side)
		if look < 0 or upper < 0 or lower < 0:
			_look.clear()
			return false
		_look[side] = look
		_upper[side] = upper
		_lower[side] = lower
		for b in [look, upper, lower]:
			_rest[b] = skeleton.get_bone_rest(b).basis.get_rotation_quaternion()
	_blink_in = _rng.randf_range(0.5, 3.0)
	return true


func active() -> bool:
	return not _look.is_empty()


## Pose the eyes for this frame. `target`: what to look at, in the skeleton's space (the camera
## in the creator), or Vector3.INF to look ahead.
func tick(skeleton: Skeleton3D, delta: float, target := Vector3.INF) -> void:
	if _look.is_empty():
		return
	var blink := 0.0
	var aim := Vector2.ZERO
	if fixed != null:
		blink = fixed.x
		_gaze = Vector2(fixed.y, fixed.z)
	else:
		aim = _aim(skeleton, target)
		_saccade_in -= delta
		if _saccade_in <= 0.0:
			# A small jump about the point looked at: eyes never hold perfectly still.
			_gaze_to = aim + Vector2(_rng.randfn(0.0, 0.07), _rng.randfn(0.0, 0.04))
			_saccade_in = _rng.randf_range(0.6, 3.0)
		_gaze = _gaze.lerp(_gaze_to, minf(1.0, delta * 30.0))
		blink = _blink(delta)
	_gaze.x = clampf(_gaze.x, -YAW_MAX, YAW_MAX)
	_gaze.y = clampf(_gaze.y, -PITCH_MAX, PITCH_MAX)
	for side in ["l", "r"]:
		var parent := skeleton.get_bone_parent(_look[side])
		var p := skeleton.get_bone_global_pose(parent).basis.orthonormalized().get_rotation_quaternion() if parent >= 0 else Quaternion.IDENTITY
		# Turns given in the skeleton's space (+Z ahead, +Y up, +X the character's left), taken
		# into each bone's own frame: local = P⁻¹ · turn · P · rest.
		var gaze := Quaternion(Vector3.UP, _gaze.x) * Quaternion(Vector3.RIGHT, -_gaze.y)
		_turn(skeleton, _look[side], p, gaze)
		var up_turn := blink * UPPER_CLOSE - _gaze.y * LID_FOLLOW
		_turn(skeleton, _upper[side], p, Quaternion(Vector3.RIGHT, up_turn))
		_turn(skeleton, _lower[side], p, Quaternion(Vector3.RIGHT, -blink * LOWER_CLOSE))


func _turn(skeleton: Skeleton3D, bone: int, p: Quaternion, turn: Quaternion) -> void:
	skeleton.set_bone_pose_rotation(bone, p.inverse() * turn * p * _rest[bone])


## (yaw, pitch) toward `target` from between the eyes, or ahead.
func _aim(skeleton: Skeleton3D, target: Vector3) -> Vector2:
	if target == Vector3.INF:
		return Vector2.ZERO
	var mid := (skeleton.get_bone_global_pose(_look["l"]).origin + skeleton.get_bone_global_pose(_look["r"]).origin) * 0.5
	var d := target - mid
	if d.length() < 0.01:
		return Vector2.ZERO
	d = d.normalized()
	return Vector2(atan2(d.x, d.z), asin(clampf(d.y, -1.0, 1.0)))


## How closed the lids are now (0 open, 1 shut), counting down to the next blink.
func _blink(delta: float) -> float:
	if _blink_t < 0.0:
		_blink_in -= delta
		if _blink_in > 0.0:
			return 0.0
		_blink_t = 0.0
	_blink_t += delta
	if _blink_t < CLOSE_TIME:
		return _blink_t / CLOSE_TIME
	if _blink_t < CLOSE_TIME + OPEN_TIME:
		var u := (_blink_t - CLOSE_TIME) / OPEN_TIME
		return 1.0 - u * u * (3.0 - 2.0 * u)
	_blink_t = -1.0
	if not _again and _rng.randf() < 0.15:
		_again = true
		_blink_in = 0.12
	else:
		_again = false
		_blink_in = _rng.randf_range(2.0, 6.0)
	return 0.0
