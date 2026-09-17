extends SceneTree

func _initialize() -> void:
	call_deferred("_verify")

func _verify() -> void:
	var frames := 30
	var screenshot := ""
	var args := OS.get_cmdline_user_args()
	for index in range(args.size() - 1):
		if args[index] == "--forge-frames":
			frames = int(args[index + 1])
		if args[index] == "--forge-screenshot":
			screenshot = args[index + 1]
	var main_scene: String = ProjectSettings.get_setting("application/run/main_scene", "")
	if main_scene.is_empty() or change_scene_to_file(main_scene) != OK:
		push_error("Forge could not load the project's main scene")
		quit(1)
		return
	for frame in range(frames):
		await process_frame
	if current_scene == null:
		push_error("Forge main scene disappeared before acceptance")
		quit(1)
		return
	if not screenshot.is_empty():
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image == null or image.is_empty() or image.save_png(screenshot) != OK:
			push_error("Forge failed to capture the viewport")
			quit(1)
			return
	print("FORGE_PROJECT_READY")
	quit(0)
