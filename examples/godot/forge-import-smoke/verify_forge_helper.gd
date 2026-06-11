extends SceneTree

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 1:
		_fail("Expected a .gsfpack path argument.")

	var pack_path := String(args[0])
	var helper_path := pack_path.path_join("assets/godot_import.json")
	var helper := _read_json(helper_path)
	var sprite_frames: Dictionary = _required_dict(helper, "spriteFrames", helper_path)
	var textures: Array = _required_array(sprite_frames, "textures", helper_path)
	var animations: Array = _required_array(sprite_frames, "animations", helper_path)
	var frame_width := int(sprite_frames.get("frameWidth", 0))
	var frame_height := int(sprite_frames.get("frameHeight", 0))

	if textures.is_empty():
		_fail("spriteFrames.textures must list at least one texture.")
	if animations.is_empty():
		_fail("spriteFrames.animations must list at least one animation.")
	if frame_width <= 0 or frame_height <= 0:
		_fail("spriteFrames frame size must be positive.")

	var manifest_rel := String(sprite_frames.get("manifest", ""))
	var atlas_rel := String(sprite_frames.get("atlas", ""))
	var manifest := _read_json(pack_path.path_join(manifest_rel))
	var atlas := _read_json(pack_path.path_join(atlas_rel))
	var manifest_sheet: Dictionary = _required_dict(manifest, "sheet", manifest_rel)
	var manifest_images: Array = _required_array(manifest_sheet, "images", manifest_rel)
	var atlas_images: Array = _required_array(atlas, "images", atlas_rel)
	var atlas_frames: Array = _required_array(atlas, "frames", atlas_rel)

	if manifest_images.size() != textures.size():
		_fail("manifest.sheet.images and helper textures must have the same length.")
	if atlas_images.size() != textures.size():
		_fail("atlas.images and helper textures must have the same length.")
	if atlas_frames.size() < textures.size():
		_fail("atlas.frames must include frame entries for the helper textures.")

	for index in range(textures.size()):
		var texture_rel := String(textures[index])
		var texture_path := pack_path.path_join(texture_rel)
		if !FileAccess.file_exists(texture_path):
			_fail("Missing helper texture: %s" % texture_rel)
		var image := Image.new()
		var error := image.load(texture_path)
		if error != OK:
			_fail("Godot Image.load failed for %s with error %s" % [texture_rel, error])
		if image.get_width() < frame_width or image.get_height() < frame_height:
			_fail("Texture %s is smaller than the declared frame size." % texture_rel)

	var first_animation: Dictionary = animations[0]
	var frames: Array = _required_array(first_animation, "frames", "spriteFrames.animations[0]")
	if frames.size() != atlas_frames.size():
		_fail("Animation frame count must match atlas frame count.")

	print("PASS Forge Godot helper sample: %s textures, %s frames" % [textures.size(), frames.size()])
	quit(0)

func _read_json(path: String) -> Dictionary:
	if path.is_empty():
		_fail("Expected a JSON path.")
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
	print("FAIL Forge Godot helper sample: %s" % message)
	quit(1)
