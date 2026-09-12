extends SceneTree

var _failed := false
var _phase := "install"

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 2:
		_fail("Expected .gsfpack and project-relative target paths.")
		return
	var pack_path := String(args[0]).simplify_path()
	var target_relative := String(args[1]).simplify_path().trim_prefix("/")
	if target_relative.contains(".."):
		_fail("Target path may not contain '..'.")
		return
	var target_res := "res://" + target_relative
	if not (args.size() > 2 and args[2] == "--verify"):
		if DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(target_res)) != OK:
			_fail("Failed to create installation target.")
			return

	var helper := _read_json(pack_path.path_join("assets/godot_import.json"))
	if _failed:
		return
	var animation_count: int = helper.get("spriteFrames", {}).get("animations", []).size()
	var asset_type := String(helper.get("assetType", "character" if animation_count > 1 else "animation"))
	if not helper.has("assetType"):
		var metadata := _read_json(pack_path.path_join("forgepack.json"))
		if _failed:
			return
		var manifest := _read_json(pack_path.path_join("assets/manifest.json"))
		if _failed:
			return
		asset_type = String(manifest.get("assetType", metadata.get("assetType", asset_type)))
	if args.size() > 2 and args[2] == "--verify":
		_phase = "verify"
		_verify_native_resources(helper, pack_path, target_res, asset_type)
		_complete(target_res, asset_type)
		return
	if asset_type == "layered":
		_verify_layered_resources(target_res)
		_complete(target_res, asset_type)
		return
	if asset_type == "terrain_set":
		_install_terrain_set(helper, target_res)
		_complete(target_res, asset_type)
		return
	if asset_type == "building_kit":
		_install_building_kit(helper, target_res)
		_complete(target_res, asset_type)
		return
	if asset_type == "map":
		var map_layout := _read_json(pack_path.path_join(String(helper.get("map", "assets/map-layout.json"))))
		if _failed:
			return
		_install_map(helper, map_layout, target_res)
		_complete(target_res, asset_type)
		return
	if asset_type == "icon_set" or asset_type == "prop_set":
		_install_static_set(helper, target_res, asset_type)
		_complete(target_res, asset_type)
		return
	var spec: Dictionary = _required_dict(helper, "spriteFrames", "godot_import.json")
	if _failed:
		return
	var atlas := _read_json(pack_path.path_join(String(spec["atlas"])))
	if _failed:
		return
	var textures: Array = _required_array(spec, "textures", "spriteFrames")
	if _failed:
		return
	var animations: Array = _required_array(spec, "animations", "spriteFrames")
	if _failed:
		return
	var atlas_frames: Array = _required_array(atlas, "frames", "atlas.json")
	if _failed:
		return
	var anchor: Dictionary = _required_dict(spec, "anchor", "spriteFrames")
	if _failed:
		return
	var rendering: Dictionary = spec.get("rendering", {})
	if typeof(rendering) != TYPE_DICTIONARY:
		_fail("spriteFrames.rendering must be an object.")
		return
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
			return
		texture_map[file_name] = texture

	var native_frames := SpriteFrames.new()
	for existing in native_frames.get_animation_names():
		native_frames.remove_animation(existing)
	for animation_value in animations:
		var animation: Dictionary = animation_value
		var animation_name := String(animation.get("name", "idle"))
		var animation_fps := float(animation.get("fps", 12.0))
		var animation_frames: Array = _required_array(animation, "frames", "animation")
		if _failed:
			return
		var frame_durations: Array = animation.get("frameDurationsMs", [])
		if !frame_durations.is_empty() and frame_durations.size() != animation_frames.size():
			_fail("Animation frameDurationsMs must match frames: %s" % animation_name)
			return
		native_frames.add_animation(animation_name)
		native_frames.set_animation_speed(animation_name, animation_fps)
		native_frames.set_animation_loop(animation_name, bool(animation.get("loop", true)))
		for animation_frame_index in animation_frames.size():
			var index_value = animation_frames[animation_frame_index]
			var index := int(index_value)
			if index < 0 or index >= atlas_frames.size():
				_fail("Animation frame index is outside atlas bounds: %s" % index)
				return
			var frame: Dictionary = atlas_frames[index]
			var image_name := String(frame.get("image", atlas.get("image", "sprite_sheet.png")))
			var atlas_texture := AtlasTexture.new()
			atlas_texture.atlas = texture_map.get(image_name)
			if atlas_texture.atlas == null:
				_fail("Atlas references missing texture: %s" % image_name)
				return
			atlas_texture.region = Rect2(
				float(frame["x"]), float(frame["y"]),
				float(frame["width"]), float(frame["height"])
			)
			atlas_texture.filter_clip = true
			var relative_duration := 1.0
			if !frame_durations.is_empty():
				var duration_ms := float(frame_durations[animation_frame_index])
				if duration_ms <= 0.0:
					_fail("Animation frame duration must be positive: %s" % animation_name)
					return
				relative_duration = duration_ms * animation_fps / 1000.0
			native_frames.add_frame(animation_name, atlas_texture, relative_duration)

	var frames_path := target_res.path_join("forge_sprite_frames.tres")
	if ResourceSaver.save(native_frames, frames_path) != OK:
		_fail("Failed to save SpriteFrames resource.")
		return
	# Reload the saved file so the scene references the delivered SpriteFrames,
	# instead of embedding a second, independently editable copy of its animations.
	native_frames = ResourceLoader.load(frames_path, "SpriteFrames", ResourceLoader.CACHE_MODE_REPLACE) as SpriteFrames
	if native_frames == null:
		_fail("Could not reload saved SpriteFrames resource.")
		return
	var root := Node2D.new()
	root.name = "ForgeAnimatedSprite"
	var player_script := load(target_res.path_join("forge_player.gd")) as Script
	if player_script == null or not player_script.can_instantiate():
		root.free()
		_fail("Cannot load the Forge playback controller.")
		return
	root.set_script(player_script)
	var sprite := AnimatedSprite2D.new()
	sprite.name = "AnimatedSprite2D"
	sprite.sprite_frames = native_frames
	var texture_filter := String(rendering.get("textureFilter", "nearest"))
	match texture_filter:
		"nearest":
			sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
		"linear":
			sprite.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
		_:
			_fail("Unsupported spriteFrames.rendering.textureFilter: %s" % texture_filter)
			return
	sprite.texture_repeat = CanvasItem.TEXTURE_REPEAT_DISABLED
	var blend := String(rendering.get("blendMode", "normal"))
	if not blend in ["normal", "add", "multiply"]:
		_fail("Unsupported animation blendMode: " + blend)
		return
	if blend == "multiply":
		var multiply_shader := load(target_res.path_join("forge_alpha_multiply.gdshader")) as Shader
		if multiply_shader == null:
			_fail("Missing alpha-aware multiply shader.")
			return
		var blend_material := ShaderMaterial.new()
		blend_material.shader = multiply_shader
		sprite.material = blend_material
	else:
		var blend_material := CanvasItemMaterial.new()
		blend_material.blend_mode = 1 if blend == "add" else 0
		sprite.material = blend_material
	var default_animation := String(spec.get("defaultAnimation", animations[0].get("name", "idle")))
	if !native_frames.has_animation(default_animation):
		_fail("Default animation is missing from SpriteFrames: %s" % default_animation)
		return
	sprite.animation = default_animation
	var anchor_x := float(anchor.get("x", frame_width / 2.0))
	var anchor_y := float(anchor.get("y", frame_height))
	var pixel_snap := bool(rendering.get("pixelSnap", texture_filter == "nearest"))
	if pixel_snap:
		if !is_equal_approx(anchor_x, round(anchor_x)) or !is_equal_approx(anchor_y, round(anchor_y)):
			_fail("Pixel-snapped sprite anchor must use integer coordinates.")
			return
		sprite.centered = false
		sprite.position = Vector2(-round(anchor_x), -round(anchor_y))
	else:
		sprite.centered = true
		sprite.position = Vector2(
			frame_width / 2.0 - anchor_x,
			frame_height / 2.0 - anchor_y
		)
	root.add_child(sprite)
	sprite.owner = root
	var packed := PackedScene.new()
	if packed.pack(root) != OK:
		_fail("Failed to pack neutral AnimatedSprite2D scene.")
		return
	if ResourceSaver.save(packed, target_res.path_join("forge_animated_sprite.tscn")) != OK:
		_fail("Failed to save neutral AnimatedSprite2D scene.")
		return
	root.free()
	_complete(target_res, asset_type)

