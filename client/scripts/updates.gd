extends Node
## Is a newer release out? At start the client fetches the newest release's manifest
## (`latest.json`) and its signature from GitHub, and `MftrRelease.check` verifies them against
## the release key it carries. Offline, or with a manifest that doesn't verify, it stays quiet:
## the game plays the same.
##
## On Windows the client then installs the new release itself (`update_now`): it downloads the
## archive the manifest names into `.update/` beside the game, `MftrRelease.install` checks it
## against the manifest's SHA-256 and swaps it in, and the new game starts in place of this one.

signal checked(info: Dictionary)
## What an update in progress is doing ("Downloading MFTR 0.18.0… 12 / 40 MB"), or why it stopped.
signal progress(text: String)

## `MftrRelease.check`'s answer (`version`, `newer`, `page`, `download`, `size`, `sha256`) once a
## manifest verified; empty before.
var info := {}
## `--update-manifest URL`: another manifest to check (its signature at URL + ".sig"), for
## testing a release before it's out.
var manifest_url: String = MftrRelease.manifest_url()
## The game's folder, which an update replaces. `--update-install-dir DIR`: another one, so a run
## from the editor can update an unpacked release (testing).
var install_dir: String = OS.get_executable_path().get_base_dir()
var _install_dir_set := false
var _download: HTTPRequest


func set_install_dir(dir: String) -> void:
	install_dir = dir
	_install_dir_set = true


func start() -> void:
	# What the last update replaced; its program has exited by now.
	if _installed():
		MftrRelease.clean(install_dir)
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


## Whether this client can install the newer release itself: Windows, an exported game (not the
## editor), in a folder it may write (not Program Files).
func can_update_itself() -> bool:
	if newer_version() == "" or info.get("sha256", "") == "" or not MftrRelease.updates_itself():
		return false
	return _installed() and MftrRelease.can_install(install_dir)


## Whether there is an installed game to update (an exported one, or `--update-install-dir`).
func _installed() -> bool:
	return MftrRelease.updates_itself() and (OS.has_feature("template") or _install_dir_set)


func updating() -> bool:
	return _download != null


## Download the newer release, install it and start it with `args` (user arguments, after
## "--"), quitting this game. Says how it goes through `progress`. Returns why it failed; the
## game is then as it was.
func update_now(args := PackedStringArray()) -> String:
	if not can_update_itself() or updating():
		return "this game can't update itself"
	var version: String = info["version"]
	var archive := install_dir.path_join(".update").path_join(String(info["download"]).get_file())
	if DirAccess.make_dir_recursive_absolute(archive.get_base_dir()) != OK:
		return _failed("could not make %s" % archive.get_base_dir())
	_download = HTTPRequest.new()
	_download.download_file = archive
	_download.download_chunk_size = 1 << 16
	add_child(_download)
	var r: Array = [HTTPRequest.RESULT_CANT_CONNECT, 0]
	if _download.request(info["download"]) == OK:
		r = await _download.request_completed
	_download.queue_free()
	_download = null
	if r[0] != HTTPRequest.RESULT_SUCCESS or r[1] != 200:
		return _failed("the download failed (%s)" % (("HTTP %d" % r[1]) if r[0] == HTTPRequest.RESULT_SUCCESS else "no connection"))
	progress.emit("Installing MFTR %s…" % version)
	await get_tree().process_frame  # show it: unpacking takes a moment
	await get_tree().process_frame
	var err: String = MftrRelease.install(archive, info["size"], info["sha256"], install_dir)
	if err != "":
		return _failed(err)
	progress.emit("Starting MFTR %s…" % version)
	print("MFTR: installed %s, restarting" % version)
	var exe := install_dir.path_join("mftr.exe")
	var all := PackedStringArray(["--"]) + args if not args.is_empty() else PackedStringArray()
	if OS.create_process(exe, all) < 0:
		return _failed("MFTR %s is installed but didn't start: start it again" % version)
	get_tree().quit()
	return ""


func _failed(why: String) -> String:
	print("MFTR: update failed: %s" % why)
	progress.emit("Couldn't update: %s." % why)
	return why


func _process(_delta: float) -> void:
	if _download != null and _download.get_http_client_status() == HTTPClient.STATUS_BODY:
		var mb := 1024.0 * 1024.0
		var total := maxi(_download.get_body_size(), int(info.get("size", 0)))
		progress.emit("Downloading MFTR %s… %.0f / %.0f MB" % [info["version"], _download.get_downloaded_bytes() / mb, total / mb])


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
