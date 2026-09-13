extends Control
## Authored presentation only. Prop scenes and AudioStreamWAV are installed by Forge.

const INK := Color("edf1df")
const MUTED := Color("92a698")
const GOLD := Color("e3bc71")
const NAMES := ["Healing potion", "Mana crystal", "Brass key", "Supply crate", "Travel barrel", "Campfire"]
const IDS := ["potion", "crystal", "key", "crate", "barrel", "campfire"]
var font := SystemFont.new()
var player := AudioStreamPlayer.new()
var timer := 0.0
var plays := 0
var waveform: Array = []
var status: Label
var cursor: Panel
var wave_bars: Array[Panel] = []
var cue_lights: Array[Panel] = []

func _ready() -> void:
	font.font_names = PackedStringArray(["Avenir Next", "Arial"])
	var inputs: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://showcase-inputs.json"))
	waveform = inputs.waveform
	box(Rect2(0, 0, 1200, 700), Color("101d18"))
	box(Rect2(0, 0, 1200, 5), GOLD)
	label_at("FORGE    /    LOCAL ASSET DELIVERY", Rect2(40, 26, 900, 27), 15, GOLD)
	label_at("From source files to scene.", Rect2(36, 64, 1100, 65), 42, INK)
	label_at("Transparent PNGs + a synthetic WAV, prepared as Packs and loaded in Godot.", Rect2(40, 132, 1100, 32), 18, MUTED)
	box(Rect2(40, 188, 766, 398), Color("1b2d24"), 16)
	label_at("01   NATIVE PROP SCENES", Rect2(62, 202, 670, 25), 14, GOLD)
	for i in range(6):
		var x: float = 168 + (i % 3) * 248
		var y: float = 367 + int(i / 3) * 166
		box(Rect2(x - 102, y - 5, 204, 1), Color("3b4c38"))
		var packed := load("res://addons/forge_assets/forest_props/scenes/%s.tscn" % IDS[i]) as PackedScene
		assert(packed != null)
		var prop := packed.instantiate() as Node2D
		assert(prop != null and prop.get_node("Sprite2D") is Sprite2D)
		add_child(prop)
		prop.position = Vector2(x, y)
		prop.scale = Vector2(0.54, 0.54)
		label_at(NAMES[i], Rect2(x - 105, y + 8, 210, 29), 16, INK, true)
	box(Rect2(826, 188, 334, 398), Color("21372d"), 16)
	label_at("02   NATIVE AUDIO", Rect2(848, 202, 288, 25), 14, GOLD)
	label_at("A little chime.", Rect2(848, 245, 286, 44), 28, INK)
	label_at("SYNTHETIC · THREE NOTES", Rect2(850, 292, 286, 25), 12, MUTED)
	box(Rect2(850, 336, 286, 100), Color("15261e"), 8)
	for i in range(waveform.size()):
		var height: float = max(3.0, float(waveform[i]) * 155.0)
		var bar := box(Rect2(858 + i * 3.35, 386 - height / 2, 2, height), Color("78947c"), 1)
		wave_bars.append(bar)
	cursor = box(Rect2(858, 346, 2, 80), GOLD, 1)
	status = label_at("Ready to play", Rect2(850, 456, 290, 30), 19, INK)
	label_at("48 kHz / stereo / PCM16 source delivery", Rect2(850, 492, 285, 25), 12, MUTED)
	for i in range(3):
		cue_lights.append(box(Rect2(852 + i * 19, 544, 7, 7), Color("536a57"), 4))
	label_at("PNG → static Pack → PackedScene", Rect2(42, 607, 720, 28), 16, INK)
	label_at("WAV → audio Pack → AudioStreamWAV", Rect2(42, 638, 720, 28), 16, INK)
	label_at("Godot 4.6  ·  authored asset demo", Rect2(792, 610, 365, 25), 13, MUTED)
	label_at("Synthetic audio  ·  no human listening review", Rect2(792, 639, 365, 25), 12, MUTED)
	var usage: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://addons/forge_assets/synthetic_audio/forge_usage.json"))
	player.stream = load(usage.audioPaths.chime) as AudioStreamWAV
	assert(player.stream != null and player.stream is AudioStreamWAV)
	add_child(player)
	print("NATIVE_SHOWCASE_READY " + JSON.stringify({"props": 6, "audioClass": player.stream.get_class(), "audioPath": usage.audioPaths.chime}))

func _process(delta: float) -> void:
	timer += delta
	if plays < 3 and timer >= 0.65 + plays * 2.2:
		player.play()
		plays += 1
		print("NATIVE_CHIME_PLAY %d" % plays)
	var active := player.playing
	status.text = "Playing chime  %02d / 03" % plays if active else "Chime tail / next cue" if plays < 3 else "Three cues delivered"
	var progress := clampf(player.get_playback_position() / 1.8, 0.0, 1.0) if active else 1.0
	cursor.position.x = 858 + progress * 268
	cursor.visible = active
	for i in range(wave_bars.size()):
		wave_bars[i].modulate = GOLD if active and float(i) / wave_bars.size() <= progress else Color.WHITE
	for i in range(cue_lights.size()):
		cue_lights[i].modulate = Color(2.1, 1.8, 1.2) if i < plays else Color.WHITE

func box(rect: Rect2, color: Color, radius: int = 0) -> Panel:
	var panel := Panel.new()
	panel.position = rect.position
	panel.size = rect.size
	var style := StyleBoxFlat.new()
	style.bg_color = color
	style.set_corner_radius_all(radius)
	panel.add_theme_stylebox_override("panel", style)
	add_child(panel)
	return panel

func label_at(text: String, rect: Rect2, size_px: int, color: Color, centered: bool = false) -> Label:
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
	return node