func _install_terrain_set(helper: Dictionary, target_res: String) -> void:
	var tile_set := _create_terrain_set(helper, target_res)
	if _failed:
		return
	var tile_set_path := target_res.path_join("forge_terrain_set.tres")
	if ResourceSaver.save(tile_set, tile_set_path) != OK:
		_fail("Failed to save Forge terrain TileSet.")
		return
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
	if _failed:
		return

func _create_terrain_set(helper: Dictionary, target_res: String) -> TileSet:
	var tile_size := int(helper.get("tileSize", 0))
	if tile_size != 16 and tile_size != 32:
		_fail("World TileSet tileSize must be 16 or 32.")
		return null
	var atlas_name := String(helper.get("atlas", helper.get("terrainAtlas", ""))).get_file()
	var texture := ResourceLoader.load(target_res.path_join(atlas_name), "Texture2D", ResourceLoader.CACHE_MODE_REPLACE)
	if texture == null or not texture is Texture2D:
		_fail("Godot could not load terrain atlas: %s" % atlas_name)
		return null
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
	# TileData setters resolve terrain/custom-data/physics layers through this owner.
	tile_set.add_source(source, 0)
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
	if _failed:
		return
	if ResourceSaver.save(tile_set, target_res.path_join("forge_building_kit.tres")) != OK:
		_fail("Failed to save Forge building TileSet.")
		return
	var scenes_res := target_res.path_join("scenes")
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(scenes_res))
	var variants: Array = _required_array(helper, "variants", "godot_import.json")
	if _failed:
		return
	for variant_value in variants:
		if typeof(variant_value) != TYPE_DICTIONARY:
			_fail("Building variant must be an object.")
			return
		var variant: Dictionary = variant_value
		var root := _create_building_node(variant, tile_set, int(helper.get("tileSize", 0)))
		var variant_id := String(variant.get("id", "building"))
		_save_scene(root, scenes_res.path_join(variant_id + ".tscn"), "building variant")
		if _failed:
			return

