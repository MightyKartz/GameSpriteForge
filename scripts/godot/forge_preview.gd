extends Node2D
## Isolated preview UI. The exported asset owns the common playback implementation.
var player: Node2D
var timeline: HSlider
var status: Label
var _updating := false

func _ready() -> void:
	var config: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://preview.json"))
	var scene := load(String(config["scene"])) as PackedScene
	if scene == null:
		push_error("Preview asset scene could not be loaded")
		get_tree().quit(1)
		return
	player = scene.instantiate() as Node2D
	add_child(player)
	player.call("pause")
	var canvas = config.get("canvas")
	if canvas is Dictionary:
		var fit := minf(940.0 / float(canvas["width"]), 670.0 / float(canvas["height"]))
		player.scale = Vector2.ONE * fit
		player.position = Vector2((1000.0 - float(canvas["width"]) * fit) / 2.0, 25.0)
	else:
		player.position = Vector2(500.0, 500.0)
	var overlay := CanvasLayer.new()
	add_child(overlay)
	var panel := VBoxContainer.new()
	panel.position = Vector2(24, 710)
	panel.size = Vector2(952, 90)
	overlay.add_child(panel)
	var row := HBoxContainer.new()
	panel.add_child(row)
	var select := OptionButton.new()
	var names: PackedStringArray = player.call("clips")
	for name in names:
		select.add_item(name)
	select.item_selected.connect(func(index): player.call("play", names[index]); player.call("pause"); _update_timeline())
	row.add_child(select)
	for action in ["Play", "Pause", "Reset pose"]:
		var button := Button.new()
		button.text = action
		button.pressed.connect(func():
			if action == "Play":
				player.call("play", select.get_item_text(select.selected) if select.item_count > 0 else "", false)
			elif action == "Pause":
				player.call("pause")
			else:
				player.call("reset_pose")
			_update_timeline())
		row.add_child(button)
	var speed := OptionButton.new()
	for value in [0.25, 0.5, 1.0, 2.0, 4.0]:
		speed.add_item(str(value) + "x")
	speed.selected = 2
	speed.item_selected.connect(func(index): player.call("set_speed", [0.25, 0.5, 1.0, 2.0, 4.0][index]))
	row.add_child(speed)
	timeline = HSlider.new()
	timeline.step = 0.001
	timeline.value_changed.connect(func(value):
		if not _updating:
			player.call("seek", value))
	panel.add_child(timeline)
	status = Label.new()
	panel.add_child(status)
	_update_timeline()
	print("FORGE_PREVIEW_READY")
	if OS.get_cmdline_user_args().has("--forge-preview-verify"):
		get_tree().quit(0)

func _process(_delta: float) -> void:
	if status != null:
		_update_timeline()

func _update_timeline() -> void:
	var value: Dictionary = player.call("state")
	_updating = true
	timeline.max_value = maxf(float(value["durationSeconds"]), 0.001)
	timeline.value = float(value["positionSeconds"])
	status.text = "%s  %.3f / %.3f s  %s" % [value["clip"], value["positionSeconds"], value["durationSeconds"], "Finished" if value["finished"] else ("Playing" if value["playing"] else "Paused")]
	_updating = false
