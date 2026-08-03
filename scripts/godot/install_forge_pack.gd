extends SceneTree

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 2:
		_fail("Expected .gsfpack and project-relative target paths.")
	var pack_path := String(args[0]).simplify_path()
	var target_relative := String(args[1]).simplify_path().trim_prefix("/")
	if target_relative.contains(".."):
		_fail("Target path may not contain '..'.")
	var target_res := "res://" + target_relative
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(target_res))

	var helper := _read_json(pack_path.path_join("assets/godot_import.json"))
	var asset_type := String(helper.get("assetType", "character"))
	if asset_type == "terrain_set":
		_install_terrain_set(helper, target_res)
		print("PASS Forge Godot install: %s" % target_res)
		quit(0)
		return
	if asset_type == "building_kit":
		_install_building_kit(helper, target_res)
		print("PASS Forge Godot install: %s" % target_res)
		quit(0)
		return
	if asset_type == "map":
		var map_layout := _read_json(pack_path.path_join(String(helper.get("map", "assets/map-layout.json"))))
		_install_map(helper, map_layout, target_res)
		print("PASS Forge Godot install: %s" % target_res)
		quit(0)
		return
	if asset_type == "icon_set" or asset_type == "prop_set":
		_install_static_set(helper, target_res, asset_type)
		print("PASS Forge Godot install: %s" % target_res)
		quit(0)
		return
	var spec: Dictionary = _required_dict(helper, "spriteFrames", "godot_import.json")
	var atlas := _read_json(pack_path.path_join(String(spec["atlas"])))
	var textures: Array = _required_array(spec, "textures", "spriteFrames")
	var animations: Array = _required_array(spec, "animations", "spriteFrames")
	var atlas_frames: Array = _required_array(atlas, "frames", "atlas.json")
	var anchor: Dictionary = _required_dict(spec, "anchor", "spriteFrames")
	var frame_width := float(spec.get("frameWidth", atlas.get("frameWidth", 0)))
	var frame_height := float(spec.get("frameHeight", atlas.get("frameHeight", 0)))

	var texture_map := {}
	for relative_value in textures:
		var relative := String(relative_value)
		var file_name := relative.get_file()
		var target_texture := target_res.path_join(file_name)
		var texture := ResourceLoader.load(target_texture, "Texture2D", ResourceLoader.CACHE_MODE_REPLACE)
		if texture == null or not texture is Texture2D:
			_fail("Godot could not load external texture: %s" % file_name)
		texture_map[file_name] = texture

	var native_frames := SpriteFrames.new()
	for existing in native_frames.get_animation_names():
		native_frames.remove_animation(existing)
	for animation_value in animations:
		var animation: Dictionary = animation_value
		var animation_name := String(animation.get("name", "idle"))
		native_frames.add_animation(animation_name)
		native_frames.set_animation_speed(animation_name, float(animation.get("fps", 12.0)))
		native_frames.set_animation_loop(animation_name, bool(animation.get("loop", true)))
		for index_value in _required_array(animation, "frames", "animation"):
			var index := int(index_value)
			if index < 0 or index >= atlas_frames.size():
				_fail("Animation frame index is outside atlas bounds: %s" % index)
			var frame: Dictionary = atlas_frames[index]
			var image_name := String(frame.get("image", atlas.get("image", "sprite_sheet.png")))
			var atlas_texture := AtlasTexture.new()
			atlas_texture.atlas = texture_map.get(image_name)
			if atlas_texture.atlas == null:
				_fail("Atlas references missing texture: %s" % image_name)
			atlas_texture.region = Rect2(
				float(frame["x"]), float(frame["y"]),
				float(frame["width"]), float(frame["height"])
			)
			native_frames.add_frame(animation_name, atlas_texture)

	var frames_path := target_res.path_join("forge_sprite_frames.tres")
	if ResourceSaver.save(native_frames, frames_path) != OK:
		_fail("Failed to save SpriteFrames resource.")
	var root := Node2D.new()
	root.name = "ForgeAnimatedSprite"
	var sprite := AnimatedSprite2D.new()
	sprite.name = "AnimatedSprite2D"
	sprite.sprite_frames = native_frames
	var default_animation := String(spec.get("defaultAnimation", animations[0].get("name", "idle")))
	if !native_frames.has_animation(default_animation):
		_fail("Default animation is missing from SpriteFrames: %s" % default_animation)
	sprite.animation = default_animation
	sprite.position = Vector2(
		frame_width / 2.0 - float(anchor.get("x", frame_width / 2.0)),
		frame_height / 2.0 - float(anchor.get("y", frame_height))
	)
	root.add_child(sprite)
	sprite.owner = root
	var packed := PackedScene.new()
	if packed.pack(root) != OK:
		_fail("Failed to pack neutral AnimatedSprite2D scene.")
	if ResourceSaver.save(packed, target_res.path_join("forge_animated_sprite.tscn")) != OK:
		_fail("Failed to save neutral AnimatedSprite2D scene.")
	root.free()
	print("PASS Forge Godot install: %s" % target_res)
	quit(0)