func _create_building_tile_set(helper: Dictionary, target_res: String) -> TileSet:
	var tile_size := int(helper.get("tileSize", 0))
	var atlas_name := String(helper.get("atlas", helper.get("buildingAtlas", ""))).get_file()
	var texture := ResourceLoader.load(target_res.path_join(atlas_name), "Texture2D", ResourceLoader.CACHE_MODE_REPLACE)
	if texture == null or not texture is Texture2D:
		_fail("Godot could not load building atlas: %s" % atlas_name)
		return null
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
	root.add_child(body)
	body.owner = root
	body_shape.owner = root
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
	root.add_child(interaction)
	interaction.owner = root
	interaction_shape.owner = root
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
	if _failed:
		return
	terrain_helper["terrainAtlas"] = helper.get("terrainAtlas", "")
	terrain_helper["atlas"] = helper.get("terrainAtlas", "")
	var terrain_set := _create_terrain_set(terrain_helper, target_res)
	if _failed:
		return
	if ResourceSaver.save(terrain_set, target_res.path_join("forge_terrain_set.tres")) != OK:
		_fail("Failed to save map terrain TileSet.")
		return
	var building_helper: Dictionary = _required_dict(helper, "buildingManifest", "godot_import.json")
	if _failed:
		return
	building_helper["buildingAtlas"] = helper.get("buildingAtlas", "")
	building_helper["atlas"] = helper.get("buildingAtlas", "")
	var building_set := _create_building_tile_set(building_helper, target_res)
	if _failed:
		return
	if ResourceSaver.save(building_set, target_res.path_join("forge_building_kit.tres")) != OK:
		_fail("Failed to save map building TileSet.")
		return
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
	if _failed:
		return
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
			return
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
	if _failed:
		return

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
		return
	if ResourceSaver.save(packed, path) != OK:
		_fail("Failed to save %s scene." % label)
		return
	root.free()

