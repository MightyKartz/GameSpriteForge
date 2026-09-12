extends SceneTree

var _failed := false

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 1:
		_fail("Expected a .gsfpack directory path.")
		return

	var pack_path := String(args[0]).simplify_path()
	var forgepack := _read_json(pack_path.path_join("forgepack.json"))
	if _failed:
		return
	var pack_name := String(forgepack.get("name", "Forge Export"))
	var resource_name := _safe_resource_name(pack_name)
	var import_root := "res://imported".path_join(resource_name)
	var sprite_frames_res_path := import_root.path_join("%s.spriteframes.tres" % resource_name)
	var scene_res_path := "res://%s.tscn" % resource_name

	_prepare_import_dir(import_root)

	var helper := _read_json(pack_path.path_join("assets/godot_import.json"))
	if _failed:
		return
	if helper.get("assetType", forgepack.get("assetType", "")) == "audio_set":
		_import_audio_set(pack_path, import_root, helper)
		return
	var sprite_frames_spec: Dictionary = _required_dict(helper, "spriteFrames", "assets/godot_import.json")
	var atlas := _read_json(pack_path.path_join(String(sprite_frames_spec["atlas"])))
	var textures: Array = _required_array(sprite_frames_spec, "textures", "spriteFrames")
	var animations: Array = _required_array(sprite_frames_spec, "animations", "spriteFrames")
	var atlas_frames: Array = _required_array(atlas, "frames", "assets/atlas.json")

	if textures.is_empty():
		_fail("Expected at least one sprite sheet texture in godot_import.json.")
	if animations.is_empty():
		_fail("Expected at least one animation in godot_import.json.")

	var texture_map := {}
	for texture_relative_value in textures:
		var texture_relative := String(texture_relative_value)
		var texture_name := texture_relative.get_file()
		if texture_name.is_empty():
			_fail("Texture path has no file name: %s" % texture_relative)
		var source_texture := pack_path.path_join(texture_relative)
		var texture_res_path := import_root.path_join(texture_name)
		if FileAccess.file_exists(texture_res_path):
			DirAccess.remove_absolute(ProjectSettings.globalize_path(texture_res_path))
		var copy_error := DirAccess.copy_absolute(source_texture, ProjectSettings.globalize_path(texture_res_path))
		if copy_error != OK:
			_fail("Failed to copy sprite sheet into project: %s" % copy_error)

		var image := Image.new()
		var image_error := image.load(texture_res_path)
		if image_error != OK:
			_fail("Godot could not load imported project texture: %s" % image_error)
		var image_texture := ImageTexture.create_from_image(image)
		if image_texture == null or image_texture.get_width() <= 0 or image_texture.get_height() <= 0:
			_fail("ImageTexture creation failed for %s." % texture_name)
		texture_map[texture_name] = image_texture

	var native_frames := SpriteFrames.new()
	for existing in native_frames.get_animation_names():
		native_frames.remove_animation(existing)

	for animation in animations:
		if typeof(animation) != TYPE_DICTIONARY:
			_fail("Animation entry must be a dictionary.")
		var animation_name := String(animation.get("name", "default"))
		var animation_frames: Array = _required_array(animation, "frames", "animation")
		var animation_fps := float(animation.get("fps", 12.0))
		var frame_durations: Array = animation.get("frameDurationsMs", [])
		if !frame_durations.is_empty() and frame_durations.size() != animation_frames.size():
			_fail("Animation frameDurationsMs must match frames: %s" % animation_name)
		native_frames.add_animation(animation_name)
		native_frames.set_animation_speed(animation_name, animation_fps)
		native_frames.set_animation_loop(animation_name, bool(animation.get("loop", true)))
		for animation_frame_index in animation_frames.size():
			var frame_index_value = animation_frames[animation_frame_index]
			var frame_index := int(frame_index_value)
			if frame_index < 0 or frame_index >= atlas_frames.size():
				_fail("Animation frame index %s is outside atlas frame range." % frame_index)
			var atlas_frame: Dictionary = atlas_frames[frame_index]
			var atlas_image := String(atlas_frame.get("image", atlas.get("image", "sprite_sheet.png")))
			var frame_texture = texture_map.get(atlas_image)
			if frame_texture == null:
				_fail("Atlas frame references missing texture: %s" % atlas_image)
			var atlas_texture := AtlasTexture.new()
			atlas_texture.atlas = frame_texture
			atlas_texture.region = Rect2(
				float(atlas_frame["x"]),
				float(atlas_frame["y"]),
				float(atlas_frame["width"]),
				float(atlas_frame["height"])
			)
			var relative_duration := 1.0
			if !frame_durations.is_empty():
				var duration_ms := float(frame_durations[animation_frame_index])
				if duration_ms <= 0.0:
					_fail("Animation frame duration must be positive: %s" % animation_name)
				relative_duration = duration_ms * animation_fps / 1000.0
			native_frames.add_frame(animation_name, atlas_texture, relative_duration)

	var first_animation: Dictionary = animations[0]
	var first_animation_name := String(first_animation.get("name", "default"))
	var expected_frame_count := _required_array(first_animation, "frames", "animation").size()
	if native_frames.get_frame_count(first_animation_name) != expected_frame_count:
		_fail("SpriteFrames frame count does not match the first animation frame count.")

	var save_frames_error := ResourceSaver.save(native_frames, sprite_frames_res_path)
	if save_frames_error != OK:
		_fail("Failed to save SpriteFrames resource: %s" % save_frames_error)

	var root := Node2D.new()
	root.name = "ForgeImportSmoke"
	var animated_sprite := AnimatedSprite2D.new()
	animated_sprite.name = resource_name
	animated_sprite.sprite_frames = native_frames
	animated_sprite.animation = first_animation_name
	animated_sprite.play()
	root.add_child(animated_sprite)
	animated_sprite.owner = root

	if !animated_sprite.is_playing():
		_fail("AnimatedSprite2D did not enter playing state.")
	if animated_sprite.sprite_frames.get_frame_count(first_animation_name) != expected_frame_count:
		_fail("AnimatedSprite2D SpriteFrames frame count mismatch.")

	var packed_scene := PackedScene.new()
	var pack_error := packed_scene.pack(root)
	if pack_error != OK:
		_fail("Failed to pack scene: %s" % pack_error)
	var save_scene_error := ResourceSaver.save(packed_scene, scene_res_path)
	if save_scene_error != OK:
		_fail("Failed to save smoke scene: %s" % save_scene_error)

	var loaded_scene := ResourceLoader.load(scene_res_path) as PackedScene
	if loaded_scene == null:
		_fail("Saved scene could not be loaded by ResourceLoader.")
	var instance: Node = loaded_scene.instantiate()
	var loaded_sprite := instance.get_node(resource_name) as AnimatedSprite2D
	if loaded_sprite == null:
		_fail("Loaded scene is missing AnimatedSprite2D.")
	if loaded_sprite.sprite_frames == null:
		_fail("Loaded AnimatedSprite2D has no SpriteFrames.")
	if loaded_sprite.sprite_frames.get_frame_count(first_animation_name) != expected_frame_count:
		_fail("Loaded SpriteFrames frame count mismatch.")
	loaded_sprite.play(first_animation_name)
	if !loaded_sprite.is_playing():
		_fail("Loaded AnimatedSprite2D could not play the imported animation.")

	instance.free()
	root.free()

	print("PASS Forge Godot import smoke: imported %s frames into %s and saved %s" % [
		expected_frame_count,
		sprite_frames_res_path,
		scene_res_path,
	])
	quit(0)

