extends SceneTree


func fail(message: String) -> void:
	printerr("DELIVERY_CHECK_FAILED: " + message)
	quit(1)


func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 1:
		fail("expected one manifest path")
		return
	var manifest = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	if not manifest is Array or manifest.is_empty():
		fail("expected nonempty manifest")
		return
	for item in manifest:
		var texture = load(item["resource"])
		if not texture is Texture2D:
			fail("not a Texture2D: " + item["resource"])
			return
		var actual: Image = texture.get_image()
		var expected := Image.load_from_file(item["source"])
		if actual == null or expected == null or actual.get_size() != expected.get_size():
			fail("missing image or incorrect dimensions")
			return
		actual.convert(Image.FORMAT_RGBA8)
		expected.convert(Image.FORMAT_RGBA8)
		var a := actual.get_data()
		var b := expected.get_data()
		if a.size() != b.size():
			fail("incorrect pixel inventory")
			return
		for offset in range(0, b.size(), 4):
			# Godot's default alpha-border repair can change RGB at alpha zero.
			if a[offset + 3] != b[offset + 3] or (b[offset + 3] != 0 and (
				a[offset] != b[offset] or a[offset + 1] != b[offset + 1] or a[offset + 2] != b[offset + 2])):
				fail("visible RGBA mismatch: " + item["resource"])
				return
	print("DELIVERY_CHECK_PASSED:" + str(manifest.size()))
	quit(0)