func _install_static_set(helper: Dictionary, target_res: String, asset_type: String) -> void:
	var items: Array = _required_array(helper, "items", "godot_import.json")
	if _failed:
		return
	var scenes_res := target_res.path_join("scenes")
	if asset_type == "prop_set":
		DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(scenes_res))
	for item_value in items:
		if typeof(item_value) != TYPE_DICTIONARY:
			_fail("Static item must be an object.")
			return
		var item: Dictionary = item_value
		var item_id := String(item.get("id", ""))
		if item_id.is_empty() or not item_id.is_valid_filename():
			_fail("Static item id is not engine-safe: %s" % item_id)
			return
		var texture_res := target_res.path_join("items").path_join(item_id + ".png")
		var texture := ResourceLoader.load(texture_res, "Texture2D", ResourceLoader.CACHE_MODE_REPLACE)
		if texture == null or not texture is Texture2D:
			_fail("Godot could not load static item texture: %s" % item_id)
			return
		if asset_type == "prop_set":
			var root := Node2D.new()
			root.name = "ForgeProp"
			var sprite := Sprite2D.new()
			sprite.name = "Sprite2D"
			sprite.texture = texture
			# Absent in legacy static Packs: preserve inherited filtering and
			# centered geometry rather than reinterpreting old placements.
			if helper.has("rendering"):
				var rendering: Dictionary = _required_dict(helper, "rendering", "static helper")
				if _failed:
					return
				match String(rendering.get("textureFilter", "")):
					"nearest":
						sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
					"linear":
						sprite.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
					_:
						_fail("Unsupported static rendering.textureFilter")
						return
				sprite.texture_repeat = CanvasItem.TEXTURE_REPEAT_DISABLED
				var anchor: Dictionary = _required_dict(helper, "anchor", "static helper")
				if _failed:
					return
				var anchor_position := Vector2(float(anchor["x"]), float(anchor["y"]))
				if bool(rendering.get("pixelSnap", false)):
					if !anchor_position.is_equal_approx(anchor_position.round()):
						_fail("Pixel-snapped static anchor must use integer coordinates")
						return
				sprite.centered = false
				sprite.position = -anchor_position
			root.add_child(sprite)
			sprite.owner = root
			var packed := PackedScene.new()
			if packed.pack(root) != OK:
				_fail("Failed to pack prop scene: %s" % item_id)
				return
			if ResourceSaver.save(packed, scenes_res.path_join(item_id + ".tscn")) != OK:
				_fail("Failed to save prop scene: %s" % item_id)
				return
			root.free()

func _read_json(path: String) -> Dictionary:
	if !FileAccess.file_exists(path):
		_fail("Missing JSON: %s" % path)
		return {}
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	if typeof(parsed) != TYPE_DICTIONARY:
		_fail("Expected JSON object: %s" % path)
		return {}
	return parsed

func _required_dict(source: Dictionary, key: String, context: String) -> Dictionary:
	if !source.has(key) or typeof(source[key]) != TYPE_DICTIONARY:
		_fail("Expected %s.%s object." % [context, key])
		return {}
	return source[key]

func _required_array(source: Dictionary, key: String, context: String) -> Array:
	if !source.has(key) or typeof(source[key]) != TYPE_ARRAY:
		_fail("Expected %s.%s array." % [context, key])
		return []
	return source[key]

func _verify_layered_resources(target_res: String) -> void:
	var manifest := _read_json(target_res.path_join("manifest.json"))
	if _failed:
		return
	var root := _load_scene(target_res.path_join("layered.tscn"))
	if _failed:
		return
	for method in ["play", "pause", "seek", "set_speed", "state", "reset_pose"]:
		if not root.has_method(method):
			root.free()
			_fail("Layered scene has no common playback method: " + method)
			return
	root.call("reset_pose")
	var container := root.get_node_or_null("Layers")
	var layers: Array = manifest.get("layers", [])
	if container == null or container.get_child_count() != layers.size():
		root.free()
		_fail("Layered scene has missing or extra layers.")
		return
	for index in layers.size():
		var layer: Dictionary = layers[index]
		var node := container.get_child(index) as Node2D
		var sprite := node.get_node_or_null("Sprite") as Sprite2D if node != null else null
		var pivot := Vector2(float(layer["pivot"][0]), float(layer["pivot"][1]))
		var transform: Dictionary = layer["transform"]
		var position := pivot + Vector2(float(transform["position"][0]), float(transform["position"][1]))
		var scale_value := Vector2(float(transform["scale"][0]), float(transform["scale"][1]))
		var expected_filter := CanvasItem.TEXTURE_FILTER_NEAREST if String(manifest["sampling"]) == "nearest" else CanvasItem.TEXTURE_FILTER_LINEAR
		if node == null or String(node.name) != String(layer["id"]) or sprite == null or sprite.texture == null:
			root.free()
			_fail("Layered scene texture/order differs from its manifest.")
			return
		if sprite.texture.get_size() != Vector2(float(manifest["canvas"]["width"]), float(manifest["canvas"]["height"])) or sprite.centered or not sprite.position.is_equal_approx(-pivot):
			root.free()
			_fail("Layered scene lost its shared canvas/pivot.")
			return
		if not node.position.is_equal_approx(position) or not node.scale.is_equal_approx(scale_value) or not is_equal_approx(node.rotation, deg_to_rad(float(transform["rotationDegrees"]))) or not is_equal_approx(node.modulate.a, float(transform["opacity"])):
			root.free()
			_fail("Layered initial transform differs from the manifest.")
			return
		if not _blend_matches(sprite, String(layer["blend"])) or sprite.texture_filter != expected_filter:
			root.free()
			_fail("Layered blend/filter differs from the manifest.")
			return
	root.free()