func _import_audio_set(pack_path: String, import_root: String, helper: Dictionary) -> void:
	# Audio smoke uses the same native-resource constructor and fresh-process
	# verifier as transactional CLI installation. It never starts playback.
	var installer: String = String(get_script().resource_path).get_base_dir().path_join("install_forge_pack.gd")
	if not FileAccess.file_exists(installer):
		_fail("Audio smoke requires install_forge_pack.gd beside this script.")
		return
	var items := _required_array(helper, "items", "audio helper")
	if _failed:
		return
	var sources := import_root.path_join("sources")
	if DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(sources)) != OK:
		_fail("Could not create audio smoke sources directory.")
		return
	var ids := {}
	for value in items:
		if typeof(value) != TYPE_DICTIONARY:
			_fail("Audio smoke item must be an object.")
			return
		var item: Dictionary = value
		var item_id := String(item.get("id", ""))
		if item_id.is_empty() or item_id == "." or item_id == ".." or not item_id.is_valid_filename() or ids.has(item_id) or item.get("path") != "assets/audio/" + item_id + ".wav":
			_fail("Audio smoke item must have a unique safe id and canonical WAV path.")
			return
		ids[item_id] = true
		var source := pack_path.path_join(String(item["path"]))
		var destination := ProjectSettings.globalize_path(sources.path_join(item_id + ".wav"))
		if DirAccess.copy_absolute(source, destination) != OK:
			_fail("Could not copy audio smoke source: %s" % item_id)
			return
	for phase in ["install", "verify"]:
		var command := PackedStringArray([
			"--headless", "--path", ProjectSettings.globalize_path("res://"),
			"--script", ProjectSettings.globalize_path(installer), "--", pack_path,
			import_root.trim_prefix("res://"),
		])
		if phase == "verify":
			command.append("--verify")
		var output: Array = []
		var status := OS.execute(OS.get_executable_path(), command, output, true)
		var completions := 0
		for text in output:
			for line in String(text).split("\n"):
				if line.contains("SCRIPT ERROR") or line.begins_with("ERROR:") or line.begins_with("FAIL "):
					_fail("Audio smoke native %s failed: %s" % [phase, line])
					return
				if line.begins_with("FORGE_INSTALL_RESULT "):
					var result: Variant = JSON.parse_string(line.trim_prefix("FORGE_INSTALL_RESULT "))
					if typeof(result) == TYPE_DICTIONARY and result.get("schemaVersion") == "1" and result.get("status") == "succeeded" and result.get("phase") == phase and result.get("assetType") == "audio_set" and result.get("target") == import_root:
						completions += 1
		if status != 0 or completions != 1:
			_fail("Audio smoke native %s did not return a successful completion result." % phase)
			return
	print("PASS Forge Godot import smoke: imported and verified %s native audio streams in %s" % [items.size(), import_root])
	quit(0)

