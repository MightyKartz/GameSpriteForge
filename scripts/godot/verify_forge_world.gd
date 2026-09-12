extends SceneTree

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 1:
		_fail("Expected one res:// world target path.")
		return
	var target := String(args[0]).trim_suffix("/")
	var terrain := ResourceLoader.load(target.path_join("terrain/forge_terrain_set.tres"), "TileSet") as TileSet
	if not _verify_terrain_data(terrain):
		return
	var building_kit := ResourceLoader.load(target.path_join("buildings/forge_building_kit.tres"), "TileSet") as TileSet
	if building_kit == null or building_kit.get_source_count() != 1:
		_fail("Building TileSet did not load with exactly one atlas source.")
		return
	var building_source := building_kit.get_source(building_kit.get_source_id(0)) as TileSetAtlasSource
	if building_source == null or building_source.texture == null or building_source.get_tiles_count() != 12:
		_fail("Building TileSet must contain its external atlas and all 12 modules.")
		return
	var building_scene_count := 0
	var scenes_dir := target.path_join("buildings/scenes")
	for file_name in DirAccess.get_files_at(scenes_dir):
		if not file_name.ends_with(".tscn"):
			continue
		var packed := ResourceLoader.load(scenes_dir.path_join(file_name), "PackedScene") as PackedScene
		if packed == null:
			_fail("Building scene did not load: %s" % file_name)
			return
		var building := packed.instantiate()
		var valid := _verify_building_shapes(building)
		building.free()
		if not valid:
			return
		building_scene_count += 1
	if building_scene_count == 0:
		_fail("Building kit contains no saved scenes.")
		return
	var world_scene := ResourceLoader.load(target.path_join("world/forge_world.tscn"), "PackedScene") as PackedScene
	if world_scene == null:
		_fail("World scene did not load.")
		return
	var world: Node = world_scene.instantiate()
	for required in [
		"Ground", "Terrain", "Decals", "Buildings", "Props", "Foreground",
		"Navigation", "Spawn", "Exit"
	]:
		if world.get_node_or_null(required) == null:
			world.free()
			_fail("World scene is missing node: %s" % required)
			return
	if not world.get_node("Ground") is TileMapLayer or not world.get_node("Terrain") is TileMapLayer:
		world.free()
		_fail("Ground and Terrain must be TileMapLayer nodes.")
		return
	for layer_name in ["Ground", "Terrain"]:
		var layer := world.get_node(layer_name) as TileMapLayer
		if not _verify_terrain_data(layer.tile_set):
			world.free()
			return
	var navigation := world.get_node("Navigation") as NavigationRegion2D
	if navigation.navigation_polygon == null or navigation.navigation_polygon.get_polygon_count() == 0:
		world.free()
		_fail("World navigation polygon is empty.")
		return
	if world.get_node("Buildings").get_child_count() == 0:
		world.free()
		_fail("World contains no buildings.")
		return
	for building in world.get_node("Buildings").get_children():
		if not _verify_building_shapes(building):
			world.free()
			return
	world.free()
	print("PASS Forge world resources load headlessly: %s" % target)
	quit(0)

func _verify_terrain_data(tile_set: TileSet) -> bool:
	if tile_set == null or tile_set.get_source_count() != 1:
		_fail("Terrain TileSet must load with exactly one atlas source.")
		return false
	if tile_set.get_terrain_sets_count() != 1 or tile_set.get_terrains_count(0) != 2 or tile_set.get_terrain_set_mode(0) != TileSet.TERRAIN_MODE_MATCH_CORNERS:
		_fail("Terrain TileSet must retain its two corner-matched terrains.")
		return false
	if tile_set.get_custom_data_layers_count() != 2 or tile_set.get_custom_data_layer_name(0) != "forge_mask" or tile_set.get_custom_data_layer_name(1) != "forge_variant" or tile_set.get_custom_data_layer_type(0) != TYPE_INT or tile_set.get_custom_data_layer_type(1) != TYPE_INT:
		_fail("Terrain custom-data layers are missing or invalid.")
		return false
	var source := tile_set.get_source(tile_set.get_source_id(0)) as TileSetAtlasSource
	if source == null or source.texture == null or source.get_tiles_count() < 16:
		_fail("Terrain TileSet must retain an external atlas and all 16 masks.")
		return false
	var seen_masks := {}
	var seen_variants := {}
	var corners := [TileSet.CELL_NEIGHBOR_TOP_LEFT_CORNER, TileSet.CELL_NEIGHBOR_TOP_RIGHT_CORNER, TileSet.CELL_NEIGHBOR_BOTTOM_RIGHT_CORNER, TileSet.CELL_NEIGHBOR_BOTTOM_LEFT_CORNER]
	for index in source.get_tiles_count():
		var coords := source.get_tile_id(index)
		var data := source.get_tile_data(coords, 0)
		if data == null or data.terrain_set != 0:
			_fail("Terrain tile has no terrain-set association: %s" % coords)
			return false
		var mask := int(data.get_custom_data("forge_mask"))
		var variant := int(data.get_custom_data("forge_variant"))
		var key := "%s/%s" % [mask, variant]
		if mask < 0 or mask > 15 or variant < 0 or seen_variants.has(key):
			_fail("Terrain custom-data mask/variant is invalid or duplicated: %s" % coords)
			return false
		seen_masks[mask] = true
		seen_variants[key] = true
		for bit in 4:
			if data.get_terrain_peering_bit(corners[bit]) != (1 if mask & (1 << bit) else 0):
				_fail("Terrain corner bits disagree with custom-data mask: %s" % coords)
				return false
	if seen_masks.size() != 16:
		_fail("Terrain custom data does not cover all 16 masks.")
		return false
	return true

func _verify_building_shapes(root: Node) -> bool:
	for node_name in ["StaticBody2D", "EntranceInteraction"]:
		var body := root.get_node_or_null(node_name)
		if body == null or body.get_child_count() != 1:
			_fail("Building scene lost its saved collision/interaction shape: %s" % node_name)
			return false
		var collision := body.get_child(0) as CollisionShape2D
		if collision == null or not collision.shape is RectangleShape2D or collision.shape.size.x <= 0 or collision.shape.size.y <= 0:
			_fail("Building scene has an empty collision/interaction shape: %s" % node_name)
			return false
	return true

func _fail(message: String) -> void:
	push_error(message)
	print("FAIL Forge world verification: %s" % message)
	quit(1)