func _install_terrain_set(helper: Dictionary, target_res: String) -> void:
	var tile_set := _create_terrain_set(helper, target_res)
	var tile_set_path := target_res.path_join("forge_terrain_set.tres")
	if ResourceSaver.save(tile_set, tile_set_path) != OK:
		_fail("Failed to save Forge terrain TileSet.")
	var root := Node2D.new()
	root.name = "ForgeTerrainPreview"
	var layer := TileMapLayer.new()
	layer.name = "Terrain"
	layer.tile_set = tile_set
	var preview_masks: Array = _terrain_mask_entries(helper)
	for mask_value in preview_masks:
		var mask_entry: Dictionary = mask_value
		if int(mask_entry.get("variant", 0)) != 0:
			continue
		var mask := int(mask_entry.get("mask", 0))
		var coords := Vector2i(int(mask_entry.get("x", mask % 4)), int(mask_entry.get("y", mask / 4)))
		layer.set_cell(Vector2i(mask % 4, mask / 4), 0, coords, 0)
	root.add_child(layer)
	layer.owner = root
	_save_scene(root, target_res.path_join("forge_terrain_preview.tscn"), "terrain preview")

func _create_terrain_set(helper: Dictionary, target_res: String) -> TileSet:
	var tile_size := int(helper.get("tileSize", 0))
	if tile_size != 16 and tile_size != 32:
		_fail("World TileSet tileSize must be 16 or 32.")
	var atlas_name := String(helper.get("atlas", helper.get("terrainAtlas", ""))).get_file()
	var texture := ResourceLoader.load(target_res.path_join(atlas_name), "Texture2D", ResourceLoader.CACHE_MODE_REPLACE)
	if texture == null or not texture is Texture2D:
		_fail("Godot could not load terrain atlas: %s" % atlas_name)
	var tile_set := TileSet.new()
	tile_set.tile_size = Vector2i(tile_size, tile_size)
	tile_set.add_terrain_set(0)
	tile_set.set_terrain_set_mode(0, TileSet.TERRAIN_MODE_MATCH_CORNERS)
	tile_set.add_terrain(0)
	tile_set.set_terrain_name(0, 0, String(helper.get("baseTerrain", "base")))
	tile_set.add_terrain(0)
	tile_set.set_terrain_name(0, 1, String(helper.get("overlayTerrain", "overlay")))
	tile_set.add_custom_data_layer(0)
	tile_set.set_custom_data_layer_name(0, "forge_mask")
	tile_set.set_custom_data_layer_type(0, TYPE_INT)
	tile_set.add_custom_data_layer(1)
	tile_set.set_custom_data_layer_name(1, "forge_variant")
	tile_set.set_custom_data_layer_type(1, TYPE_INT)
	tile_set.add_physics_layer(0)
	var source := TileSetAtlasSource.new()
	source.texture = texture
	source.texture_region_size = Vector2i(tile_size, tile_size)
	for mask_value in _terrain_mask_entries(helper):
		var mask_entry: Dictionary = mask_value
		var mask := int(mask_entry.get("mask", 0))
		var variant := int(mask_entry.get("variant", 0))
		var coords := Vector2i(int(mask_entry.get("x", mask % 4)), int(mask_entry.get("y", mask / 4)))
		source.create_tile(coords)
		var data := source.get_tile_data(coords, 0)
		data.terrain_set = 0
		data.terrain = -1
		data.set_terrain_peering_bit(TileSet.CELL_NEIGHBOR_TOP_LEFT_CORNER, 1 if mask & 1 else 0)
		data.set_terrain_peering_bit(TileSet.CELL_NEIGHBOR_TOP_RIGHT_CORNER, 1 if mask & 2 else 0)
		data.set_terrain_peering_bit(TileSet.CELL_NEIGHBOR_BOTTOM_RIGHT_CORNER, 1 if mask & 4 else 0)
		data.set_terrain_peering_bit(TileSet.CELL_NEIGHBOR_BOTTOM_LEFT_CORNER, 1 if mask & 8 else 0)
		data.set_custom_data("forge_mask", mask)
		data.set_custom_data("forge_variant", variant)
		var blocked := bool(helper.get("overlayCollision", "none") == "blocked") and mask != 0
		if blocked:
			data.add_collision_polygon(0)
			data.set_collision_polygon_points(0, 0, PackedVector2Array([
				Vector2(-tile_size / 2.0, -tile_size / 2.0),
				Vector2(tile_size / 2.0, -tile_size / 2.0),
				Vector2(tile_size / 2.0, tile_size / 2.0),
				Vector2(-tile_size / 2.0, tile_size / 2.0)
			]))
	tile_set.add_source(source, 0)
	return tile_set

