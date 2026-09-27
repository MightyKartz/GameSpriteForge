extends Node2D

# One video-derived lateral movement candidate; up/down remain still references.
const MOVE_SCENE = preload("res://addons/forge_assets/courier_run_v3/forge_animated_sprite.tscn")
const MOVE_PIVOT := Vector2(200, 360)
const MOVE_SCALE := 0.7
const FRAME_DURATIONS_MS := [42, 41, 42, 42, 41, 42, 42, 41, 42, 42, 41, 42, 42, 41, 42, 42, 41, 42, 42, 41, 42, 42, 41, 42, 42, 41, 42, 42, 41]
const SEGMENTS := [{"direction":"right","ticks":135},{"direction":"left","ticks":99},{"direction":"right","ticks":99},{"direction":"up","ticks":60},{"direction":"down","ticks":60},{"direction":"left","ticks":135},{"direction":"right","ticks":99},{"direction":"left","ticks":99}]
const TEXTURES := {
	"right": preload("res://addons/forge_assets/courier_views/items/right.png"),
	"up": preload("res://addons/forge_assets/courier_views/items/up.png"),
	"down": preload("res://addons/forge_assets/courier_views/items/down.png"),
}
const PIVOTS := {
	"right": Vector2(132, 483.5),
	"up": Vector2(123.5, 481.5),
	"down": Vector2(114, 483.5),
}
const DIRECTIONS := ["right", "up", "left", "down"]
const VECTORS := [Vector2.RIGHT, Vector2.UP, Vector2.LEFT, Vector2.DOWN]
const SPEED := 140.0
const VERTICAL_SPEED := 45.0
const TEST_TICKS := 786
const DISPLAY_SCALE := 0.5

var actor := CharacterBody2D.new()
var visual := Node2D.new()
var sprite := Sprite2D.new()
var collider := CollisionShape2D.new()
var move_player: Node2D
var move_sprite: AnimatedSprite2D
var segment_index := 0
var segment_ticks := 0
var cycles_seen := 0
var previous_phase := 0.0
var frames_seen: Dictionary = {}
var timing_valid := false
var slow := false
var status := Label.new()
var facing := "right"
var testing := false
var demo := false
var ticks := 0
var report_path := "user://direction-report.json"
var checks: Array[Dictionary] = []
var failures: Array[String] = []
var max_velocity_error := 0.0
var visited: Dictionary = {}

func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg == "--test":
			testing = true
			demo = true
		elif arg == "--slow":
			slow = true
		elif arg.begins_with("--report="):
			report_path = arg.trim_prefix("--report=")
	actor.motion_mode = CharacterBody2D.MOTION_MODE_FLOATING
	actor.position = Vector2(500, 625)
	add_child(actor)
	actor.add_child(visual)
	visual.add_child(sprite)
	move_player = MOVE_SCENE.instantiate()
	visual.add_child(move_player)
	move_player.set_process(false)
	move_sprite = move_player.get_node("AnimatedSprite2D")
	move_player.play("move_right")
	validate_timing()
	sprite.centered = false
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	var shape := CircleShape2D.new()
	shape.radius = 16
	collider.shape = shape
	actor.add_child(collider)
	set_facing("right")
	label_at("FOREST COURIER", Vector2(60, 36), 32, Color("dce7d4"))
	label_at("One video-derived lateral cycle / mirrored left-right movement", Vector2(60, 83), 19)
	label_at("REVIEW PROTOTYPE — UP / DOWN USE STILL REFERENCES", Vector2(60, 123), 16, Color("f0be72"))
	label_at("Arrow keys / WASD: move     Space: automatic turn test", Vector2(60, 158), 16)
	label_at("29 frames / 1.208 s cycle (source speed) / Space: demo / Shift: half speed", Vector2(60, 190), 15)
	label_at("Source motion and timing retained. Processed result pending visual review.", Vector2(60, 217), 15)
	for index in range(3):
		var direction: String = ["right", "up", "down"][index]
		var thumb := Sprite2D.new()
		thumb.texture = TEXTURES[direction]
		thumb.centered = false
		thumb.scale = Vector2.ONE * 0.3
		thumb.position = Vector2(815 + index * 125, 258) - PIVOTS[direction] * 0.3
		thumb.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
		add_child(thumb)
		label_at(direction.to_upper(), Vector2(790 + index * 125, 270), 13)
	status.position = Vector2(76, 325)
	status.add_theme_font_size_override("font_size", 17)
	add_child(status)
	queue_redraw()

