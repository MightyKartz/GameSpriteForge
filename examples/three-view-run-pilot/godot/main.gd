extends Node2D

# Direction preflight only. These are still references, not a generated run loop.
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
const SPEED := 60.0
const DISPLAY_SCALE := 0.5

var actor := CharacterBody2D.new()
var visual := Node2D.new()
var sprite := Sprite2D.new()
var collider := CollisionShape2D.new()
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
		elif arg.begins_with("--report="):
			report_path = arg.trim_prefix("--report=")
	actor.motion_mode = CharacterBody2D.MOTION_MODE_FLOATING
	actor.position = Vector2(480, 650)
	add_child(actor)
	actor.add_child(visual)
	visual.add_child(sprite)
	sprite.centered = false
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	var shape := CircleShape2D.new()
	shape.radius = 16
	collider.shape = shape
	actor.add_child(collider)
	set_facing("right")
	label_at("FOREST COURIER", Vector2(60, 36), 32, Color("dce7d4"))
	label_at("Three views from one image  /  direction preflight", Vector2(60, 83), 19)
	label_at("STILL REFERENCES — RUN VIDEO PENDING", Vector2(60, 123), 16, Color("f0be72"))
	label_at("Arrow keys / WASD: move     Space: automatic turn test", Vector2(60, 158), 16)
	label_at("Left uses the right reference mirrored about the foot pivot.", Vector2(60, 190), 15)
	label_at("This verifies turning and placement, not gait or animation quality.", Vector2(60, 217), 15)
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

func set_facing(direction: String) -> void:
	var before := actor.global_position
	var collision_before := collider.global_transform
	facing = direction
	var source := "right" if direction == "left" else direction
	sprite.texture = TEXTURES[source]
	sprite.position = -PIVOTS[source]
	visual.scale = Vector2(-DISPLAY_SCALE if direction == "left" else DISPLAY_SCALE, DISPLAY_SCALE)
	# Mirror the visual parent, so the foot origin stays fixed and the collider is not mirrored.
	var anchor_world := sprite.to_global(PIVOTS[source])
	var okay := anchor_world.distance_to(actor.global_position) < 0.001
	okay = okay and actor.global_position == before and collider.global_transform == collision_before
	okay = okay and sprite.texture == TEXTURES[source]
	checks.append({"direction": direction, "source": source, "mirrored": direction == "left", "anchor_and_collision_stable": okay})
	if not okay:
		failures.append("turn registration failed: " + direction)
	visited[direction] = true

func _unhandled_key_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_SPACE:
		demo = not demo
		ticks = 0

func _physics_process(delta: float) -> void:
	var direction := Vector2.ZERO
	if demo:
		var index := int(ticks / 60) % 4
		direction = VECTORS[index]
		if facing != DIRECTIONS[index]:
			set_facing(DIRECTIONS[index])
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
	actor.velocity = direction * SPEED
	actor.move_and_slide()
	if testing and delta > 0:
		max_velocity_error = maxf(max_velocity_error, ((actor.position - before) / delta - actor.velocity).length())
	if not testing:
		actor.position = actor.position.clamp(Vector2(110, 590), Vector2(1040, 660))
	status.text = "Facing: %s     %s     source: static reference" % [facing.to_upper(), "AUTO" if demo else "MANUAL"]
	ticks += 1
	queue_redraw()
	if testing and ticks == 480:
		finish_test()

func finish_test() -> void:
	if visited.size() != 4:
		failures.append("not all four directions visited")
	if max_velocity_error > 0.1:
		failures.append("measured velocity differs from requested velocity")
	var output := FileAccess.open(report_path, FileAccess.WRITE)
	if output == null:
		push_error("Cannot write test report: " + report_path)
		get_tree().quit(1)
		return
	output.store_string(JSON.stringify({
		"scope": "static reference direction preflight; no run animation",
		"godot_version": Engine.get_version_info().string,
		"ticks": ticks, "directions": visited.keys(), "turn_checks": checks,
		"max_velocity_error_px_per_second": max_velocity_error,
		"speed_px_per_second": SPEED, "visual_scale": DISPLAY_SCALE,
		"collision_radius": 16, "failures": failures, "passed": failures.is_empty(),
		"video_generated": false, "run_animation_tested": false,
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
