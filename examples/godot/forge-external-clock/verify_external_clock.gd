extends SceneTree

const Clock = preload("res://forge_sprite_clock.gd")

func _initialize() -> void:
	var frames := SpriteFrames.new()
	frames.remove_animation("default")
	frames.add_animation("burst")
	frames.set_animation_speed("burst", 10.0)
	frames.set_animation_loop("burst", false)
	var texture := GradientTexture2D.new()
	for duration in [0.8, 2.4, 1.25]:
		frames.add_frame("burst", texture, duration)
	var sprite := AnimatedSprite2D.new()
	sprite.sprite_frames = frames
	var checks := [
		[-1.0, 0, false], [0.079, 0, false], [0.081, 1, false],
		[0.319, 1, false], [0.321, 2, false], [0.444, 2, false], [0.446, 2, true], [10.0, 2, true]
	]
	for check in checks:
		var result := Clock.apply_at(sprite, "burst", check[0])
		if result.has("error") or result["frame"] != check[1] or result["finished"] != check[2]:
			fail("nonuniform/nonloop check failed at %s: %s" % [check[0], result], sprite)
			return
		if sprite.frame != check[1] or sprite.is_playing():
			fail("AnimatedSprite2D did not adopt external clock", sprite)
			return
	var paused := Clock.apply_at(sprite, "burst", 0.12)
	var paused_again := Clock.apply_at(sprite, "burst", 0.12)
	if paused != paused_again or not is_equal_approx(sprite.frame_progress, 1.0 / 6.0):
		fail("paused clock drifted", sprite)
		return
	frames.set_animation_loop("burst", true)
	var looped := Clock.apply_at(sprite, "burst", 0.455)
	if looped.has("error") or looped["frame"] != 0 or looped["finished"]:
		fail("loop wrap failed", sprite)
		return
	if not Clock.sample(frames, "missing", 0.0).has("error"):
		fail("missing animation accepted", sprite)
		return
	sprite.free()
	print("PASS forge external clock: nonuniform timing, pause, nonloop completion, loop wrap, native node state")
	quit(0)

func fail(message: String, sprite: AnimatedSprite2D) -> void:
	sprite.free()
	push_error(message)
	quit(1)