func _terrain_mask_entries(helper: Dictionary) -> Array:
	var configured: Variant = helper.get("masks", null)
	if typeof(configured) == TYPE_ARRAY and not configured.is_empty():
		return configured
	var fallback: Array = []
	for mask in range(16):
		fallback.append({"mask": mask, "variant": 0, "x": mask % 4, "y": mask / 4})
	return fallback

func _install_building_kit(helper: Dictionary, target_res: String) -> void:
	var tile_set := _create_building_tile_set(helper, target_res)
	if ResourceSaver.save(tile_set, target_res.path_join("forge_building_kit.tres")) != OK:
		_fail("Failed to save Forge building TileSet.")
	var scenes_res := target_res.path_join("scenes")
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(scenes_res))
	var variants: Array = _required_array(helper, "variants", "godot_import.json")
	for variant_value in variants:
		if typeof(variant_value) != TYPE_DICTIONARY:
			_fail("Building variant must be an object.")
		var variant: Dictionary = variant_value
		var root := _create_building_node(variant, tile_set, int(helper.get("tileSize", 0)))
		var variant_id := String(variant.get("id", "building"))
		_save_scene(root, scenes_res.path_join(variant_id + ".tscn"), "building variant")

func _create_building_tile_set(helper: Dictionary, target_res: String) -> TileSet:
	var tile_size := int(helper.get("tileSize", 0))
	var atlas_name := String(helper.get("atlas", helper.get("buildingAtlas", ""))).get_file()
	var texture := ResourceLoader.load(target_res.path_join(atlas_name), "Texture2D", ResourceLoader.CACHE_MODE_REPLACE)
	if texture == null or not texture is Texture2D:
		_fail("Godot could not load building atlas: %s" % atlas_name)
	var tile_set := TileSet.new()
	tile_set.tile_size = Vector2i(tile_size, tile_size)
	var source := TileSetAtlasSource.new()
	source.texture = texture
	source.texture_region_size = Vector2i(tile_size, tile_size)
	for index in range(12):
		source.create_tile(Vector2i(index % 4, index / 4))
	tile_set.add_source(source, 0)
	return tile_set

