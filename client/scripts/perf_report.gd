extends RefCounted
## `--perf-report SECONDS` (05 §9): measures this many seconds of play, after a few to settle
## (shaders compiling, the map building), then prints a summary and the game quits. Every
## second it prints a line too. The lines start with `MFTR perf`, so a run's output greps:
##
##     godot --path client -- 127.0.0.1:7777 --graphics low --perf-report 30 | grep "MFTR perf"
##
## GPU time is the 3D view's own, measured by the renderer: when it's about the frame time, the
## graphics card is the limit (lower Render scale or World detail); when the frame is longer,
## it's the CPU (the scripts, the simulation).

const SETTLE_S := 4.0

var seconds := 30.0
var _settle := SETTLE_S
var _t := 0.0
var _second := 0.0
var _second_frames := 0
var _frame_ms := PackedFloat32Array()
var _gpu_ms := PackedFloat32Array()
var _cpu_ms := PackedFloat32Array()
var _draw_calls := 0.0
var _primitives := 0.0
var _measuring := false


func _init(seconds_: float) -> void:
	seconds = maxf(seconds_, 1.0)


## Each frame; true once the report is printed (time to quit).
func update(delta: float, viewport: Viewport, playing: bool, settings) -> bool:
	if not playing:
		if _frame_ms.is_empty():
			return false
		# The match ended mid-measurement: report what there is rather than wait for another.
		print("MFTR perf summary: cut short, the match ended after %.1f s" % _t)
		_summary(viewport, settings)
		return true
	var rid := viewport.get_viewport_rid()
	if not _measuring:
		RenderingServer.viewport_set_measure_render_time(rid, true)
		_measuring = true
	if _settle > 0.0:
		_settle -= delta
		return false
	_t += delta
	_second += delta
	_second_frames += 1
	_frame_ms.append(delta * 1000.0)
	_gpu_ms.append(RenderingServer.viewport_get_measured_render_time_gpu(rid))
	_cpu_ms.append(RenderingServer.viewport_get_measured_render_time_cpu(rid) + RenderingServer.get_frame_setup_time_cpu())
	_draw_calls += Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	_primitives += Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME)
	if _second >= 1.0:
		var n := _gpu_ms.size()
		print("MFTR perf: t=%.0f fps=%d frame_ms=%.1f gpu_ms=%.1f render_cpu_ms=%.1f" % [_t, _second_frames,
			_second * 1000.0 / _second_frames, _gpu_ms[n - 1], _cpu_ms[n - 1]])
		_second = 0.0
		_second_frames = 0
	if _t < seconds:
		return false
	_summary(viewport, settings)
	return true


func _summary(viewport: Viewport, settings) -> void:
	var n := _frame_ms.size()
	var frames := _frame_ms.duplicate()
	frames.sort()
	var gpu := _gpu_ms.duplicate()
	gpu.sort()
	var size := Vector2(DisplayServer.window_get_size())
	var scale: float = settings.render_scale / 100.0
	print("MFTR perf summary: %s · %s · window %dx%d, 3D at %dx%d" % [RenderingServer.get_video_adapter_name(),
		RenderingServer.get_current_rendering_method(), size.x, size.y, size.x * scale, size.y * scale])
	print("MFTR perf summary: preset %s · render scale %d%% · AA %s · shadows %s · world detail %s · bloom %s · atmosphere %s" % [
		settings.graphics_preset, settings.render_scale, settings.antialias, settings.shadow_quality,
		settings.world_detail, settings.bloom, settings.atmosphere])
	print("MFTR perf summary: %d frames in %.1f s · %.1f fps · 1%% low %.1f fps" % [n, _t, n / _t, 1000.0 / _percentile(frames, 0.99)])
	print("MFTR perf summary: frame ms mean %.2f · p50 %.2f · p95 %.2f · p99 %.2f · max %.2f" % [_t * 1000.0 / n,
		_percentile(frames, 0.5), _percentile(frames, 0.95), _percentile(frames, 0.99), frames[n - 1]])
	print("MFTR perf summary: GPU ms mean %.2f · p95 %.2f · render CPU ms mean %.2f" % [_mean(_gpu_ms),
		_percentile(gpu, 0.95), _mean(_cpu_ms)])
	print("MFTR perf summary: draw calls %.0f · primitives %.0f (per frame, mean)" % [_draw_calls / n, _primitives / n])


static func _percentile(sorted: PackedFloat32Array, q: float) -> float:
	return sorted[clampi(int(ceil(q * sorted.size())) - 1, 0, sorted.size() - 1)]


static func _mean(a: PackedFloat32Array) -> float:
	var s := 0.0
	for v in a:
		s += v
	return s / maxf(a.size(), 1.0)
