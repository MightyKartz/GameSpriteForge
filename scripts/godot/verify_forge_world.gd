extends SceneTree

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 1:
		_fail("Expected one res:// world target path.")
	var target := String(args[0]).trim_suffix("/")
	var terrain := ResourceLoader.load(target.path_join("terrain/forge_terrain_set.tres"), "TileSet")
	if terrain == null or not terrain is TileSet:
		_fail("Terrain TileSet did not load.")
	if terrain.get_source_count() != 1:
		_fail("Terrain TileSet must contain exactly one atlas source.")
	var building_kit := ResourceLoader.load(target.path_join("buildings/forge_building_kit.tres"), "TileSet")
	if building_kit == null or not building_kit is TileSet:
		_fail("Building TileSet did not load.")
	var world_scene := ResourceLoader.load(target.path_join("world/forge_world.tscn"), "PackedScene")
	if world_scene == null or not world_scene is PackedScene:
		_fail("World scene did not load.")
	var world: Node = world_scene.instantiate()
	for required in [
		"Ground", "Terrain", "Decals", "Buildings", "Props", "Foreground",
		"Navigation", "Spawn", "Exit"
	]:
		if world.get_node_or_null(required) == null:
			_fail("World scene is missing node: %s" % required)
	if not world.get_node("Ground") is TileMapLayer or not world.get_node("Terrain") is TileMapLayer:
		_fail("Ground and Terrain must be TileMapLayer nodes.")
	var navigation := world.get_node("Navigation") as NavigationRegion2D
	if navigation.navigation_polygon == null or navigation.navigation_polygon.get_polygon_count() == 0:
		_fail("World navigation polygon is empty.")
	if world.get_node("Buildings").get_child_count() == 0:
		_fail("World contains no buildings.")
	world.free()
	print("PASS Forge world resources load headlessly: %s" % target)
	quit(0)

func _fail(message: String) -> void:
	push_error(message)
	print("FAIL Forge world verification: %s" % message)
	quit(1)
