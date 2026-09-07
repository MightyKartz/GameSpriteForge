extends SceneTree

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 2:
		fail("Expected SpriteFrames resource and source manifest paths")
		return
	var frames := load(args[0]) as SpriteFrames
	var manifest = JSON.parse_string(FileAccess.get_file_as_string(args[1]))
	if frames == null or not manifest is Dictionary:
		fail("Could not load installed frames or source manifest")
		return
	var checked := 0
	for animation in manifest["animations"]:
		var name := String(animation["name"])
		var durations: Array = animation.get("frameDurationsMs", [])
		if durations.is_empty():
			for unused in animation["frames"]:
				durations.append(1000.0 / float(animation["fps"]))
		if not frames.has_animation(name) or frames.get_frame_count(name) != durations.size():
			fail("Animation coverage mismatch: " + name)
			return
		if frames.get_animation_loop(name) != bool(animation["loop"]):
			fail("Animation loop mismatch: " + name)
			return
		for index in range(durations.size()):
			var texture := frames.get_frame_texture(name, index)
			var actual := frames.get_frame_duration(name, index) * 1000.0 / frames.get_animation_speed(name)
			if texture == null or texture.get_width() <= 0 or absf(actual - float(durations[index])) > 0.01:
				fail("Texture or native duration mismatch: %s[%s]" % [name, index])
				return
			checked += 1
	if checked == 0:
		fail("No frames checked")
		return
	print("PASS installed Godot SpriteFrames: %s frames with native timing" % checked)
	quit(0)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
