extends Control
## A presentation scene using actual Forge-matted PNGs; it is not a generated map.
const INK := Color("29382f")
const MUTED := Color("6f786d")
const PAPER := Color("f5f2e9")
const ACCENT := Color("b15a35")
const ITEMS := ["potion", "crystal", "key", "pouch", "crate", "barrel", "signpost", "campfire"]
const NAMES := ["Healing potion", "Mana crystal", "Brass key", "Coin pouch", "Supply crate", "Travel barrel", "Trail sign", "Campfire"]
var font := SystemFont.new()

func _ready() -> void:
	font.font_names = PackedStringArray(["Avenir Next", "Arial"])
	var args := OS.get_cmdline_user_args()
	var mode := String(args[0]) if args.size() > 0 else "gallery"
	box(Rect2(0, 0, 1280, 760), PAPER)
	match mode:
		"gallery": gallery()
		"processing": processing()
		"godot": engine_preview()
		_: push_error("Unknown showcase mode"); get_tree().quit(1); return
	await get_tree().process_frame
	await get_tree().process_frame
	await RenderingServer.frame_post_draw
	var destination := ProjectSettings.globalize_path("res://%s.png" % mode)
	var capture := get_viewport().get_texture().get_image()
	if capture == null or capture.save_png(destination) != OK:
		push_error("Could not save showcase capture")
		get_tree().quit(1)
		return
	print("SHOWCASE_CAPTURE " + JSON.stringify({"mode": mode, "path": destination, "width": capture.get_width(), "height": capture.get_height()}))
	get_tree().quit(0)

func box(rect: Rect2, color: Color, radius: int = 0) -> void:
	var panel := Panel.new()
	panel.position = rect.position
	panel.size = rect.size
	var style := StyleBoxFlat.new()
	style.bg_color = color
	style.set_corner_radius_all(radius)
	panel.add_theme_stylebox_override("panel", style)
	add_child(panel)

func label_at(text: String, rect: Rect2, size_px: int, color: Color = INK, centered: bool = false) -> void:
	var node := Label.new()
	node.text = text
	node.position = rect.position
	node.size = rect.size
	node.add_theme_font_override("font", font)
	node.add_theme_font_size_override("font_size", size_px)
	node.add_theme_color_override("font_color", color)
	if centered: node.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	add_child(node)

func sprite(path: String, rect: Rect2) -> void:
	var image := TextureRect.new()
	image.texture = load(path) as Texture2D
	assert(image.texture != null, "Missing demo texture: " + path)
	image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	image.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	image.position = rect.position
	image.size = rect.size
	add_child(image)

func header(kicker: String, title: String, subtitle: String) -> void:
	label_at(kicker, Rect2(52, 28, 1100, 28), 16, ACCENT)
	label_at(title, Rect2(48, 60, 1180, 64), 43)
	label_at(subtitle, Rect2(52, 132, 1160, 32), 20, MUTED)

func gallery() -> void:
	header("FORGE / DEMO ART", "A matching kit for your next world.", "Four inventory icons. Four scene props. One forest-inspired style.")
	for i in range(8):
		var x := 48 + (i % 4) * 302
		var y := 192 + (i / 4) * 257
		box(Rect2(x, y, 278, 235), Color("e9e7dc"), 12)
		sprite("res://sprites/%s.png" % ITEMS[i], Rect2(x + 51, y + 3, 176, 195))
		label_at(NAMES[i], Rect2(x, y + 191, 278, 37), 18, INK, true)
	label_at("Illustrative source artwork · Sprites prepared with Forge", Rect2(52, 714, 1160, 26), 15, MUTED)

func checker(rect: Rect2) -> void:
	box(rect, Color("e5e6de"))
	for y in range(int(rect.size.y / 20)):
		for x in range(int(rect.size.x / 20)):
			if (x + y) % 2 == 0:
				box(Rect2(rect.position + Vector2(x * 20, y * 20), Vector2(20, 20)), Color("f2f2ea"))

func processing() -> void:
	header("FORGE / LOCAL SPRITE PREPARATION", "Keep the artwork. Remove the background.", "The same source sprite, before and after Forge's chroma-key processing.")
	box(Rect2(48, 198, 544, 458), Color("eee8e1"), 12)
	checker(Rect2(688, 198, 540, 460))
	sprite("res://source/potion-before.png", Rect2(145, 207, 344, 432))
	sprite("res://sprites/potion.png", Rect2(785, 207, 344, 432))
	label_at("→", Rect2(592, 375, 96, 60), 40, ACCENT, true)
	label_at("SOURCE ARTWORK", Rect2(48, 671, 544, 30), 17, MUTED, true)
	label_at("TRANSPARENT PNG", Rect2(688, 671, 540, 30), 17, MUTED, true)

func engine_preview() -> void:
	header("FORGE / GODOT PREVIEW", "See your sprites in context.", "A small demo scene assembled in Godot with Forge-prepared PNGs.")
	box(Rect2(48, 190, 1184, 512), Color("283c32"), 16)
	box(Rect2(74, 214, 830, 460), Color("788e69"), 12)
	# Authored scene layout, kept separate from asset generation and export.
	for row in range(9):
		for col in range(17):
			if (row * 7 + col * 3) % 11 == 0:
				box(Rect2(86 + col * 47, 227 + row * 48, 14, 3), Color("91a079"), 1)
	box(Rect2(74, 494, 830, 75), Color("aeac89"))
	for i in range(16):
		box(Rect2(90 + i * 50, 512 + (i % 2) * 18, 36, 18), Color("c4bea0"), 5)
	sprite("res://sprites/crate.png", Rect2(113, 311, 158, 210))
	sprite("res://sprites/barrel.png", Rect2(235, 278, 145, 215))
	sprite("res://sprites/campfire.png", Rect2(394, 317, 198, 240))
	sprite("res://sprites/signpost.png", Rect2(687, 278, 164, 226))
	sprite("res://sprites/crate.png", Rect2(644, 477, 124, 168))
	sprite("res://sprites/barrel.png", Rect2(755, 469, 113, 168))
	label_at("FOREST OUTPOST", Rect2(100, 232, 700, 30), 17, Color("253c2d"))
	label_at("INVENTORY", Rect2(936, 226, 240, 30), 17, Color("d7dfce"))
	for i in range(4):
		var x := 932 + (i % 2) * 132
		var y := 280 + (i / 2) * 160
		box(Rect2(x, y, 116, 138), Color("3e5342"), 8)
		sprite("res://sprites/%s.png" % ITEMS[i], Rect2(x + 8, y + 4, 100, 124))
	label_at("Godot 4.6 · Demo scene", Rect2(935, 620, 260, 32), 16, Color("c9d6bf"))