func _create_building_node(variant: Dictionary, tile_set: TileSet, tile_size: int) -> Node2D:
	var width := int(variant.get("width", 3))
	var height := int(variant.get("height", 3))
	var entrance_x := int(variant.get("entranceX", 1))
	var root := Node2D.new()
	root.name = "ForgeBuilding"
	root.y_sort_enabled = true
	var modules := TileMapLayer.new()
	modules.name = "Modules"
	modules.tile_set = tile_set
	for y in range(height):
		for x in range(width):
			var module := _building_module_index(x, y, width, height)
			if y == height - 1:
				module = 10 if x == entrance_x else 9
			modules.set_cell(Vector2i(x, y), 0, Vector2i(module % 4, module / 4), 0)
	root.add_child(modules)
	modules.owner = root
	var body := StaticBody2D.new()
	body.name = "StaticBody2D"
	var body_shape := CollisionShape2D.new()
	var rectangle := RectangleShape2D.new()
	rectangle.size = Vector2(width * tile_size, height * tile_size)
	body_shape.shape = rectangle
	body_shape.position = Vector2(width * tile_size / 2.0, height * tile_size / 2.0)
	body.add_child(body_shape)
	body_shape.owner = root
	root.add_child(body)
	body.owner = root
	var entrance := Marker2D.new()
	entrance.name = "Entrance"
	entrance.position = Vector2((entrance_x + 0.5) * tile_size, (height + 0.5) * tile_size)
	root.add_child(entrance)
	entrance.owner = root
	var interaction := Area2D.new()
	interaction.name = "EntranceInteraction"
	interaction.position = entrance.position
	var interaction_shape := CollisionShape2D.new()
	var interaction_rectangle := RectangleShape2D.new()
	interaction_rectangle.size = Vector2(tile_size, tile_size)
	interaction_shape.shape = interaction_rectangle
	interaction.add_child(interaction_shape)
	interaction_shape.owner = root
	root.add_child(interaction)
	interaction.owner = root
	var occluder := LightOccluder2D.new()
	occluder.name = "RoofOccluder"
	var polygon := OccluderPolygon2D.new()
	polygon.polygon = PackedVector2Array([
		Vector2(0, 0), Vector2(width * tile_size, 0),
		Vector2(width * tile_size, height * tile_size), Vector2(0, height * tile_size)
	])
	occluder.occluder = polygon
	root.add_child(occluder)
	occluder.owner = root
	return root

func _building_module_index(x: int, y: int, width: int, height: int) -> int:
	if x == 0 and y == 0:
		return 5
	if x == width - 1 and y == 0:
		return 6
	if x == 0 and y == height - 1:
		return 7
	if x == width - 1 and y == height - 1:
		return 8
	if y == 0:
		return 1
	if x == width - 1:
		return 2
	if y == height - 1:
		return 3
	if x == 0:
		return 4
	return 0