func _fail(message: String) -> void:
	_failed = true
	push_error(message)
	print("FAIL Forge Godot install: %s" % message)
	quit(1)

# Verification runs in a fresh Godot process so it reads saved resources instead
# of accepting the objects that were just constructed by the installer.
func _verify_native_resources(helper: Dictionary, pack_path: String, target_res: String, asset_type: String) -> void:
	if asset_type == "layered":
		_verify_layered_resources(target_res)
		return
	if asset_type == "icon_set" or asset_type == "prop_set":
		var items := _required_array(helper, "items", "godot_import.json")
		if _failed:
			return
		if items.is_empty():
			_fail("Installed static set has no items.")
			return
		for item in items:
			var item_id := String(item.get("id", ""))
			var texture_path := target_res.path_join("items").path_join(item_id + ".png")
			var texture := ResourceLoader.load(texture_path, "Texture2D") as Texture2D
			if texture == null or texture.get_width() <= 0 or texture.get_height() <= 0:
				_fail("Installed static texture is missing or empty: %s" % item_id)
				return
			if helper.has("frameWidth") and (texture.get_width() != int(helper["frameWidth"]) or texture.get_height() != int(helper["frameHeight"])):
				_fail("Installed static texture dimensions differ from the Pack: %s" % item_id)
				return
			if asset_type == "prop_set":
				var root := _load_scene(target_res.path_join("scenes").path_join(item_id + ".tscn"))
				if _failed:
					return
				var sprite := root.get_node_or_null("Sprite2D") as Sprite2D
				if sprite == null or sprite.texture == null or sprite.texture.resource_path != texture_path:
					root.free()
					_fail("Installed prop scene has no matching external texture: %s" % item_id)
					return
				if helper.has("rendering"):
					_verify_rendering(sprite, helper["rendering"], helper["anchor"], texture.get_size(), false)
				root.free()
				if _failed:
					return
		return
	if asset_type == "terrain_set" or asset_type == "building_kit" or asset_type == "map":
		if asset_type != "building_kit":
			_verify_tile_set(target_res.path_join("forge_terrain_set.tres"), helper if asset_type == "terrain_set" else helper["terrainManifest"])
		if asset_type != "terrain_set":
			_verify_tile_set(target_res.path_join("forge_building_kit.tres"))
		if _failed:
			return
		var scene_paths: Array[String] = []
		if asset_type == "terrain_set":
			scene_paths.append(target_res.path_join("forge_terrain_preview.tscn"))
		elif asset_type == "map":
			scene_paths.append(target_res.path_join("forge_world.tscn"))
		else:
			for variant in _required_array(helper, "variants", "godot_import.json"):
				scene_paths.append(target_res.path_join("scenes").path_join(String(variant["id"]) + ".tscn"))
			if _failed:
				return
		if scene_paths.is_empty():
			_fail("Installed world Pack has no scenes.")
			return
		for scene_path in scene_paths:
			var root := _load_scene(scene_path)
			if _failed:
				return
			if asset_type == "building_kit":
				_verify_building_shapes(root)
			elif asset_type == "map":
				var buildings := root.get_node_or_null("Buildings")
				if buildings == null:
					root.free()
					_fail("Installed map has no Buildings node.")
					return
				for building in buildings.get_children():
					_verify_building_shapes(building)
					if _failed:
						break
			root.free()
			if _failed:
				return
		return

	var spec := _required_dict(helper, "spriteFrames", "godot_import.json")
	if _failed:
		return
	var atlas := _read_json(pack_path.path_join(String(spec["atlas"])))
	if _failed:
		return
	var frames := ResourceLoader.load(target_res.path_join("forge_sprite_frames.tres"), "SpriteFrames") as SpriteFrames
	if frames == null or frames.get_animation_names().size() != spec["animations"].size():
		_fail("Installed SpriteFrames has missing or extra animations.")
		return
	for animation in spec["animations"]:
		var animation_name := String(animation["name"])
		if not frames.has_animation(animation_name) or frames.get_frame_count(animation_name) != animation["frames"].size():
			_fail("Installed animation has missing frames: %s" % animation_name)
			return
		if not is_equal_approx(frames.get_animation_speed(animation_name), float(animation["fps"])) or frames.get_animation_loop(animation_name) != bool(animation["loop"]):
			_fail("Installed animation timing differs from the Pack: %s" % animation_name)
			return
		for index in animation["frames"].size():
			var texture := frames.get_frame_texture(animation_name, index) as AtlasTexture
			var source: Dictionary = atlas["frames"][int(animation["frames"][index])]
			var expected_region := Rect2(float(source["x"]), float(source["y"]), float(source["width"]), float(source["height"]))
			var expected_path := target_res.path_join(String(source.get("image", atlas.get("image", "sprite_sheet.png"))))
			if texture == null or texture.atlas == null or texture.atlas.resource_path != expected_path or texture.region != expected_region or not texture.filter_clip:
				_fail("Installed animation atlas reference differs from the Pack: %s frame %s" % [animation_name, index])
				return
			var expected_duration := 1.0
			if animation.has("frameDurationsMs"):
				expected_duration = float(animation["frameDurationsMs"][index]) * float(animation["fps"]) / 1000.0
			if not is_equal_approx(frames.get_frame_duration(animation_name, index), expected_duration):
				_fail("Installed per-frame duration differs from the Pack: %s frame %s" % [animation_name, index])
				return
	var root := _load_scene(target_res.path_join("forge_animated_sprite.tscn"))
	if _failed:
		return
	var sprite := root.get_node_or_null("AnimatedSprite2D") as AnimatedSprite2D
	if sprite == null or sprite.sprite_frames == null or sprite.sprite_frames.resource_path != frames.resource_path or sprite.animation != String(spec["defaultAnimation"]):
		root.free()
		_fail("Installed AnimatedSprite2D scene does not reference the expected SpriteFrames/default animation.")
		return
	_verify_rendering(sprite, spec.get("rendering", {"textureFilter": "nearest", "pixelSnap": true}), spec["anchor"], Vector2(float(spec.get("frameWidth", atlas["frameWidth"])), float(spec.get("frameHeight", atlas["frameHeight"]))), true)
	if not root.has_method("play") or not root.has_method("seek") or not root.has_method("set_speed"):
		_fail("Installed animation scene has no common playback interface.")
	root.free()

