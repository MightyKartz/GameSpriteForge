extends Node2D

var textures: Dictionary = {}
var font := SystemFont.new()
var mode := 0
var anchors := false
var args := OS.get_cmdline_user_args()
var placed := [
	{"id":"lantern", "pos":Vector2(330,350), "scale":0.65},
	{"id":"chest", "pos":Vector2(450,445), "scale":0.48},
	{"id":"lantern", "pos":Vector2(810,485), "scale":0.65},
	{"id":"chest", "pos":Vector2(735,620), "scale":0.48},
]
var dragging := -1

func _ready() -> void:
	font.font_names = PackedStringArray(["PingFang SC", "Arial"])
	for pack in ["props", "background"]:
		var usage_path := "res://addons/forge_assets/forest_%s/forge_usage.json" % pack
		var usage: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(usage_path))
		for id in usage.texturePaths:
			textures[id] = load(usage.texturePaths[id])
			assert(textures[id] is Texture2D)
	if "--inspection" in args: mode = 1
	queue_redraw()
	if "--capture" in args:
		await get_tree().process_frame
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		var image := get_viewport().get_texture().get_image()
		var path := "res://inspection.png" if mode == 1 else "res://scene.png"
		assert(image.save_png(ProjectSettings.globalize_path(path)) == OK)
		print("TRIAL_CAPTURE " + path)
		get_tree().quit()

func text_at(s: String, p: Vector2, size_px := 20, color := Color.WHITE) -> void:
	draw_string(font, p, s, HORIZONTAL_ALIGNMENT_LEFT, -1, size_px, color)

func _draw() -> void:
	if mode == 0:
		draw_texture_rect(textures.forest, Rect2(0,0,1152,768), false)
		for item in placed:
			var p: Vector2 = item.pos
			var scale_value: float = item.scale
			draw_set_transform(p, 0, Vector2(scale_value,scale_value))
			draw_texture(textures[item.id], Vector2(-128,-240))
			if anchors:
				draw_line(Vector2(-10,0),Vector2(10,0),Color.CYAN,2)
				draw_line(Vector2(0,-10),Vector2(0,10),Color.CYAN,2)
			draw_set_transform(Vector2.ZERO)
	else:
		draw_rect(Rect2(0,0,1152,768),Color("19251f"))
		text_at("浅色背景",Vector2(90,122),24)
		text_at("深色背景",Vector2(640,122),24)
		for row in range(2):
			var id := "chest" if row == 0 else "lantern"
			var y := 145+row*280
			for col in range(2):
				var x := 40+col*550
				draw_rect(Rect2(x,y,510,245),Color("ecece7") if col == 0 else Color("09110c"))
				for index in range(3):
					var canvas_size: float = [96.0,144.0,224.0][index]
					var tx: float = x+[8.0,112.0,264.0][index]
					draw_texture_rect(textures[id],Rect2(tx,y+230-canvas_size,canvas_size,canvas_size),false)
			text_at("宝箱 / 96、144、224 像素画布" if row == 0 else "石灯 / 96、144、224 像素画布",Vector2(72,y+266),18)
		text_at("检查轮廓、边缘与小尺寸可读性；原图与加工参数均已保留。",Vector2(72,746),20)
	draw_rect(Rect2(0,0,1152,70),Color(0.035,0.075,0.055,0.95))
	text_at("Forge · 森林资产试作",Vector2(30,32),25)
	text_at("1 场景  /  2 边缘检查  /  A 锚点  /  鼠标拖动物件",Vector2(30,58),17,Color("bbd0bc"))

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed:
		if event.keycode == KEY_1: mode = 0
		if event.keycode == KEY_2: mode = 1
		if event.keycode == KEY_A: anchors = not anchors
		queue_redraw()
	if mode != 0: return
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		if event.pressed:
			for index in range(placed.size()-1,-1,-1):
				var item: Dictionary = placed[index]
				var p: Vector2 = item.pos
				var s: float = item.scale
				if Rect2(p-Vector2(128,240)*s,Vector2(256,256)*s).has_point(event.position):
					dragging = index
					break
		else: dragging = -1
	if event is InputEventMouseMotion and dragging >= 0:
		placed[dragging].pos = event.position
		queue_redraw()
