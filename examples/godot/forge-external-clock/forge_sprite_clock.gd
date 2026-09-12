extends RefCounted
## Sample native SpriteFrames from an externally owned elapsed time in seconds.
## Keep elapsed_seconds fixed while paused. Game rules and time accumulation belong
## to the consumer; this sampler does not own a second animation clock.

static func sample(frames: SpriteFrames, animation: StringName, elapsed_seconds: float) -> Dictionary:
	if frames == null or not frames.has_animation(animation):
		return {"error": "missing_animation"}
	var count := frames.get_frame_count(animation)
	var fps := frames.get_animation_speed(animation)
	if count == 0 or fps <= 0.0 or not is_finite(elapsed_seconds):
		return {"error": "invalid_animation_timing"}
	var durations: Array[float] = []
	var total := 0.0
	for index in range(count):
		# Godot stores each duration as a multiple of 1 / animation FPS.
		var duration := frames.get_frame_duration(animation, index) / fps
		if duration <= 0.0 or not is_finite(duration):
			return {"error": "invalid_frame_duration"}
		durations.append(duration)
		total += duration
	var time := maxf(0.0, elapsed_seconds)
	var looping := frames.get_animation_loop(animation)
	if not looping and time >= total:
		return {"frame": count - 1, "progress": 1.0, "finished": true, "durationSeconds": total}
	if looping:
		time = fposmod(time, total)
	for index in range(count):
		if time < durations[index] or index == count - 1:
			return {"frame": index, "progress": clampf(time / durations[index], 0.0, 1.0),
				"finished": false, "durationSeconds": total}
		time -= durations[index]
	return {"error": "unreachable_animation_time"}

static func apply_at(sprite: AnimatedSprite2D, animation: StringName, elapsed_seconds: float) -> Dictionary:
	if sprite == null:
		return {"error": "missing_sprite"}
	var result := sample(sprite.sprite_frames, animation, elapsed_seconds)
	if result.has("error"):
		return result
	# Disable the node's internal clock so the external simulation clock is authoritative.
	sprite.pause()
	sprite.animation = animation
	sprite.set_frame_and_progress(int(result["frame"]), float(result["progress"]))
	return result
