extends SceneTree

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 1:
		_fail("Expected a .gsfpack directory path.")

	var pack_path := String(args[0]).simplify_path()
	var import_root := "res://imported/GodotSmokeWalk"
	var texture_res_path := import_root.path_join("sprite_sheet.png")
	var sprite_frames_res_path := import_root.path_join("GodotSmokeWalk.spriteframes.tres")
	var scene_res_path := "res://GodotSmokeWalkSmoke.tscn"

	_prepare_import_dir(import_root)

	var helper := _read_json(pack_path.path_join("assets/godot_import.json"))
	var sprite_frames_spec: Dictionary = _required_dict(helper, "spriteFrames", "assets/godot_import.json")
	var manifest := _read_json(pack_path.path_join(String(sprite_frames_spec["manifest"])))
	var atlas := _read_json(pack_path.path_join(String(sprite_frames_spec["atlas"])))
	var textures: Array = _required_array(sprite_frames_spec, "textures", "spriteFrames")
	var animations: Array = _required_array(sprite_frames_spec, "animations", "spriteFrames")
	var atlas_frames: Array = _required_array(atlas, "frames", "assets/atlas.json")

	if textures.size() != 1:
		_fail("This smoke expects a single sprite sheet texture; got %s." % textures.size())
	if animations.is_empty():
		_fail("Expected at least one animation in godot_import.json.")

	var source_texture := pack_path.path_join(String(textures[0]))
	if FileAccess.file_exists(texture_res_path):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(texture_res_path))
	var copy_error := DirAccess.copy_absolute(source_texture, ProjectSettings.globalize_path(texture_res_path))
	if copy_error != OK:
		_fail("Failed to copy sprite sheet into project: %s" % copy_error)

	var image := Image.new()
	var image_error := image.load(texture_res_path)
	if image_error != OK:
		_fail("Godot could not load imported project texture: %s" % image_error)
	var base_texture := ImageTexture.create_from_image(image)
	if base_texture == null or base_texture.get_width() <= 0 or base_texture.get_height() <= 0:
		_fail("ImageTexture creation failed.")

	var native_frames := SpriteFrames.new()
	for existing in native_frames.get_animation_names():
		native_frames.remove_animation(existing)

	for animation in animations:
		if typeof(animation) != TYPE_DICTIONARY:
			_fail("Animation entry must be a dictionary.")
		var animation_name := String(animation.get("name", "default"))
		var animation_frames: Array = _required_array(animation, "frames", "animation")
		native_frames.add_animation(animation_name)
		native_frames.set_animation_speed(animation_name, float(animation.get("fps", 12.0)))
		native_frames.set_animation_loop(animation_name, bool(animation.get("loop", true)))
		for frame_index_value in animation_frames:
			var frame_index := int(frame_index_value)
			if frame_index < 0 or frame_index >= atlas_frames.size():
				_fail("Animation frame index %s is outside atlas frame range." % frame_index)
			var atlas_frame: Dictionary = atlas_frames[frame_index]
			var atlas_texture := AtlasTexture.new()
			atlas_texture.atlas = base_texture
			atlas_texture.region = Rect2(
				float(atlas_frame["x"]),
				float(atlas_frame["y"]),
				float(atlas_frame["width"]),
				float(atlas_frame["height"])
			)
			native_frames.add_frame(animation_name, atlas_texture)

	var first_animation: Dictionary = animations[0]
	var first_animation_name := String(first_animation.get("name", "default"))
	if native_frames.get_frame_count(first_animation_name) != atlas_frames.size():
		_fail("SpriteFrames frame count does not match atlas frame count.")

	var save_frames_error := ResourceSaver.save(native_frames, sprite_frames_res_path)
	if save_frames_error != OK:
		_fail("Failed to save SpriteFrames resource: %s" % save_frames_error)

	var root := Node2D.new()
	root.name = "ForgeImportSmoke"
	var animated_sprite := AnimatedSprite2D.new()
	animated_sprite.name = "GodotSmokeWalk"
	animated_sprite.sprite_frames = native_frames
	animated_sprite.animation = first_animation_name
	animated_sprite.play()
	root.add_child(animated_sprite)
	animated_sprite.owner = root

	if !animated_sprite.is_playing():
		_fail("AnimatedSprite2D did not enter playing state.")
	if animated_sprite.sprite_frames.get_frame_count(first_animation_name) != atlas_frames.size():
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
	var loaded_sprite := instance.get_node("GodotSmokeWalk") as AnimatedSprite2D
	if loaded_sprite == null:
		_fail("Loaded scene is missing AnimatedSprite2D.")
	if loaded_sprite.sprite_frames == null:
		_fail("Loaded AnimatedSprite2D has no SpriteFrames.")
	if loaded_sprite.sprite_frames.get_frame_count(first_animation_name) != atlas_frames.size():
		_fail("Loaded SpriteFrames frame count mismatch.")
	loaded_sprite.play(first_animation_name)
	if !loaded_sprite.is_playing():
		_fail("Loaded AnimatedSprite2D could not play the imported animation.")

	instance.free()
	root.free()

	print("PASS Forge Godot import smoke: imported %s frames into %s and saved %s" % [
		atlas_frames.size(),
		sprite_frames_res_path,
		scene_res_path,
	])
	quit(0)

func _prepare_import_dir(path: String) -> void:
	var absolute := ProjectSettings.globalize_path(path)
	DirAccess.make_dir_recursive_absolute(absolute)

func _read_json(path: String) -> Dictionary:
	if !FileAccess.file_exists(path):
		_fail("Missing JSON file: %s" % path)
	var text := FileAccess.get_file_as_string(path)
	var parsed = JSON.parse_string(text)
	if typeof(parsed) != TYPE_DICTIONARY:
		_fail("Expected JSON object in %s" % path)
	return parsed

func _required_dict(source: Dictionary, key: String, context: String) -> Dictionary:
	if !source.has(key) or typeof(source[key]) != TYPE_DICTIONARY:
		_fail("Expected %s.%s to be an object." % [context, key])
	return source[key]

func _required_array(source: Dictionary, key: String, context: String) -> Array:
	if !source.has(key) or typeof(source[key]) != TYPE_ARRAY:
		_fail("Expected %s.%s to be an array." % [context, key])
	return source[key]

func _fail(message: String) -> void:
	push_error(message)
	print("FAIL Forge Godot import smoke: %s" % message)
	quit(1)
