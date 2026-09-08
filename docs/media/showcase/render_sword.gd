extends Control
## Replays existing Sword SpriteFrames in a separate presentation, not gameplay.
const FPS := 50.0
const SIZE := Vector2(1040, 440)
const GROUPS := {
	"spells": [["fire_fx", "FIRE", 1.4, 0.64], ["frost_fx", "FROST", 1.4, 0.24], ["lightning_fx", "LIGHTNING", 0.7, 0.08]],
	"enemies": [["ghost", "WISP", 0.0, 0.0], ["golem", "STONE GOLEM", 0.0, 0.0], ["vine", "VINE SPIRIT", 0.0, 0.0], ["boss_slam", "GUARDIAN", 2.0, 0.0]],
}
var font := SystemFont.new()
var actors: Array[Dictionary] = []

func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 2 or not GROUPS.has(args[0]):
		push_error("Expected spells|enemies and an output directory")
		get_tree().quit(1)
		return
	var mode: String = args[0]
	var destination: String = args[1]
	var entries: Array = GROUPS[mode]
	var manifest: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://inputs.json"))
	font.font_names = PackedStringArray(["Avenir Next", "Arial"])
	panel(Rect2(Vector2.ZERO, SIZE), Color("111f1b"))
	label_at("SWORD / " + mode.to_upper(), Rect2(30, 18, 900, 38), 26, Color("eee9d7"))
	label_at("Art prepared with Forge · Godot asset preview", Rect2(32, 61, 960, 26), 17, Color("b1bdac"))
	var width := (SIZE.x - 48.0) / entries.size()
	for i in range(entries.size()):
		var entry: Array = entries[i]
		var card := Rect2(24 + width * i, 104, width - 12, 280)
		panel(card, Color("1d3027"), 12)
		label_at(entry[1], Rect2(card.position.x, 343, card.size.x, 28), 16, Color("ded6b9"), true)
		var key: String = manifest[entry[0]]
		var usage: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://addons/forge_assets/%s/forge_usage.json" % key))
		var frames := load(usage.spriteFramesPath) as SpriteFrames
		assert(frames != null)
		var animation := StringName(usage.defaultAnimation)
		var anchor := Vector2(usage.anchor.x, usage.anchor.y)
		var bounds := Rect2()
		var durations: Array[float] = []
		for index in range(frames.get_frame_count(animation)):
			var texture := frames.get_frame_texture(animation, index)
			var used := Rect2(texture.get_image().get_used_rect())
			used.position -= anchor
			bounds = used if index == 0 else bounds.merge(used)
			durations.append(frames.get_frame_duration(animation, index) / frames.get_animation_speed(animation))
		# A single transform per animation retains all inter-frame coordinates.
		var scale_factor := minf((card.size.x - 36) / bounds.size.x, 211.0 / bounds.size.y)
		var sprite := AnimatedSprite2D.new()
		sprite.sprite_frames = frames
		sprite.animation = animation
		sprite.centered = false
		sprite.offset = -anchor
		sprite.scale = Vector2.ONE * scale_factor
		sprite.position = Vector2(card.get_center().x - bounds.get_center().x * scale_factor, 329 - bounds.end.y * scale_factor)
		sprite.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
		add_child(sprite)
		actors.append({"sprite": sprite, "durations": durations, "loop": frames.get_animation_loop(animation), "period": float(entry[2]), "phase": float(entry[3])})
	label_at("Prototype assets · Replayed from existing animation frames", Rect2(32, 401, 960, 24), 15, Color("a8b5a1"))
	await get_tree().process_frame
	await get_tree().process_frame
	var count := 210 if mode == "spells" else 200
	for index in range(count):
		for actor in actors:
			seek_actor(actor, index / FPS)
		await RenderingServer.frame_post_draw
		var capture := get_viewport().get_texture().get_image()
		if capture == null or capture.save_png(destination.path_join("%04d.png" % index)) != OK:
			push_error("Could not save showcase frame")
			get_tree().quit(1)
			return
		await get_tree().process_frame
	print("SWORD_CAPTURE " + JSON.stringify({"mode": mode, "frames": count, "fps": FPS, "width": SIZE.x, "height": SIZE.y}))
	get_tree().quit()

func seek_actor(actor: Dictionary, time: float) -> void:
	var total := 0.0
	for duration: float in actor.durations:
		total += duration
	var phase := fposmod(time + actor.phase, total if actor.loop else actor.period)
	var sprite: AnimatedSprite2D = actor.sprite
	# One-shot effects repeat only after a presentation pause. The guardian rests
	# on its last frame between attacks; spell effects disappear after completion.
	sprite.visible = actor.loop or phase < total or actor.period == 2.0
	var end := 0.0
	for index in range(actor.durations.size()):
		end += actor.durations[index]
		if phase + 0.000001 < end or index == actor.durations.size() - 1:
			sprite.frame = index
			return

func panel(rect: Rect2, color: Color, radius: int = 0) -> void:
	var node := Panel.new()
	node.position = rect.position
	node.size = rect.size
	var style := StyleBoxFlat.new()
	style.bg_color = color
	style.set_corner_radius_all(radius)
	node.add_theme_stylebox_override("panel", style)
	add_child(node)

func label_at(text: String, rect: Rect2, size_px: int, color: Color, centered: bool = false) -> void:
	var node := Label.new()
	node.text = text
	node.position = rect.position
	node.size = rect.size
	node.add_theme_font_override("font", font)
	node.add_theme_font_size_override("font_size", size_px)
	node.add_theme_color_override("font_color", color)
	if centered:
		node.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	add_child(node)
