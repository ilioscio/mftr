extends Node
## Is a newer release out? At start the client fetches the newest release's manifest
## (`latest.json`) and its signature from GitHub, and `MftrRelease.check` verifies them against
## the release key it carries. Offline, or with a manifest that doesn't verify, it stays quiet:
## the game plays the same.

signal checked(info: Dictionary)

## `MftrRelease.check`'s answer (`version`, `newer`, `page`, `download`) once a manifest
## verified; empty before.
var info := {}
## `--update-manifest URL`: another manifest to check (its signature at URL + ".sig"), for
## testing a release before it's out.
var manifest_url: String = MftrRelease.manifest_url()


func start() -> void:
	var json := await _fetch(manifest_url)
	var sig := await _fetch(manifest_url + ".sig") if not json.is_empty() else PackedByteArray()
	if sig.is_empty():
		return
	var result: Dictionary = MftrRelease.check(json, sig)
	if result.has("error"):
		print("MFTR: %s" % result["error"])
		return
	info = result
	if info.get("newer", false):
		print("MFTR: %s is out (this is %s)" % [info["version"], MftrRelease.current()])
	checked.emit(info)


## A release to get instead of this one: the newest, when it's newer. Empty otherwise.
func newer_version() -> String:
	return info.get("version", "") if info.get("newer", false) else ""


## `url`'s body, or nothing when it can't be had (offline, no manifest yet: GitHub answers 404
## until a release carries one).
func _fetch(url: String) -> PackedByteArray:
	var http := HTTPRequest.new()
	http.timeout = 15.0
	http.body_size_limit = 1 << 20
	add_child(http)
	var body := PackedByteArray()
	if http.request(url) == OK:
		var r: Array = await http.request_completed
		if r[0] == HTTPRequest.RESULT_SUCCESS and r[1] == 200:
			body = r[3]
	http.queue_free()
	return body