func _install_map(helper: Dictionary, map_layout: Dictionary, target_res: String) -> void:
	var terrain_helper: Dictionary = _required_dict(helper, "terrainManifest", "godot_import.json")
	terrain_helper["terrainAtlas"] = helper.get("terrainAtlas", "")
	terrain_helper["atlas"] = helper.get("terrainAtlas", "")
	var terrain_set := _create_terrain_set(terrain_helper, target_res)
	if ResourceSaver.save(terrain_set, target_res.path_join("forge_terrain_set.tres")) != OK:
		_fail("Failed to save map terrain TileSet.")
	var building_helper: Dictionary = _required_dict(helper, "buildingManifest", "godot_import.json")
	building_helper["buildingAtlas"] = helper.get("buildingAtlas", "")
	building_helper["atlas"] = helper.get("buildingAtlas", "")
	var building_set := _create_building_tile_set(building_helper, target_res)
	if ResourceSaver.save(building_set, target_res.path_join("forge_building_kit.tres")) != OK:
		_fail("Failed to save map building TileSet.")
	var root := Node2D.new()
	root.name = "ForgeWorld"
	var ground := _new_world_layer("Ground", terrain_set, root)
	var terrain := _new_world_layer("Terrain", terrain_set, root)
	_new_world_layer("Decals", terrain_set, root)
	var width := int(map_layout.get("width", 0))
	var height := int(map_layout.get("height", 0))
	for y in range(height):
		for x in range(width):
			ground.set_cell(Vector2i(x, y), 0, Vector2i(0, 0), 0)
	for cell_value in _required_array(map_layout, "terrainCells", "map-layout.json"):
		var cell: Dictionary = cell_value
		var mask := int(cell.get("mask", 15))
		terrain.set_cell(Vector2i(int(cell["x"]), int(cell["y"])), 0, Vector2i(mask % 4, mask / 4), 0)
	var buildings := Node2D.new()
	buildings.name = "Buildings"
	root.add_child(buildings)
	buildings.owner = root
	var variants_by_id := {}
	for variant_value in _required_array(building_helper, "variants", "buildingManifest"):
		var variant: Dictionary = variant_value
		variants_by_id[String(variant.get("id", ""))] = variant
	for placed_value in _required_array(map_layout, "buildings", "map-layout.json"):
		var placed: Dictionary = placed_value
		var variant: Dictionary = variants_by_id.get(String(placed.get("variant", "")), placed)
		var building := _create_building_node(variant, building_set, int(helper.get("tileSize", 0)))
		building.name = String(placed.get("id", "Building"))
		building.position = Vector2(int(placed["x"]), int(placed["y"])) * int(helper.get("tileSize", 0))
		buildings.add_child(building)
		building.owner = root
		_set_owner_recursive(building, root)
	var props := Node2D.new()
	props.name = "Props"
	root.add_child(props)
	props.owner = root
	var prop_textures: Array = _required_array(helper, "propTextures", "godot_import.json") if helper.has("propTextures") else []
	var prop_index := 0
	for prop_value in _required_array(map_layout, "props", "map-layout.json"):
		if prop_textures.is_empty():
			break
		var prop: Dictionary = prop_value
		var texture_entry: Dictionary = prop_textures[prop_index % prop_textures.size()]
		var texture_name := String(texture_entry.get("texture", "")).get_file()
		var texture := ResourceLoader.load(target_res.path_join(texture_name), "Texture2D", ResourceLoader.CACHE_MODE_REPLACE)
		if texture == null:
			_fail("Could not load map prop texture: %s" % texture_name)
		var sprite := Sprite2D.new()
		sprite.name = String(prop.get("id", "Prop"))
		sprite.texture = texture
		sprite.position = Vector2((int(prop["x"]) + 0.5), (int(prop["y"]) + 0.5)) * int(helper.get("tileSize", 0))
		props.add_child(sprite)
		sprite.owner = root
		prop_index += 1
	_new_world_layer("Foreground", terrain_set, root)
	var navigation := NavigationRegion2D.new()
	navigation.name = "Navigation"
	var navigation_polygon := NavigationPolygon.new()
	navigation_polygon.agent_radius = max(1.0, int(helper.get("tileSize", 0)) * 0.2)
	var source_geometry := NavigationMeshSourceGeometryData2D.new()
	var outline_index := 0
	for outline_value in _required_array(map_layout, "navigationOutlines", "map-layout.json"):
		var points := PackedVector2Array()
		for point_value in outline_value:
			points.append(Vector2(float(point_value[0]), float(point_value[1])))
		if outline_index == 0:
			source_geometry.add_traversable_outline(points)
		else:
			source_geometry.add_obstruction_outline(points)
		outline_index += 1
	NavigationServer2D.bake_from_source_geometry_data(navigation_polygon, source_geometry)
	navigation.navigation_polygon = navigation_polygon
	root.add_child(navigation)
	navigation.owner = root
	var spawn := Marker2D.new()
	spawn.name = "Spawn"
	spawn.position = Vector2(float(map_layout["spawn"][0]) + 0.5, float(map_layout["spawn"][1]) + 0.5) * int(helper.get("tileSize", 0))
	root.add_child(spawn)
	spawn.owner = root
	var exit_marker := Marker2D.new()
	exit_marker.name = "Exit"
	exit_marker.position = Vector2(float(map_layout["exit"][0]) + 0.5, float(map_layout["exit"][1]) + 0.5) * int(helper.get("tileSize", 0))
	root.add_child(exit_marker)
	exit_marker.owner = root
	_save_scene(root, target_res.path_join("forge_world.tscn"), "world")

