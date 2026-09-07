extends SceneTree

const ANIMATION := &"walk_down"
const FRAME_COUNT := 4
const FRAME_SIZE := Vector2i(256, 256)
const FRAMES_RESOURCE_PATH := "res://output/walk_down.spriteframes.tres"
const SCENE_RESOURCE_PATH := "res://output/walk_down_smoke.tscn"
const REPORT_PATH := "res://output/godot-smoke-report.json"
const CONTACT_SHEET_PATH := "res://output/walk_down-runtime-contact-sheet.png"
const PLAYBACK_STRIP_PATH := "res://output/walk_down-runtime-strip.png"


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 6:
		_fail("Expected source Job ID, FPS, and four frame SHA-256 values.")
		return

	var source_job_id := String(args[0])
	var playback_fps := float(args[1])
	var expected_sha256s: Array[String] = []
	for index in range(FRAME_COUNT):
		expected_sha256s.push_back(String(args[index + 2]))
	if playback_fps <= 0.0:
		_fail("Playback FPS must be positive.")
		return

	var textures: Array[Texture2D] = []
	var images: Array[Image] = []
	var frame_reports: Array[Dictionary] = []
	var baseline_bottoms: Array[int] = []
	for index in range(FRAME_COUNT):
		var frame_path := "res://frames/frame-%02d.png" % index
		var actual_sha256 := FileAccess.get_sha256(frame_path)
		if actual_sha256 != expected_sha256s[index]:
			_fail("Frame %d SHA-256 mismatch." % index)
			return
		var texture := ResourceLoader.load(frame_path) as Texture2D
		if texture == null:
			_fail("Godot could not import external texture %s." % frame_path)
			return
		if texture.resource_path != frame_path:
			_fail("Frame %d did not remain an external texture resource." % index)
			return
		if Vector2i(texture.get_width(), texture.get_height()) != FRAME_SIZE:
			_fail("Frame %d is not 256x256." % index)
			return
		var image := texture.get_image()
		if image == null or image.is_empty():
			_fail("Godot could not read imported texture pixels for frame %d." % index)
			return
		var alpha_bbox := _alpha_bbox(image)
		if alpha_bbox.size.x <= 0 or alpha_bbox.size.y <= 0:
			_fail("Frame %d has no visible pixels." % index)
			return
		if !_has_transparent_border(image):
			_fail("Frame %d touches the 256x256 canvas boundary." % index)
			return
		var baseline_bottom := alpha_bbox.position.y + alpha_bbox.size.y - 1
		baseline_bottoms.push_back(baseline_bottom)
		textures.push_back(texture)
		images.push_back(image)
		frame_reports.push_back({
			"frameIndex": index,
			"resourcePath": frame_path,
			"resourceClass": texture.get_class(),
			"sha256": actual_sha256,
			"width": image.get_width(),
			"height": image.get_height(),
			"alphaBBox": [
				alpha_bbox.position.x,
				alpha_bbox.position.y,
				alpha_bbox.size.x,
				alpha_bbox.size.y,
			],
			"baselineBottom": baseline_bottom,
			"transparentBorder": true,
		})

	var baseline_min: int = int(baseline_bottoms.min())
	var baseline_max: int = int(baseline_bottoms.max())
	var baseline_drift: int = baseline_max - baseline_min
	if baseline_drift > 3:
		_fail("Runtime baseline drift exceeds 3 pixels: %d." % baseline_drift)
		return

	var native_frames := SpriteFrames.new()
	for existing_animation in native_frames.get_animation_names():
		native_frames.remove_animation(existing_animation)
	native_frames.add_animation(ANIMATION)
	native_frames.set_animation_speed(ANIMATION, playback_fps)
	native_frames.set_animation_loop(ANIMATION, true)
	for texture in textures:
		native_frames.add_frame(ANIMATION, texture, 1.0)
	if native_frames.get_frame_count(ANIMATION) != FRAME_COUNT:
		_fail("SpriteFrames did not retain four ordered frames.")
		return
	if ResourceSaver.save(native_frames, FRAMES_RESOURCE_PATH) != OK:
		_fail("Failed to save external-texture SpriteFrames resource.")
		return

	var reloaded_frames := ResourceLoader.load(
		FRAMES_RESOURCE_PATH,
		"SpriteFrames",
		ResourceLoader.CACHE_MODE_REPLACE
	) as SpriteFrames
	if reloaded_frames == null:
		_fail("Saved SpriteFrames could not be loaded.")
		return
	if reloaded_frames.get_frame_count(ANIMATION) != FRAME_COUNT:
		_fail("Reloaded SpriteFrames frame count changed.")
		return
	if !reloaded_frames.get_animation_loop(ANIMATION):
		_fail("Reloaded walk_down animation is not looping.")
		return
	if absf(reloaded_frames.get_animation_speed(ANIMATION) - playback_fps) > 0.001:
		_fail("Reloaded walk_down animation FPS changed.")
		return
	for index in range(FRAME_COUNT):
		var reloaded_texture := reloaded_frames.get_frame_texture(ANIMATION, index)
		if reloaded_texture == null or reloaded_texture.resource_path != "res://frames/frame-%02d.png" % index:
			_fail("SpriteFrames frame %d is not bound to its ordered external PNG." % index)
			return

	var scene_root := Node2D.new()
	scene_root.name = "ForgeV95WalkDownSmoke"
	var sprite := AnimatedSprite2D.new()
	sprite.name = "WalkDown"
	sprite.sprite_frames = reloaded_frames
	sprite.animation = ANIMATION
	sprite.centered = true
	sprite.position = Vector2(320.0, 320.0)
	sprite.scale = Vector2(2.0, 2.0)
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	scene_root.add_child(sprite)
	sprite.owner = scene_root
	sprite.play(ANIMATION)

	var packed_scene := PackedScene.new()
	if packed_scene.pack(scene_root) != OK:
		_fail("Failed to pack the AnimatedSprite2D smoke scene.")
		return
	if ResourceSaver.save(packed_scene, SCENE_RESOURCE_PATH) != OK:
		_fail("Failed to save the AnimatedSprite2D smoke scene.")
		return

	get_root().add_child(scene_root)
	var observed_frames: Array[int] = [sprite.frame]
	var playback_started_ms := Time.get_ticks_msec()
	var playback_duration_ms := int(ceil(1250.0 / playback_fps * 4.0))
	while Time.get_ticks_msec() - playback_started_ms < playback_duration_ms:
		await create_timer(0.02).timeout
		if observed_frames.back() != sprite.frame:
			observed_frames.push_back(sprite.frame)
	if !_contains_complete_loop(observed_frames):
		_fail("AnimatedSprite2D did not advance through 0,1,2,3,0: %s." % [observed_frames])
		return

	var loaded_scene := ResourceLoader.load(
		SCENE_RESOURCE_PATH,
		"PackedScene",
		ResourceLoader.CACHE_MODE_REPLACE
	) as PackedScene
	if loaded_scene == null:
		_fail("Saved smoke scene could not be loaded.")
		return
	var loaded_root := loaded_scene.instantiate()
	var loaded_sprite := loaded_root.get_node("WalkDown") as AnimatedSprite2D
	if loaded_sprite == null:
		_fail("Reloaded scene is missing AnimatedSprite2D.")
		return
	if loaded_sprite.sprite_frames.resource_path != FRAMES_RESOURCE_PATH:
		_fail("Reloaded scene does not reference the external SpriteFrames resource.")
		return
	if loaded_sprite.texture_filter != CanvasItem.TEXTURE_FILTER_NEAREST:
		_fail("Reloaded scene lost nearest-neighbor texture filtering.")
		return
	if loaded_sprite.scale != Vector2(2.0, 2.0) or !loaded_sprite.centered:
		_fail("Reloaded scene lost its integer scale or centered anchor.")
		return
	loaded_sprite.play(ANIMATION)
	if !loaded_sprite.is_playing():
		_fail("Reloaded AnimatedSprite2D could not play walk_down.")
		return

	if !_write_contact_sheet(images, CONTACT_SHEET_PATH):
		_fail("Failed to save the Godot-loaded contact sheet.")
		return
	if !_write_playback_strip(images, PLAYBACK_STRIP_PATH):
		_fail("Failed to save the Godot-loaded playback strip.")
		return

	var version := Engine.get_version_info()
	var report := {
		"schemaVersion": "1",
		"profile": "forge-godot-walk-smoke@1.0.0",
		"verdict": "pass",
		"sourceJobId": source_job_id,
		"godot": {
			"version": String(version.get("string", "unknown")),
			"headless": DisplayServer.get_name() == "headless",
		},
		"animation": {
			"name": String(ANIMATION),
			"fps": playback_fps,
			"loop": true,
			"frameCount": FRAME_COUNT,
			"orderedFrameIndices": [0, 1, 2, 3],
			"observedRuntimeFrames": observed_frames,
		},
		"runtime": {
			"nodeType": loaded_sprite.get_class(),
			"centered": loaded_sprite.centered,
			"position": [loaded_sprite.position.x, loaded_sprite.position.y],
			"scale": [loaded_sprite.scale.x, loaded_sprite.scale.y],
			"textureFilter": "nearest",
			"framesResourcePath": FRAMES_RESOURCE_PATH,
			"sceneResourcePath": SCENE_RESOURCE_PATH,
		},
		"alpha": {
			"allFramesHaveTransparentBorder": true,
			"baselineBottomMin": baseline_min,
			"baselineBottomMax": baseline_max,
			"baselineDriftPx": baseline_drift,
		},
		"frames": frame_reports,
		"outputs": {
			"contactSheet": CONTACT_SHEET_PATH,
			"playbackStrip": PLAYBACK_STRIP_PATH,
			"spriteFrames": FRAMES_RESOURCE_PATH,
			"scene": SCENE_RESOURCE_PATH,
		},
	}
	if !_write_json(REPORT_PATH, report):
		_fail("Failed to save the Godot smoke report.")
		return

	loaded_root.free()
	scene_root.queue_free()
	print("PASS Forge V9.5 Godot walk_down smoke: 4 external PNGs, 4 FPS loop, baseline drift %d px" % baseline_drift)
	quit(0)


