extends SceneTree

var failures: Array[String] = []

func check(condition: bool, label: String) -> void:
	if not condition:
		failures.append(label)
		printerr("FAIL:" + label)

func same_visible_pixels(actual: Image, source: Image) -> bool:
	if actual == null or source == null or actual.get_size() != source.get_size():
		return false
	actual.convert(Image.FORMAT_RGBA8)
	source.convert(Image.FORMAT_RGBA8)
	var a := actual.get_data()
	var b := source.get_data()
	for i in range(0, a.size(), 4):
		if a[i + 3] != b[i + 3]:
			return false
		# Godot may fill hidden RGB under alpha=0 during import.
		if b[i + 3] != 0 and (a[i] != b[i] or a[i + 1] != b[i + 1] or a[i + 2] != b[i + 2]):
			return false
	return true

func _initialize() -> void:
	call_deferred("verify")

func verify() -> void:
	var contract = JSON.parse_string(FileAccess.get_file_as_string("res://acceptance.json"))
	var count := 0
	for character in contract.characters:
		var scene = load(character.scene)
		check(scene is PackedScene, "scene:" + character.id)
		if not scene is PackedScene:
			continue
		var player = scene.instantiate()
		root.add_child(player)
		player.set_process(false)
		var sprite = player.get_node("AnimatedSprite2D")
		var frames: SpriteFrames = sprite.sprite_frames
		check(frames.get_animation_names().size() == character.actions.size(), "action-count:" + character.id)
		check(not sprite.centered, "centering:" + character.id)
		check(sprite.position == -Vector2(character.anchor[0], character.anchor[1]), "anchor:" + character.id)
		check(sprite.texture_filter == CanvasItem.TEXTURE_FILTER_NEAREST, "sampling:" + character.id)
		for action in character.actions:
			check(frames.has_animation(action.name), "action:" + action.name)
			if not frames.has_animation(action.name):
				continue
			check(frames.get_frame_count(action.name) == action.frames.size(), "frame-count:" + action.name)
			if frames.get_frame_count(action.name) != action.frames.size():
				continue
			check(frames.get_animation_loop(action.name) == action.loop, "loop:" + action.name)
			var elapsed := 0.0
			check(player.play(action.name), "play:" + action.name)
			for i in range(action.frames.size()):
				var texture = frames.get_frame_texture(action.name, i)
				var ms := frames.get_frame_duration(action.name, i) / frames.get_animation_speed(action.name) * 1000.0
				check(absf(ms - action.durationsMs[i]) < 0.001, "duration:%s:%s:%s" % [character.id, action.name, i])
				check(texture != null and same_visible_pixels(texture.get_image(), Image.load_from_file(action.frames[i])), "pixels:%s:%s:%s" % [character.id, action.name, i])
				player.seek((elapsed + action.durationsMs[i] / 2.0) / 1000.0)
				check(sprite.frame == i, "seek:" + action.name)
				elapsed += action.durationsMs[i]
			player.play(action.name)
			player.advance(elapsed / 1000.0 + 0.00001)
			check(player.state().finished == (not action.loop), "completion:" + action.name)
			check(player.state().playing == action.loop, "playing:" + action.name)
		player.free()
		count += 1
	for item in contract.static:
		var texture = load(item.texture)
		check(texture is Texture2D, "static-load:" + item.id)
		if texture is Texture2D:
			check(same_visible_pixels(texture.get_image(), Image.load_from_file(item.source)), "static-pixels:" + item.id)
		count += 1
	for item in contract.audio:
		var stream = load(item.stream)
		check(stream is AudioStreamWAV, "audio-load:" + item.id)
		if stream is AudioStreamWAV:
			check(stream.format == AudioStreamWAV.FORMAT_16_BITS, "audio-pcm:" + item.id)
			check(stream.mix_rate == item.sampleRate and stream.stereo == (item.channels == 2), "audio-format:" + item.id)
			check(stream.data.size() == item.frames * item.channels * 2, "audio-frames:" + item.id)
			check(stream.loop_mode == (AudioStreamWAV.LOOP_FORWARD if item.loop else AudioStreamWAV.LOOP_DISABLED), "audio-loop:" + item.id)
		count += 1
	if failures.is_empty():
		print("RESOURCE_TASK_PASS:" + str(count))
		quit(0)
	else:
		quit(1)
