extends Node2D
## Forge's common player for a layered scene or a child AnimatedSprite2D.
## Times are seconds. Tracks contain absolute transforms in source pixels.
signal completed(clip_name: String)

var playing := false
var finished := false
var speed := 1.0
var position_seconds := 0.0
var current_clip := ""
var _manifest: Dictionary = {}
var _sprite: AnimatedSprite2D
var _loaded := false

func _ready() -> void:
	_initialize_player()
	_apply_pose()

func _initialize_player() -> void:
	if _loaded:
		return
	_loaded = true
	_sprite = get_node_or_null("AnimatedSprite2D") as AnimatedSprite2D
	if _sprite != null:
		_sprite.pause()
		current_clip = String(_sprite.animation)
	else:
		var path: String = get_script().resource_path.get_base_dir().path_join("manifest.json")
		var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
		if parsed is Dictionary:
			_manifest = parsed
		current_clip = String(_manifest.get("defaultClip", ""))
		if current_clip.is_empty() and not _manifest.get("clips", []).is_empty():
			current_clip = String(_manifest["clips"][0]["id"])

func clips() -> PackedStringArray:
	_initialize_player()
	if _sprite != null:
		return _sprite.sprite_frames.get_animation_names()
	var result := PackedStringArray()
	for clip in _manifest.get("clips", []):
		result.append(String(clip["id"]))
	return result

func play(clip_name: String = "", restart: bool = true) -> bool:
	_initialize_player()
	var selected := current_clip if clip_name.is_empty() else clip_name
	if selected.is_empty() and _sprite == null:
		selected = String(_manifest.get("defaultClip", ""))
		if selected.is_empty() and not clips().is_empty():
			selected = clips()[0]
	if selected.is_empty() or not clips().has(selected):
		return false
	if restart or finished or selected != current_clip:
		position_seconds = 0.0
	current_clip = selected
	finished = false
	playing = true
	_apply_pose()
	return true

func pause() -> void:
	playing = false
	if _sprite != null:
		_sprite.pause()

func set_speed(value: float) -> bool:
	if not is_finite(value) or value < 0.0:
		return false
	speed = value
	return true

func seek(seconds: float) -> bool:
	_initialize_player()
	if not is_finite(seconds) or seconds < 0.0:
		return false
	position_seconds = seconds
	_apply_pose()
	if finished:
		playing = false
	return true

func reset_pose() -> void:
	_initialize_player()
	playing = false
	finished = false
	position_seconds = 0.0
	if _sprite == null:
		current_clip = ""
	_apply_pose()

func duration_seconds() -> float:
	_initialize_player()
	if _sprite != null and _sprite.sprite_frames.has_animation(current_clip):
		var frames := _sprite.sprite_frames
		var fps := frames.get_animation_speed(current_clip)
		if fps <= 0.0:
			return 0.0
		var total := 0.0
		for index in frames.get_frame_count(current_clip):
			total += frames.get_frame_duration(current_clip, index) / fps
		return total
	return float(_clip().get("durationMs", 0)) / 1000.0

func state() -> Dictionary:
	_initialize_player()
	return {"clip": current_clip, "playing": playing, "finished": finished,
		"positionSeconds": position_seconds, "durationSeconds": duration_seconds(), "speed": speed}

func _process(delta: float) -> void:
	advance(delta)

func advance(delta_seconds: float) -> void:
	if not playing or not is_finite(delta_seconds) or delta_seconds < 0.0:
		return
	var was_finished := finished
	var next_position := position_seconds + delta_seconds * speed
	if not is_finite(next_position):
		return
	position_seconds = next_position
	_apply_pose()
	if finished:
		playing = false
		if not was_finished:
			completed.emit(current_clip)

func _clip() -> Dictionary:
	for clip in _manifest.get("clips", []):
		if String(clip["id"]) == current_clip:
			return clip
	return {}

func _apply_pose() -> void:
	var duration := duration_seconds()
	var looping := false
	if _sprite != null and _sprite.sprite_frames.has_animation(current_clip):
		looping = _sprite.sprite_frames.get_animation_loop(current_clip)
	else:
		looping = bool(_clip().get("loop", false))
	# A nanosecond tolerance absorbs duration-weight summation roundoff without
	# treating a visibly earlier frame as complete (is_equal_approx is too broad).
	finished = duration > 0.0 and not looping and position_seconds >= duration - 0.000000001
	if duration > 0.0:
		if looping:
			position_seconds = fposmod(position_seconds, duration)
			if duration - position_seconds < 0.000000001:
				position_seconds = 0.0
		elif finished:
			position_seconds = duration
	if _sprite != null:
		_apply_frames()
		return
	var transforms := {}
	for layer in _manifest.get("layers", []):
		transforms[String(layer["id"])] = layer["transform"]
	for track in _clip().get("tracks", []):
		transforms[String(track["layerId"])] = _sample_track(track["keyframes"], position_seconds * 1000.0)
	for layer in _manifest.get("layers", []):
		var node := get_node_or_null("Layers/" + String(layer["id"])) as Node2D
		if node == null:
			continue
		var transform: Dictionary = transforms[String(layer["id"])]
		node.position = _vector(layer["pivot"]) + _vector(transform["position"])
		node.rotation = deg_to_rad(float(transform["rotationDegrees"]))
		node.scale = _vector(transform["scale"])
		node.modulate.a = float(transform["opacity"])

func _apply_frames() -> void:
	var frames := _sprite.sprite_frames
	if not frames.has_animation(current_clip) or frames.get_animation_speed(current_clip) <= 0.0:
		return
	_sprite.pause()
	_sprite.animation = current_clip
	var remaining := position_seconds
	var count := frames.get_frame_count(current_clip)
	for index in count:
		var duration := frames.get_frame_duration(current_clip, index) / frames.get_animation_speed(current_clip)
		if duration <= 0.0:
			return
		if remaining + 0.000000001 < duration or index == count - 1:
			_sprite.set_frame_and_progress(index, clampf(remaining / duration, 0.0, 1.0))
			return
		remaining -= duration

func _sample_track(keys: Array, time_ms: float) -> Dictionary:
	if time_ms <= float(keys[0]["timeMs"]):
		return keys[0]["transform"]
	for index in range(1, keys.size()):
		if time_ms <= float(keys[index]["timeMs"]):
			var a: Dictionary = keys[index - 1]
			var b: Dictionary = keys[index]
			var weight := (time_ms - float(a["timeMs"])) / (float(b["timeMs"]) - float(a["timeMs"]))
			var start: Dictionary = a["transform"]
			var end: Dictionary = b["transform"]
			var position := _vector(start["position"]).lerp(_vector(end["position"]), weight)
			var scale_value := _vector(start["scale"]).lerp(_vector(end["scale"]), weight)
			return {"position": [position.x, position.y], "scale": [scale_value.x, scale_value.y],
				"rotationDegrees": lerpf(float(start["rotationDegrees"]), float(end["rotationDegrees"]), weight),
				"opacity": lerpf(float(start["opacity"]), float(end["opacity"]), weight)}
	return keys[-1]["transform"]

func _vector(value: Array) -> Vector2:
	return Vector2(float(value[0]), float(value[1]))