func label_at(text: String, point: Vector2, size: int, color := Color("aab8bf")) -> void:
	var label := Label.new()
	label.text = text
	label.position = point
	label.add_theme_font_size_override("font_size", size)
	label.modulate = color
	add_child(label)

func validate_timing() -> void:
	var frames := move_sprite.sprite_frames
	timing_valid = frames.has_animation("move_right") and frames.get_frame_count("move_right") == FRAME_DURATIONS_MS.size()
	if timing_valid:
		for i in range(FRAME_DURATIONS_MS.size()):
			var milliseconds := frames.get_frame_duration("move_right", i) / frames.get_animation_speed("move_right") * 1000.0
			timing_valid = timing_valid and absf(milliseconds - FRAME_DURATIONS_MS[i]) < 0.001
	if not timing_valid:
		failures.append("installed SpriteFrames timing mismatch")

func set_facing(direction: String) -> void:
	var before := actor.global_position
	var collision_before := collider.global_transform
	var frame_before := move_sprite.frame
	var progress_before := move_sprite.frame_progress
	facing = direction
	var lateral := direction == "left" or direction == "right"
	var source := "right" if direction == "left" else direction
	sprite.texture = TEXTURES[source]
	sprite.position = -PIVOTS[source]
	sprite.visible = not lateral
	move_player.visible = lateral
	var scale_value := MOVE_SCALE if lateral else DISPLAY_SCALE
	visual.scale = Vector2(-scale_value if direction == "left" else scale_value, scale_value)
	# Changing facing only mirrors the visual; it never restarts the animation clock.
	var anchor_world := move_sprite.to_global(MOVE_PIVOT - Vector2(200, 200)) if lateral else sprite.to_global(PIVOTS[source])
	var okay := anchor_world.distance_to(actor.global_position) < 0.001
	okay = okay and actor.global_position == before and collider.global_transform == collision_before
	var phase_okay := move_sprite.frame == frame_before and move_sprite.frame_progress == progress_before
	checks.append({"direction": direction, "source": "move_right" if lateral else source, "mirrored": direction == "left", "anchor_and_collision_stable": okay, "phase_preserved": phase_okay, "frame_before": frame_before, "frame_after": move_sprite.frame})
	if not okay or not phase_okay:
		failures.append("turn registration or phase failed: " + direction)
	visited[direction] = true

func _unhandled_key_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_SPACE:
		demo = not demo
		ticks = 0
		segment_index = 0
		segment_ticks = 0