func _alpha_bbox(image: Image) -> Rect2i:
	var min_x := image.get_width()
	var min_y := image.get_height()
	var max_x := -1
	var max_y := -1
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			if image.get_pixel(x, y).a > 0.0:
				min_x = mini(min_x, x)
				min_y = mini(min_y, y)
				max_x = maxi(max_x, x)
				max_y = maxi(max_y, y)
	if max_x < min_x or max_y < min_y:
		return Rect2i()
	return Rect2i(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)


func _has_transparent_border(image: Image) -> bool:
	var last_x := image.get_width() - 1
	var last_y := image.get_height() - 1
	for x in range(image.get_width()):
		if image.get_pixel(x, 0).a > 0.0 or image.get_pixel(x, last_y).a > 0.0:
			return false
	for y in range(image.get_height()):
		if image.get_pixel(0, y).a > 0.0 or image.get_pixel(last_x, y).a > 0.0:
			return false
	return true


func _contains_complete_loop(observed: Array[int]) -> bool:
	var expected := [0, 1, 2, 3, 0]
	if observed.size() < expected.size():
		return false
	for start in range(observed.size() - expected.size() + 1):
		var matches := true
		for offset in range(expected.size()):
			if observed[start + offset] != expected[offset]:
				matches = false
				break
		if matches:
			return true
	return false