func _verify_rendering(sprite: Node2D, rendering: Dictionary, anchor: Dictionary, size: Vector2, animated: bool) -> void:
	if animated:
		if not _blend_matches(sprite, String(rendering.get("blendMode", "normal"))):
			_fail("Installed animation blendMode differs from the Pack.")
			return
	var filter_name := String(rendering.get("textureFilter", "nearest"))
	var expected_filter := CanvasItem.TEXTURE_FILTER_NEAREST if filter_name == "nearest" else CanvasItem.TEXTURE_FILTER_LINEAR
	var anchor_position := Vector2(float(anchor["x"]), float(anchor["y"]))
	var centered := animated and not bool(rendering.get("pixelSnap", filter_name == "nearest"))
	var expected_position := size / 2.0 - anchor_position if centered else -anchor_position
	if sprite.texture_filter != expected_filter or sprite.texture_repeat != CanvasItem.TEXTURE_REPEAT_DISABLED or sprite.centered != centered or not sprite.position.is_equal_approx(expected_position):
		_fail("Installed scene rendering/anchor differs from the Pack.")
		return

func _blend_matches(sprite: Node2D, mode: String) -> bool:
	if mode == "multiply":
		var material := sprite.material as ShaderMaterial
		return material != null and material.shader != null and material.shader.resource_path.get_file() == "forge_alpha_multiply.gdshader"
	var material := sprite.material as CanvasItemMaterial
	return material != null and material.blend_mode == (1 if mode == "add" else 0)