func _physics_process(delta: float) -> void:
	var direction := Vector2.ZERO
	if demo:
		var selected: String = SEGMENTS[segment_index].direction
		direction = VECTORS[DIRECTIONS.find(selected)]
		if facing != selected:
			set_facing(selected)
	else:
		direction.x = float(Input.is_physical_key_pressed(KEY_D) or Input.is_action_pressed("ui_right")) - float(Input.is_physical_key_pressed(KEY_A) or Input.is_action_pressed("ui_left"))
		direction.y = float(Input.is_physical_key_pressed(KEY_S) or Input.is_action_pressed("ui_down")) - float(Input.is_physical_key_pressed(KEY_W) or Input.is_action_pressed("ui_up"))
		if direction != Vector2.ZERO:
			var next := "right" if direction.x > 0 else "left"
			if absf(direction.y) > absf(direction.x):
				next = "down" if direction.y > 0 else "up"
			if next != facing:
				set_facing(next)
		direction = direction.normalized()
	var before := actor.position
	var rate := 0.5 if slow or Input.is_physical_key_pressed(KEY_SHIFT) else 1.0
	actor.velocity = Vector2(direction.x * SPEED, direction.y * VERTICAL_SPEED) * rate
	if direction != Vector2.ZERO and move_player.visible:
		move_player.advance(delta * rate)
		frames_seen[move_sprite.frame] = true
		var phase: float = move_player.position_seconds
		if phase < previous_phase:
			cycles_seen += 1
		previous_phase = phase
	actor.move_and_slide()
	if testing and delta > 0:
		max_velocity_error = maxf(max_velocity_error, ((actor.position - before) / delta - actor.velocity).length())
	if not testing:
		actor.position = actor.position.clamp(Vector2(180, 580), Vector2(960, 655))
	status.text = "Facing: %s   %s   %s   frame %02d / 29" % [facing.to_upper(), "AUTO" if demo else "MANUAL", "MOVE LOOP" if move_player.visible else "STILL", move_sprite.frame + 1]
	ticks += 1
	if demo:
		segment_ticks += 1
		if segment_ticks >= SEGMENTS[segment_index].ticks:
			segment_ticks = 0
			segment_index = (segment_index + 1) % SEGMENTS.size()
	queue_redraw()
	if testing and ticks == TEST_TICKS:
		finish_test()

func finish_test() -> void:
	if visited.size() != 4:
		failures.append("not all four directions visited")
	if max_velocity_error > 0.1:
		failures.append("measured velocity differs from requested velocity")
	if cycles_seen < 2 or frames_seen.size() != FRAME_DURATIONS_MS.size():
		failures.append("insufficient complete cycles or missing rendered frames")
	var output := FileAccess.open(report_path, FileAccess.WRITE)
	if output == null:
		push_error("Cannot write test report: " + report_path)
		get_tree().quit(1)
		return
	output.store_string(JSON.stringify({
		"scope": "video-derived lateral movement review; up/down are static references",
		"godot_version": Engine.get_version_info().string,
		"ticks": ticks, "directions": visited.keys(), "turn_checks": checks,
		"max_velocity_error_px_per_second": max_velocity_error,
		"speed_px_per_second": SPEED, "vertical_speed_px_per_second": VERTICAL_SPEED, "visual_scale": MOVE_SCALE,
		"timing_valid": timing_valid, "frame_count": FRAME_DURATIONS_MS.size(), "frame_durations_ms": FRAME_DURATIONS_MS, "cycle_ms": 1208, "cycles_seen": cycles_seen, "unique_frames_seen": frames_seen.size(), "slow": slow,
		"collision_radius": 16, "failures": failures, "passed": failures.is_empty(),
		"video_generated": true, "run_animation_tested": true, "up_down_animation_tested": false, "quality_verdict": "prototype_usable", "visual_approval": "pending",
	}, "\t"))
	output.close()
	get_tree().quit(0 if failures.is_empty() else 1)

func _draw() -> void:
	draw_style_box(panel_style(), Rect2(60, 310, 1032, 370))
	for x in range(84, 1080, 48):
		for y in range(376, 670, 48):
			draw_circle(Vector2(x, y), 1, Color("2b3d43"))
	if is_instance_valid(actor) and actor.is_inside_tree():
		var point := actor.position
		draw_arc(point, 16, 0, TAU, 48, Color("70c7ba"), 1.0)
		draw_line(point - Vector2(9, 0), point + Vector2(9, 0), Color("f4c578"), 1.0)
		draw_line(point - Vector2(0, 9), point + Vector2(0, 9), Color("f4c578"), 1.0)

func panel_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color("111c21")
	style.set_corner_radius_all(18)
	return style
