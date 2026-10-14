extends SceneTree
## Live queue/lobby/augment/wave smoke test against a local server offering Horde:
## godot --headless --path client --script res://tests/horde_test.gd -- 127.0.0.1:7777
var client: MatchClient
var fails := 0

func check(ok: bool, what: String) -> void:
	print(("PASS  " if ok else "FAIL  ") + what)
	if not ok:
		fails += 1

func _init() -> void:
	_run.call_deferred()

func wait_for(predicate: Callable, seconds: float) -> bool:
	var until := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < until:
		if predicate.call():
			return true
		await process_frame
	return false

func _run() -> void:
	var args := OS.get_cmdline_user_args()
	if args.is_empty():
		push_error("Pass the local server address after --")
		quit(1)
		return
	client = MatchClient.new()
	root.add_child(client)
	check(client.connect_to_server(args[0]), "connect to local server")
	if not await wait_for(func(): return client.phase() == "queues", 10.0):
		check(false, "server offers its queues")
		quit(1)
		return
	var queues: Array = client.queues()
	var horde := -1
	for i in queues.size():
		if queues[i].mode == "Mayhem: Horde":
			check(queues[i].coop, "Horde is co-op only")
			horde = i
	check(horde >= 0, "Horde appears in the normal queue list")
	if horde < 0:
		quit(1)
		return
	client.choose_queue(horde)
	if not await wait_for(func(): return client.phase() == "lobby", 10.0):
		check(false, "Horde champion select")
		quit(1)
		return
	var lobby: Dictionary = client.lobby_state()
	check(lobby.pick and lobby.slots.size() == 5, "five champion picks with bot-filled seats")
	check(lobby.slots.all(func(slot): return slot.team == "blue"), "all champions are on blue")
	client.lobby_take("Ember")
	check(await wait_for(func(): return client.lobby_state().slots.any(func(s): return s.you and s.champion == "Ember"), 3.0), "pick an existing champion")
	client.lobby_ready(true)
	if not await wait_for(func(): return client.phase() == "playing" and not client.horde_status().is_empty(), 10.0):
		check(false, "new Horde client receives match progress")
		quit(1)
		return
	var h: Dictionary = client.horde_status()
	check(client.game_type() == "Mayhem: Horde" and h.goal == 20, "welcome and 20-wave objective")
	check(h.base_max_health == 4000 and h.preparing, "blue Base and preparation countdown")
	check(await wait_for(func(): return client.own_status().get("offer", []).size() == 3, 3.0), "first Mayhem augment draft opens before wave 1")
	client.reroll_augment(0)
	client.pick_augment(0)
	check(await wait_for(func(): return client.own_status().get("augments", []).size() == 1, 3.0), "reroll and keep an existing augment")
	check(await wait_for(func(): return client.scoreboard().size() == 5, 3.0), "scoreboard has exactly five champions")
	check(await wait_for(func(): return client.horde_status().get("wave", 0) == 1 and client.horde_status().get("enemies", 0) > 0, 35.0), "wave 1 spawns red enemies and reaches the client")
	client.disconnect_from_server()
	print("FAILS: %d" % fails)
	quit(1 if fails > 0 else 0)