func _new_world_layer(name: String, tile_set: TileSet, root: Node2D) -> TileMapLayer:
	var layer := TileMapLayer.new()
	layer.name = name
	layer.tile_set = tile_set
	root.add_child(layer)
	layer.owner = root
	return layer

func _set_owner_recursive(node: Node, owner: Node) -> void:
	for child in node.get_children():
		child.owner = owner
		_set_owner_recursive(child, owner)

func _save_scene(root: Node, path: String, label: String) -> void:
	var packed := PackedScene.new()
	if packed.pack(root) != OK:
		_fail("Failed to pack %s scene." % label)
	if ResourceSaver.save(packed, path) != OK:
		_fail("Failed to save %s scene." % label)
	root.free()

func _install_static_set(helper: Dictionary, target_res: String, asset_type: String) -> void:
	var items: Array = _required_array(helper, "items", "godot_import.json")
	var scenes_res := target_res.path_join("scenes")
	if asset_type == "prop_set":
		DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(scenes_res))
	for item_value in items:
		if typeof(item_value) != TYPE_DICTIONARY:
			_fail("Static item must be an object.")
		var item: Dictionary = item_value
		var item_id := String(item.get("id", ""))
		if item_id.is_empty() or not item_id.is_valid_filename():
			_fail("Static item id is not engine-safe: %s" % item_id)
		var texture_res := target_res.path_join("items").path_join(item_id + ".png")
		var texture := ResourceLoader.load(texture_res, "Texture2D", ResourceLoader.CACHE_MODE_REPLACE)
		if texture == null or not texture is Texture2D:
			_fail("Godot could not load static item texture: %s" % item_id)
		if asset_type == "prop_set":
			var root := Node2D.new()
			root.name = "ForgeProp"
			var sprite := Sprite2D.new()
			sprite.name = "Sprite2D"
			sprite.texture = texture
			root.add_child(sprite)
			sprite.owner = root
			var packed := PackedScene.new()
			if packed.pack(root) != OK:
				_fail("Failed to pack prop scene: %s" % item_id)
			if ResourceSaver.save(packed, scenes_res.path_join(item_id + ".tscn")) != OK:
				_fail("Failed to save prop scene: %s" % item_id)
			root.free()

func _read_json(path: String) -> Dictionary:
	if !FileAccess.file_exists(path):
		_fail("Missing JSON: %s" % path)
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	if typeof(parsed) != TYPE_DICTIONARY:
		_fail("Expected JSON object: %s" % path)
	return parsed

func _required_dict(source: Dictionary, key: String, context: String) -> Dictionary:
	if !source.has(key) or typeof(source[key]) != TYPE_DICTIONARY:
		_fail("Expected %s.%s object." % [context, key])
	return source[key]

func _required_array(source: Dictionary, key: String, context: String) -> Array:
	if !source.has(key) or typeof(source[key]) != TYPE_ARRAY:
		_fail("Expected %s.%s array." % [context, key])
	return source[key]

func _fail(message: String) -> void:
	push_error(message)
	print("FAIL Forge Godot install: %s" % message)
	quit(1)