func _verify_tile_set(path: String, terrain_helper: Dictionary = {}) -> void:
	var tile_set := ResourceLoader.load(path, "TileSet") as TileSet
	if tile_set == null or tile_set.get_source_count() == 0:
		_fail("Installed TileSet is empty: %s" % path)
		return
	for index in tile_set.get_source_count():
		var source := tile_set.get_source(tile_set.get_source_id(index)) as TileSetAtlasSource
		if source == null or source.texture == null or source.texture.resource_path.is_empty() or source.get_tiles_count() == 0:
			_fail("Installed TileSet has no external atlas or tiles: %s" % path)
			return
	if not terrain_helper.is_empty():
		if tile_set.get_source_count() != 1 or tile_set.get_terrain_sets_count() != 1 or tile_set.get_terrains_count(0) != 2 or tile_set.get_terrain_set_mode(0) != TileSet.TERRAIN_MODE_MATCH_CORNERS:
			_fail("Installed terrain definitions differ from the Pack: %s" % path)
			return
		if tile_set.get_custom_data_layers_count() != 2 or tile_set.get_custom_data_layer_name(0) != "forge_mask" or tile_set.get_custom_data_layer_name(1) != "forge_variant" or tile_set.get_custom_data_layer_type(0) != TYPE_INT or tile_set.get_custom_data_layer_type(1) != TYPE_INT:
			_fail("Installed terrain custom-data layers differ from the Pack: %s" % path)
			return
		var source := tile_set.get_source(0) as TileSetAtlasSource
		var masks := _terrain_mask_entries(terrain_helper)
		if source == null or source.get_tiles_count() != masks.size():
			_fail("Installed terrain atlas tile count differs from the Pack: %s" % path)
			return
		var corners := [TileSet.CELL_NEIGHBOR_TOP_LEFT_CORNER, TileSet.CELL_NEIGHBOR_TOP_RIGHT_CORNER, TileSet.CELL_NEIGHBOR_BOTTOM_RIGHT_CORNER, TileSet.CELL_NEIGHBOR_BOTTOM_LEFT_CORNER]
		for entry in masks:
			var mask := int(entry.get("mask", 0))
			var coords := Vector2i(int(entry.get("x", mask % 4)), int(entry.get("y", mask / 4)))
			if not source.has_tile(coords):
				_fail("Installed terrain atlas is missing a required tile: %s" % coords)
				return
			var data := source.get_tile_data(coords, 0)
			if data == null or data.terrain_set != 0 or int(data.get_custom_data("forge_mask")) != mask or int(data.get_custom_data("forge_variant")) != int(entry.get("variant", 0)):
				_fail("Installed terrain tile/custom-data differs from the Pack: %s" % coords)
				return
			for bit in 4:
				if data.get_terrain_peering_bit(corners[bit]) != (1 if mask & (1 << bit) else 0):
					_fail("Installed terrain peering bits differ from the Pack: %s" % coords)
					return
			var expected_polygons := 1 if terrain_helper.get("overlayCollision", "none") == "blocked" and mask != 0 else 0
			if data.get_collision_polygons_count(0) != expected_polygons:
				_fail("Installed terrain collision differs from the Pack: %s" % coords)
				return

func _verify_building_shapes(root: Node) -> void:
	for node_name in ["StaticBody2D", "EntranceInteraction"]:
		var body := root.get_node_or_null(node_name)
		if body == null or body.get_child_count() != 1:
			_fail("Installed building is missing its saved collision/interaction shape: %s" % node_name)
			return
		var collision := body.get_child(0) as CollisionShape2D
		if collision == null or not collision.shape is RectangleShape2D or collision.shape.size.x <= 0 or collision.shape.size.y <= 0:
			_fail("Installed building has an empty collision/interaction shape: %s" % node_name)
			return

func _load_scene(path: String) -> Node:
	var packed := ResourceLoader.load(path, "PackedScene") as PackedScene
	if packed == null:
		_fail("Installed scene cannot be loaded: %s" % path)
		return null
	var root := packed.instantiate()
	if root == null:
		_fail("Installed scene cannot be instantiated: %s" % path)
		return null
	return root

func _complete(target_res: String, asset_type: String) -> void:
	if _failed:
		return
	print("FORGE_INSTALL_RESULT " + JSON.stringify({
		"schemaVersion": "1", "status": "succeeded", "phase": _phase,
		"target": target_res, "assetType": asset_type,
	}))
	print("PASS Forge Godot %s: %s" % [_phase, target_res])
	quit(0)
