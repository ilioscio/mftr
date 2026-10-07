## The SFX player (05 §7, A4c): positional one-shots from the packs' sounds, which `mftr-pack`
## decodes and hands over as PCM (`MftrModel.sounds()`); pack bytes never reach Godot.
##
## A pool of 3D voices with stealing; variants on one event never repeat back to back; each
## play gets the sound's random pitch spread. The listener sits a few meters above the point the
## camera looks at (not at the camera, high up), so units on screen sound near and those off
## screen fade.
extends Node3D

const VOICES := 24
const LISTENER_HEIGHT := 6.0
const UNIT_SIZE := 9.0            # meters at which a sound plays at its own volume
const MAX_DISTANCE := 40.0

var _voices: Array[AudioStreamPlayer3D] = []
var _listener: AudioListener3D
var _last := {}                   # event key -> the variant played last
var _rng := RandomNumberGenerator.new()


func _ready() -> void:
	_rng.randomize()
	_listener = AudioListener3D.new()
	add_child(_listener)
	_listener.make_current()
	for i in VOICES:
		var p := AudioStreamPlayer3D.new()
		p.unit_size = UNIT_SIZE
		p.max_distance = MAX_DISTANCE
		p.attenuation_filter_db = 0.0   # no distance muffling: the mix stays crisp
		p.panning_strength = 0.6
		add_child(p)
		_voices.append(p)


## Follows the camera's look point (call every frame).
func listen(look: Vector3, basis: Basis) -> void:
	_listener.global_transform = Transform3D(basis, look + Vector3(0, LISTENER_HEIGHT, 0))


## Plays one of `variants` (`MftrModel.sounds()` entries bound to `key`) at `pos`.
func play(key: String, variants: Array, pos: Vector3) -> void:
	if variants.is_empty():
		return
	var pick: Dictionary = variants[0]
	if variants.size() > 1:
		var options := variants.filter(func(v): return v != _last.get(key))
		pick = options[_rng.randi() % options.size()]
	_last[key] = pick
	var v := _free_voice()
	v.stream = pick.stream
	var spread: float = pick.pitch
	v.pitch_scale = 1.0 + _rng.randf_range(-spread, spread)
	v.volume_db = linear_to_db(maxf(float(pick.volume), 0.001))
	v.global_position = pos
	v.play()


## A silent voice, else the one furthest through its sound.
func _free_voice() -> AudioStreamPlayer3D:
	var oldest := _voices[0]
	for v in _voices:
		if not v.playing:
			return v
		if v.get_playback_position() > oldest.get_playback_position():
			oldest = v
	return oldest