func _write_contact_sheet(images: Array[Image], path: String) -> bool:
	var sheet := Image.create(FRAME_SIZE.x * 2, FRAME_SIZE.y * 2, false, Image.FORMAT_RGBA8)
	for y in range(0, sheet.get_height(), 16):
		for x in range(0, sheet.get_width(), 16):
			var even := int(x / 16) + int(y / 16)
			var color := Color("34435e") if even % 2 == 0 else Color("536583")
			sheet.fill_rect(Rect2i(x, y, 16, 16), color)
	for index in range(FRAME_COUNT):
		var target := Vector2i((index % 2) * FRAME_SIZE.x, (index / 2) * FRAME_SIZE.y)
		sheet.blend_rect(images[index], Rect2i(Vector2i.ZERO, FRAME_SIZE), target)
	return sheet.save_png(path) == OK


func _write_playback_strip(images: Array[Image], path: String) -> bool:
	var sequence := [0, 1, 2, 3, 0]
	var strip := Image.create(FRAME_SIZE.x * sequence.size(), FRAME_SIZE.y, false, Image.FORMAT_RGBA8)
	strip.fill(Color(0.055, 0.063, 0.082, 1.0))
	for output_index in range(sequence.size()):
		strip.blend_rect(
			images[sequence[output_index]],
			Rect2i(Vector2i.ZERO, FRAME_SIZE),
			Vector2i(output_index * FRAME_SIZE.x, 0)
		)
	return strip.save_png(path) == OK


func _write_json(path: String, value: Dictionary) -> bool:
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(value, "\t") + "\n")
	file.close()
	return true


func _fail(message: String) -> void:
	printerr("FAIL Forge V9.5 Godot walk_down smoke: %s" % message)
	quit(1)