func _prepare_import_dir(path: String) -> void:
	var absolute := ProjectSettings.globalize_path(path)
	DirAccess.make_dir_recursive_absolute(absolute)

func _read_json(path: String) -> Dictionary:
	if !FileAccess.file_exists(path):
		_fail("Missing JSON file: %s" % path)
		return {}
	var text := FileAccess.get_file_as_string(path)
	var parsed = JSON.parse_string(text)
	if typeof(parsed) != TYPE_DICTIONARY:
		_fail("Expected JSON object in %s" % path)
		return {}
	return parsed

func _required_dict(source: Dictionary, key: String, context: String) -> Dictionary:
	if !source.has(key) or typeof(source[key]) != TYPE_DICTIONARY:
		_fail("Expected %s.%s to be an object." % [context, key])
		return {}
	return source[key]

func _required_array(source: Dictionary, key: String, context: String) -> Array:
	if !source.has(key) or typeof(source[key]) != TYPE_ARRAY:
		_fail("Expected %s.%s to be an array." % [context, key])
		return []
	return source[key]

func _safe_resource_name(value: String) -> String:
	var result := ""
	for index in range(value.length()):
		var code := value.unicode_at(index)
		var character := value.substr(index, 1)
		var is_digit := code >= 48 and code <= 57
		var is_upper := code >= 65 and code <= 90
		var is_lower := code >= 97 and code <= 122
		if is_digit or is_upper or is_lower:
			result += character
		elif result.length() == 0 or !result.ends_with("_"):
			result += "_"
	result = result.strip_edges().trim_prefix("_").trim_suffix("_")
	if result.is_empty():
		return "ForgeExport"
	return result

func _fail(message: String) -> void:
	_failed = true
	push_error(message)
	print("FAIL Forge Godot import smoke: %s" % message)
	quit(1)
