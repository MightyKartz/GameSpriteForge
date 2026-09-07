extends Node2D
## Review existing installed Packs. This scene never edits SpriteFrames or frame pixels.
## Run: godot --path PROJECT -- --qa-auto-quit
## Baseline/calibrated pairs share native playback timing, not a resampled common cycle.

const CONTRACT_PATH := "res://runtime-contracts.json"
const REPORT_PATH := "res://qa-output/runtime-report.json"
const DIRECTIONS := ["right", "down", "up"]
const SEQUENCE := ["right", "down", "up", "right"]
const SEGMENT_SECONDS := 2.3
const START_POSITION := Vector2(480.0, 670.0)
const EPSILON := 0.002
const DURATION_TOLERANCE_MS := 0.05

var contracts: Dictionary = {}
var common: Dictionary = {}
var directions: Dictionary = {}
var body: CharacterBody2D
var collision: CollisionShape2D
var instances: Dictionary = {}
var sprites: Dictionary = {}
var samples: Dictionary = {}
var resources: Dictionary = {}
var failures: Array = []
var transitions: Array = []
var movement_segments: Array = []
var screenshots: Array = []
var active_direction := "right"
var calibrated := true
var paused := false
var automatic := true
var ready_for_review := false
var finalized := false
var finalizing := false
var sequence_index := 0
var animation_time := 0.0
var maximum_process_delta := 0.0
var segment_elapsed := 0.0
var segment_start := START_POSITION
var segment_distance := 0.0
var segment_move_calls := 0
var segment_velocity_error := 0.0
var segment_visits: Dictionary = {}
var segment_cycles: Array = []
var segment_last_loop := 0.0
var max_sync_frame_error := 0
var max_sync_progress_error := 0.0
var max_anchor_body_error := 0.0
var up_capture_requested := false
var report_runtime_passed := false
var manual_motion := false


func _ready() -> void:
	get_tree().auto_accept_quit = false
	contracts = _load_json(CONTRACT_PATH)
	common = contracts.get("common", {})
	directions = contracts.get("directions", {})
	if not _valid_contract():
		_fatal("Missing or invalid common/directions runtime contract")
		return
	_create_body()
	if not _instantiate_assets():
		_fatal("Unable to validate the installed animation resources")
		return
	ready_for_review = true
	_switch_direction("right", "automatic_start")
	queue_redraw()


func _valid_contract() -> bool:
	if directions.size() != 3 or not common.get("collision") is Dictionary:
		return false
	if float(common.get("visualScale", 0.0)) <= 0.0:
		return false
	if float(common.get("movementSpeedPixelsPerSecond", 0.0)) <= 0.0:
		return false
	for key in ["width", "height", "offsetX", "offsetY"]:
		if not common["collision"].has(key):
			return false
	if float(common["collision"]["width"]) <= 0.0 or float(common["collision"]["height"]) <= 0.0:
		return false
	for direction in DIRECTIONS:
		if not directions.get(direction) is Dictionary:
			return false
		for key in ["scene", "animation", "frameCount", "frameDurationsMs", "sourceNativePivotPx", "sourceCycleDurationMs", "calibrationScale", "stableScaleMedianPx"]:
			if not directions[direction].has(key):
				return false
		if float(directions[direction]["calibrationScale"]) <= 0.0:
			return false
	return true


func _create_body() -> void:
	body = CharacterBody2D.new()
	body.name = "MovingCharacter"
	body.motion_mode = CharacterBody2D.MOTION_MODE_FLOATING
	body.position = START_POSITION
	add_child(body)
	collision = CollisionShape2D.new()
	collision.name = "SharedCollision"
	var rectangle := RectangleShape2D.new()
	var spec: Dictionary = common["collision"]
	rectangle.size = Vector2(float(spec["width"]), float(spec["height"]))
	collision.shape = rectangle
	collision.position = Vector2(float(spec["offsetX"]), float(spec["offsetY"]))
	body.add_child(collision)


func _instantiate_assets() -> bool:
	var columns := [152.0, 360.0, 568.0]
	for index in DIRECTIONS.size():
		var direction: String = DIRECTIONS[index]
		var config: Dictionary = directions[direction]
		var packed := load(String(config["scene"])) as PackedScene
		if packed == null:
			_fail("installed_scene", "Could not load " + direction)
			return false
		for role in ["baseline", "calibrated", "moving"]:
			var key: String = role + "/" + direction
			var instance := packed.instantiate() as Node2D
			if instance == null:
				return false
			instance.name = role + "_" + direction
			var sprite := instance.get_node_or_null("AnimatedSprite2D") as AnimatedSprite2D
			if sprite == null or sprite.sprite_frames == null:
				instance.free()
				_fail("installed_sprite", "Missing AnimatedSprite2D in " + direction)
				return false
			var animation := StringName(config["animation"])
			if not sprite.sprite_frames.has_animation(animation):
				instance.free()
				return false
			if role == "moving":
				body.add_child(instance)
				instance.visible = false
			else:
				instance.position = Vector2(columns[index] + (720.0 if role == "calibrated" else 0.0), 340.0)
				add_child(instance)
			instance.scale = Vector2.ONE * _expected_scale(role, direction)
			sprite.speed_scale = 1.0
			sprite.stop()
			sprite.play(animation)
			if role == "moving":
				sprite.pause()
			instances[key] = instance
			sprites[key] = sprite
			samples[key] = {
				"sampleCount": 0,
				"scaleMin": instance.scale.x,
				"scaleMax": instance.scale.x,
				"scaleDeviationMax": 0.0,
				"nativePosition": _vector(sprite.position),
				"nativeScale": _vector(sprite.scale),
				"nativeOffset": _vector(sprite.offset),
				"sceneLocalPosition": _vector(instance.position),
				"nativeTransformDeviationMax": 0.0,
				"scenePositionDeviationMax": 0.0,
				"speedScaleDeviationMax": 0.0,
				"frameVisits": {},
				"observedCycleDurationsMs": [],
				"lastLoopAtSeconds": 0.0,
			}
			sprite.frame_changed.connect(_on_frame_changed.bind(key))
			sprite.animation_looped.connect(_on_animation_looped.bind(key))
			_on_frame_changed(key)
			if role == "baseline":
				resources[direction] = _inspect_resource(sprite, config)
				if not resources[direction]["matchesSourceContract"]:
					_fail("native_resource_contract", direction + " frames, durations, loop, or pivot do not match the source")
	return true


func _expected_scale(role: String, direction: String) -> float:
	var multiplier := 1.0
	if role == "calibrated" or (role == "moving" and calibrated):
		multiplier = float(directions[direction]["calibrationScale"])
	return float(common["visualScale"]) * multiplier


func _inspect_resource(sprite: AnimatedSprite2D, config: Dictionary) -> Dictionary:
	var animation := StringName(config["animation"])
	var frames := sprite.sprite_frames
	var count := frames.get_frame_count(animation)
	var fps := frames.get_animation_speed(animation)
	var durations: Array = []
	var sizes: Array = []
	var texture_records: Array = []
	var duration_sum := 0.0
	var match_contract := count == int(config["frameCount"]) and count > 0 and fps > 0.0
	var expected_durations: Array = config["frameDurationsMs"]
	match_contract = match_contract and expected_durations.size() == count
	var maximum_pivot_error := 0.0
	for index in count:
		var duration_ms := frames.get_frame_duration(animation, index) / maxf(fps, 0.000001) * 1000.0
		durations.append(duration_ms)
		duration_sum += duration_ms
		if index >= expected_durations.size() or duration_ms <= 0.0:
			match_contract = false
		elif absf(duration_ms - float(expected_durations[index])) > DURATION_TOLERANCE_MS:
			match_contract = false
		var texture := frames.get_frame_texture(animation, index)
		if texture == null:
			match_contract = false
			continue
		sizes.append(_vector(texture.get_size()))
		var texture_record := {
			"frameIndex": index, "textureClass": texture.get_class(),
			"textureResourcePath": texture.resource_path, "sizePx": _vector(texture.get_size()),
		}
		if texture is AtlasTexture:
			var atlas_texture := texture as AtlasTexture
			texture_record["atlasResourcePath"] = atlas_texture.atlas.resource_path if atlas_texture.atlas != null else ""
			texture_record["region"] = _rect(atlas_texture.region)
			texture_record["margin"] = _rect(atlas_texture.margin)
			texture_record["filterClip"] = atlas_texture.filter_clip
		texture_records.append(texture_record)
		var pivot_local := _texture_pivot_in_sprite(sprite, texture, config)
		maximum_pivot_error = maxf(maximum_pivot_error, (sprite.transform * pivot_local).length())
	match_contract = match_contract and maximum_pivot_error <= EPSILON
	match_contract = match_contract and absf(duration_sum - float(config["sourceCycleDurationMs"])) <= DURATION_TOLERANCE_MS
	match_contract = match_contract and frames.get_animation_loop(animation)
	return {
		"scene": config["scene"], "animation": String(animation),
		"frameCount": count, "animationFramesPerSecond": fps,
		"frameDurationsMs": durations, "frameTextureSizesPx": sizes,
		"frameTextures": texture_records,
		"nativeCycleDurationMs": duration_sum,
		"sourceExpectedCycleDurationMs": config["sourceCycleDurationMs"],
		"loop": frames.get_animation_loop(animation),
		"maximumNativePivotErrorPx": maximum_pivot_error,
		"nativeSpritePosition": _vector(sprite.position),
		"nativeSpriteOffset": _vector(sprite.offset), "nativeSpriteCentered": sprite.centered,
		"sourceNativePivotPx": config["sourceNativePivotPx"],
		"matchesSourceContract": match_contract,
	}


func _texture_pivot_in_sprite(sprite: AnimatedSprite2D, texture: Texture2D, config: Dictionary) -> Vector2:
	var pivot: Dictionary = config["sourceNativePivotPx"]
	var result := Vector2(float(pivot["x"]), float(pivot["y"])) + sprite.offset
	if sprite.centered:
		result -= texture.get_size() * 0.5
	return result


func _world_anchor(direction: String) -> Vector2:
	var sprite: AnimatedSprite2D = sprites["moving/" + direction]
	var texture := sprite.sprite_frames.get_frame_texture(sprite.animation, sprite.frame)
	return sprite.to_global(_texture_pivot_in_sprite(sprite, texture, directions[direction]))


func _process(delta: float) -> void:
	if not ready_for_review:
		return
	if not paused:
		animation_time += delta
		maximum_process_delta = maxf(maximum_process_delta, delta)
		_sample_runtime()
	queue_redraw()
	if automatic and sequence_index == 2 and segment_elapsed >= 1.1 and not up_capture_requested:
		up_capture_requested = true
		_capture("up-phase.png")


func _physics_process(delta: float) -> void:
	if not ready_for_review or paused or finalizing:
		return
	var desired_direction := active_direction
	var moving := automatic
	if not automatic:
		moving = false
		for pair in [[KEY_RIGHT, "right"], [KEY_DOWN, "down"], [KEY_UP, "up"]]:
			if Input.is_physical_key_pressed(pair[0]):
				desired_direction = pair[1]
				moving = true
				break
		if desired_direction != active_direction:
			_switch_direction(desired_direction, "manual_direction")
		var active_sprite: AnimatedSprite2D = sprites["moving/" + active_direction]
		if moving and not manual_motion:
			active_sprite.play()
		elif not moving and manual_motion:
			active_sprite.pause()
		manual_motion = moving
	if not moving:
		body.velocity = Vector2.ZERO
		return
	var vector := _direction_vector(active_direction)
	var speed := float(common["movementSpeedPixelsPerSecond"])
	body.velocity = vector * speed
	var before := body.global_position
	body.move_and_slide()
	var displacement := body.global_position - before
	if automatic:
		segment_elapsed += delta
		segment_move_calls += 1
		segment_distance += displacement.length()
		segment_velocity_error = maxf(segment_velocity_error, (displacement / delta - vector * speed).length())
		if segment_elapsed + 0.000001 >= SEGMENT_SECONDS:
			_finish_segment()
			sequence_index += 1
			if sequence_index < SEQUENCE.size():
				_switch_direction(SEQUENCE[sequence_index], "automatic_direction")
			else:
				automatic = false
				(sprites["moving/" + active_direction] as AnimatedSprite2D).pause()
				body.velocity = Vector2.ZERO
				_finalize(false)


func _switch_direction(direction: String, reason: String) -> void:
	var old_direction := active_direction
	var before_body := body.global_position
	var before_anchor := _world_anchor(old_direction)
	for item in DIRECTIONS:
		(instances["moving/" + item] as Node2D).visible = false
		(sprites["moving/" + item] as AnimatedSprite2D).pause()
	active_direction = direction
	var instance: Node2D = instances["moving/" + direction]
	instance.visible = true
	var sprite: AnimatedSprite2D = sprites["moving/" + direction]
	# Reset playback only. Imported sprite position/pivot and frame textures stay native.
	sprite.stop()
	sprite.play(StringName(directions[direction]["animation"]))
	if paused:
		sprite.pause()
	segment_elapsed = 0.0
	segment_distance = 0.0
	segment_move_calls = 0
	segment_velocity_error = 0.0
	segment_start = body.global_position
	segment_visits = {0: true}
	segment_cycles = []
	segment_last_loop = animation_time
	samples["moving/" + direction]["lastLoopAtSeconds"] = animation_time
	var after_anchor := _world_anchor(direction)
	transitions.append({
		"reason": reason, "from": old_direction, "to": direction,
		"beforeWorldPivot": _vector(before_anchor), "afterWorldPivot": _vector(after_anchor),
		"worldPivotDeltaPx": before_anchor.distance_to(after_anchor),
		"bodyPositionDeltaPx": before_body.distance_to(body.global_position),
		"actualVisualScale": _vector(instance.scale),
		"collision": _collision_measurement(),
		"movementSpeedPixelsPerSecond": float(common["movementSpeedPixelsPerSecond"]),
	})


func _sample_runtime() -> void:
	for key in sprites:
		var sprite: AnimatedSprite2D = sprites[key]
		var instance: Node2D = instances[key]
		var measurement: Dictionary = samples[key]
		var parts := String(key).split("/")
		var expected := _expected_scale(parts[0], parts[1])
		measurement["sampleCount"] += 1
		measurement["scaleMin"] = minf(float(measurement["scaleMin"]), instance.scale.x)
		measurement["scaleMax"] = maxf(float(measurement["scaleMax"]), instance.scale.x)
		measurement["scaleDeviationMax"] = maxf(float(measurement["scaleDeviationMax"]), instance.scale.distance_to(Vector2.ONE * expected))
		var native_error := sprite.position.distance_to(_as_vector(measurement["nativePosition"]))
		native_error += sprite.scale.distance_to(_as_vector(measurement["nativeScale"]))
		native_error += sprite.offset.distance_to(_as_vector(measurement["nativeOffset"]))
		measurement["nativeTransformDeviationMax"] = maxf(float(measurement["nativeTransformDeviationMax"]), native_error)
		measurement["scenePositionDeviationMax"] = maxf(float(measurement["scenePositionDeviationMax"]), instance.position.distance_to(_as_vector(measurement["sceneLocalPosition"])))
		measurement["speedScaleDeviationMax"] = maxf(float(measurement["speedScaleDeviationMax"]), absf(sprite.speed_scale - 1.0))
		if sprite.flip_h or sprite.flip_v:
			_fail("no_mirroring", String(key) + " has a flipped sprite")
	for direction in DIRECTIONS:
		var baseline: AnimatedSprite2D = sprites["baseline/" + direction]
		var calibrated_sprite: AnimatedSprite2D = sprites["calibrated/" + direction]
		max_sync_frame_error = maxi(max_sync_frame_error, absi(baseline.frame - calibrated_sprite.frame))
		max_sync_progress_error = maxf(max_sync_progress_error, absf(baseline.frame_progress - calibrated_sprite.frame_progress))
	max_anchor_body_error = maxf(max_anchor_body_error, _world_anchor(active_direction).distance_to(body.global_position))
	var actual_collision := _collision_measurement()
	if not actual_collision["matchesCommonContract"]:
		_fail("shared_collision", "Actual body collision differs from the common contract")


func _on_frame_changed(key: String) -> void:
	if not sprites.has(key):
		return
	var sprite: AnimatedSprite2D = sprites[key]
	samples[key]["frameVisits"][sprite.frame] = true
	if key == "moving/" + active_direction and automatic:
		segment_visits[sprite.frame] = true


func _on_animation_looped(key: String) -> void:
	var measurement: Dictionary = samples[key]
	measurement["observedCycleDurationsMs"].append((animation_time - float(measurement["lastLoopAtSeconds"])) * 1000.0)
	measurement["lastLoopAtSeconds"] = animation_time
	if key == "moving/" + active_direction and automatic:
		segment_cycles.append((animation_time - segment_last_loop) * 1000.0)
		segment_last_loop = animation_time


func _finish_segment() -> void:
	var displacement := body.global_position - segment_start
	var expected := _direction_vector(active_direction) * float(common["movementSpeedPixelsPerSecond"]) * segment_elapsed
	var frames: Array = segment_visits.keys()
	frames.sort()
	movement_segments.append({
		"sequenceIndex": sequence_index, "direction": active_direction,
		"durationSeconds": segment_elapsed, "moveAndSlideCallCount": segment_move_calls,
		"startWorldPosition": _vector(segment_start), "endWorldPosition": _vector(body.global_position),
		"actualDisplacementPx": _vector(displacement), "expectedDisplacementPx": _vector(expected),
		"displacementErrorPx": displacement.distance_to(expected),
		"actualTravelDistancePx": segment_distance,
		"actualMeanSpeedPixelsPerSecond": segment_distance / maxf(segment_elapsed, 0.000001),
		"maximumVelocityErrorPixelsPerSecond": segment_velocity_error,
		"actualVisualScale": _vector((instances["moving/" + active_direction] as Node2D).scale),
		"calibrationEnabled": calibrated, "collision": _collision_measurement(),
		"visitedFrames": frames,
		"allNativeFramesVisited": frames.size() == int(resources[active_direction]["frameCount"]),
		"observedCycleDurationsMs": segment_cycles.duplicate(),
		"resourceCycleDurationMs": resources[active_direction]["nativeCycleDurationMs"],
	})


func _collision_measurement() -> Dictionary:
	var spec: Dictionary = common["collision"]
	var rectangle := collision.shape as RectangleShape2D
	var expected_size := Vector2(float(spec["width"]), float(spec["height"]))
	var expected_offset := Vector2(float(spec["offsetX"]), float(spec["offsetY"]))
	var matches := rectangle != null and not collision.disabled
	if rectangle != null:
		matches = matches and rectangle.size.distance_to(expected_size) <= EPSILON
	matches = matches and collision.position.distance_to(expected_offset) <= EPSILON
	matches = matches and collision.scale.distance_to(Vector2.ONE) <= EPSILON and body.scale.distance_to(Vector2.ONE) <= EPSILON
	return {
		"shapeInstanceId": collision.shape.get_instance_id(),
		"width": rectangle.size.x if rectangle != null else 0.0,
		"height": rectangle.size.y if rectangle != null else 0.0,
		"offsetX": collision.position.x, "offsetY": collision.position.y,
		"bodyScale": _vector(body.scale), "shapeScale": _vector(collision.scale),
		"matchesCommonContract": matches,
	}


func _unhandled_key_input(event: InputEvent) -> void:
	if not ready_for_review or finalizing or not event is InputEventKey:
		return
	if not event.pressed or event.echo:
		return
	if event.physical_keycode == KEY_SPACE:
		paused = not paused
		for key in sprites:
			var sprite: AnimatedSprite2D = sprites[key]
			if paused:
				sprite.pause()
			elif not String(key).begins_with("moving/") or (key == "moving/" + active_direction and (automatic or manual_motion)):
				sprite.play()
	elif event.physical_keycode in [KEY_RIGHT, KEY_DOWN, KEY_UP]:
		automatic = false
		finalized = false
	elif event.physical_keycode == KEY_B:
		calibrated = not calibrated
		# An explicit mode change applies one fixed multiplier per direction.
		for direction in DIRECTIONS:
			(instances["moving/" + direction] as Node2D).scale = Vector2.ONE * _expected_scale("moving", direction)
	elif event.physical_keycode == KEY_A:
		_restart_demo()
	elif event.physical_keycode in [KEY_ESCAPE, KEY_Q]:
		_finalize(true)


func _restart_demo() -> void:
	body.position = START_POSITION
	sequence_index = 0
	movement_segments.clear()
	transitions.clear()
	automatic = true
	manual_motion = false
	finalized = false
	_switch_direction("right", "automatic_restart")


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		_finalize(true)


func _finalize(force_quit: bool) -> void:
	if finalizing:
		return
	finalizing = true
	_sample_runtime()
	_validate_measurements()
	await _capture("review.png")
	var source_geometry := {}
	var calibration_metrics := {}
	var reference_median := float(directions["right"]["stableScaleMedianPx"])
	for direction in DIRECTIONS:
		var config: Dictionary = directions[direction]
		source_geometry[direction] = {
			"stableScaleMedianPx": config["stableScaleMedianPx"],
			"stableScaleRelativeError": config.get("stableScaleRelativeError"),
			"stableScaleGatePassed": config.get("stableScaleGatePassed"),
			"verdict": "source evidence preserved; runtime calibration does not replace source geometry assessment",
		}
		var multiplied := float(config["stableScaleMedianPx"]) * float(config["calibrationScale"])
		calibration_metrics[direction] = {
			"fixedMultiplier": config["calibrationScale"], "commonVisualScale": common["visualScale"],
			"mathematicalMedianPx": multiplied,
			"mathematicalRelativeErrorFromRight": absf(multiplied - reference_median) / reference_median,
			"assessment": "mathematical calibration only", "visualReview": "pending",
		}
	report_runtime_passed = failures.is_empty()
	var report := {
		"schemaVersion": "1", "profile": "existing-directional-walk-runtime-review@1.0.0",
		"engine": Engine.get_version_info(), "displayServer": DisplayServer.get_name(),
		"runtimePassed": report_runtime_passed, "visualReview": "pending",
		"verdict": "runtime_checks_passed_visual_review_pending" if report_runtime_passed else "runtime_checks_failed",
		"sourceGeometryAssessment": source_geometry, "mathematicalCalibration": calibration_metrics,
		"commonRuntimeContract": common, "nativeResources": resources,
		"sourceProviderRequestCountThisOperation": contracts.get("providerRequestCountThisOperation"),
		"runtimeProviderRequestCount": 0,
		"sequence": SEQUENCE, "segmentDurationSeconds": SEGMENT_SECONDS,
		"movementSegments": movement_segments, "transitionMeasurements": transitions,
		"instanceMeasurements": samples,
		"maximumSourceAnchorToBodyDistancePx": max_anchor_body_error,
		"pairedPreviewMaximumFrameDifference": max_sync_frame_error,
		"pairedPreviewMaximumProgressDifference": max_sync_progress_error,
		"cycleMeasurementToleranceMs": _cycle_tolerance_ms(),
		"screenshots": screenshots,
		"screenshotStatus": "skipped_headless" if DisplayServer.get_name() == "headless" else "captured" if failures.is_empty() else "see_failures",
		"failures": failures,
		"controls": "Right/Down/Up: move; Space: pause; B: lower character calibration; A: replay demo; Q/Escape: report and quit",
		"scope": "Existing frame textures, native frame durations and native pivots retained. Fixed outer scale only. No left mirroring or synthetic idle. Runtime evidence does not establish visual acceptance.",
	}
	if not _write_report(report):
		get_tree().quit(1)
		return
	print("RUNTIME_REVIEW ", "PASS" if report_runtime_passed else "FAIL", " ", ProjectSettings.globalize_path(REPORT_PATH))
	finalized = true
	finalizing = false
	if force_quit or "--qa-auto-quit" in OS.get_cmdline_user_args():
		get_tree().quit(0 if report_runtime_passed else 1)


func _validate_measurements() -> void:
	if movement_segments.size() != SEQUENCE.size():
		_fail("complete_sequence", "Automatic four-segment movement demonstration is incomplete")
	var total_frames := 0
	for direction in DIRECTIONS:
		total_frames += int(resources[direction]["frameCount"])
		var rechecked := _inspect_resource(sprites["baseline/" + direction], directions[direction])
		if not rechecked["matchesSourceContract"]:
			_fail("native_resource_contract", direction + " frames, durations, loop, or pivot do not match the source")
		if rechecked != resources[direction]:
			_fail("resource_texture_regions_unchanged", direction + " resource textures, regions, or native transforms changed during playback")
	if total_frames != 52:
		_fail("52_existing_frames", "Actual SpriteFrames total is %d; expected 52" % total_frames)
	for segment in movement_segments:
		if not segment["allNativeFramesVisited"] or segment["observedCycleDurationsMs"].is_empty():
			_fail("complete_native_cycle", String(segment["direction"]) + " did not display all source frames and finish a loop")
		if float(segment["displacementErrorPx"]) > 0.02 or int(segment["moveAndSlideCallCount"]) < 1:
			_fail("actual_movement", String(segment["direction"]) + " actual move_and_slide displacement does not match requested movement")
		if float(segment["maximumVelocityErrorPixelsPerSecond"]) > 0.05:
			_fail("actual_speed", String(segment["direction"]) + " speed differs from common speed")
		for duration in segment["observedCycleDurationsMs"]:
			if absf(float(duration) - float(segment["resourceCycleDurationMs"])) > _cycle_tolerance_ms():
				_fail("native_cycle_timing", String(segment["direction"]) + " observed cycle differs from SpriteFrames duration")
	for transition in transitions:
		if float(transition["worldPivotDeltaPx"]) > EPSILON or float(transition["bodyPositionDeltaPx"]) > EPSILON:
			_fail("switch_anchor", "Direction switch changed the actual source pivot world position")
	for key in samples:
		var sample: Dictionary = samples[key]
		if int(sample["sampleCount"]) < 1:
			_fail("runtime_sampling", String(key) + " was not sampled")
		for metric in ["scaleDeviationMax", "nativeTransformDeviationMax", "scenePositionDeviationMax", "speedScaleDeviationMax"]:
			if float(sample[metric]) > EPSILON:
				_fail("fixed_native_transform", String(key) + " changed " + metric)
		if not String(key).begins_with("moving/"):
			var direction := String(key).split("/")[1]
			if sample["frameVisits"].size() != int(resources[direction]["frameCount"]):
				_fail("preview_frame_coverage", String(key) + " missed source frames")
	if max_sync_frame_error != 0 or max_sync_progress_error > EPSILON:
		_fail("paired_preview_sync", "Baseline and calibrated native playback pairs diverged")
	if max_anchor_body_error > EPSILON:
		_fail("native_pivot_body_anchor", "Imported frame source pivot differs from CharacterBody2D position")


func _cycle_tolerance_ms() -> float:
	# Signal observation is quantized to process frames; cap the permitted uncertainty.
	return minf(100.0, maxf(35.0, maximum_process_delta * 2000.0))


func _capture(filename: String) -> void:
	if DisplayServer.get_name() == "headless":
		if screenshots.is_empty():
			screenshots.append({"status": "skipped", "reason": "headless display has no rendered viewport"})
		return
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path("res://qa-output"))
	await get_tree().process_frame
	await RenderingServer.frame_post_draw
	var path := "res://qa-output/" + filename
	var screenshot := get_viewport().get_texture().get_image()
	if screenshot == null or screenshot.is_empty():
		_fail("rendered_screenshot", "Viewport image is empty")
	elif screenshot.save_png(path) != OK:
		_fail("rendered_screenshot", "Could not save " + path)
	else:
		screenshots.append({"path": path, "width": screenshot.get_width(), "height": screenshot.get_height(), "direction": active_direction})


func _write_report(report: Dictionary) -> bool:
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path("res://qa-output"))
	var file := FileAccess.open(REPORT_PATH, FileAccess.WRITE)
	if file == null:
		push_error("Cannot write " + REPORT_PATH)
		return false
	file.store_string(JSON.stringify(report, "\t"))
	file.close()
	return true


func _fatal(message: String) -> void:
	_fail("startup", message)
	_write_report({"schemaVersion": "1", "runtimePassed": false, "visualReview": "pending", "failures": failures})
	get_tree().quit(1)


func _fail(gate: String, message: String) -> void:
	var item := {"gate": gate, "message": message}
	if item not in failures:
		failures.append(item)
		push_error(gate + ": " + message)


func _direction_vector(direction: String) -> Vector2:
	return {"right": Vector2.RIGHT, "down": Vector2.DOWN, "up": Vector2.UP}[direction]


func _load_json(path: String) -> Dictionary:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		return {}
	var parsed = JSON.parse_string(file.get_as_text())
	file.close()
	return parsed if parsed is Dictionary else {}


func _vector(value: Vector2) -> Dictionary:
	return {"x": value.x, "y": value.y}


func _as_vector(value: Dictionary) -> Vector2:
	return Vector2(float(value["x"]), float(value["y"]))


func _rect(value: Rect2) -> Dictionary:
	return {"x": value.position.x, "y": value.position.y, "width": value.size.x, "height": value.size.y}


func _draw() -> void:
	draw_rect(Rect2(0, 0, 1440, 900), Color("101821"))
	if not ready_for_review:
		return
	var font := ThemeDB.fallback_font
	_text(font, Vector2(30, 27), "FORGE / Existing directional walks", 23, "f0f5fa")
	_text(font, Vector2(820, 26), "52 original frames  /  native timing  /  visual review pending", 15, "bdcbd9")
	for role_index in 2:
		var x := 16.0 + role_index * 720.0
		draw_style_box(_panel(Color("172432")), Rect2(x, 58, 688, 347))
		_text(font, Vector2(x + 20, 51), "BASELINE / shared scale" if role_index == 0 else "CALIBRATED / fixed direction multiplier", 15, "93a9bf")
		draw_line(Vector2(x + 20, 340), Vector2(x + 668, 340), Color("4d91aa"), 1.0)
		for index in DIRECTIONS.size():
			var direction: String = DIRECTIONS[index]
			var anchor_x := 152.0 + index * 208.0 + role_index * 720.0
			_draw_anchor(Vector2(anchor_x, 340), Color("f2c879"))
			var role := "baseline" if role_index == 0 else "calibrated"
			var sprite: AnimatedSprite2D = sprites[role + "/" + direction]
			var multiplier := 1.0 if role_index == 0 else float(directions[direction]["calibrationScale"])
			_text(font, Vector2(anchor_x - 74, 365), "%s  x%.5f" % [direction.to_upper(), multiplier], 14, "f0f5fa")
			_text(font, Vector2(anchor_x - 84, 388), "%02d/%02d  |  %.0f ms native" % [sprite.frame + 1, int(resources[direction]["frameCount"]), float(resources[direction]["nativeCycleDurationMs"])], 12, "93a9bf")
	draw_line(Vector2(20, 421), Vector2(1420, 421), Color("344455"), 1.0)
	_text(font, Vector2(30, 453), "MOVE AND SWITCH", 20, "f0f5fa")
	_text(font, Vector2(30, 478), "CharacterBody2D / move_and_slide", 13, "93a9bf")
	_text(font, Vector2(30, 499), "Right > Down > Up > Right / 2.3 s each", 13, "93a9bf")
	for x in range(32, 1000, 40):
		draw_line(Vector2(x, 502), Vector2(x, 854), Color(0.2, 0.3, 0.4, 0.2))
	for y in range(514, 855, 40):
		draw_line(Vector2(32, y), Vector2(990, y), Color(0.2, 0.3, 0.4, 0.2))
	var route := PackedVector2Array([START_POSITION, START_POSITION + Vector2(184, 0), START_POSITION + Vector2(184, 184), START_POSITION + Vector2(184, 0), START_POSITION + Vector2(368, 0)])
	draw_polyline(route, Color("425770"), 2.0)
	var rectangle := collision.shape as RectangleShape2D
	var rect := Rect2(body.position + collision.position - rectangle.size * 0.5, rectangle.size)
	draw_rect(rect, Color(0.3, 0.86, 0.64, 0.1))
	draw_rect(rect, Color("65d5a5"), false, 1.5)
	_draw_anchor(body.position, Color("f2c879"))
	draw_style_box(_panel(Color("172432")), Rect2(1020, 441, 400, 412))
	var mode := "AUTO" if automatic else "MANUAL"
	if paused:
		mode += " / PAUSED"
	_text(font, Vector2(1040, 475), mode + "   " + active_direction.to_upper(), 20, "f0f5fa")
	var lines := [
		"Lower character: " + ("calibrated" if calibrated else "baseline"),
		"Fixed outer scale: %.5f" % (instances["moving/" + active_direction] as Node2D).scale.x,
		"Movement speed: %.0f px/s" % float(common["movementSpeedPixelsPerSecond"]),
		"Collision: %.0f x %.0f px" % [rectangle.size.x, rectangle.size.y],
		"Position: (%.1f, %.1f)" % [body.position.x, body.position.y],
		"Native anchor error: %.5f px" % max_anchor_body_error,
		"Completed segments: %d / 4" % movement_segments.size(),
		"",
		"Arrows RIGHT / DOWN / UP: move",
		"SPACE: pause     B: baseline / calibration",
		"A: replay auto    Q / ESC: report + quit",
		"",
		"Visual acceptance: PENDING",
		"Scale match is mathematical calibration.",
		"Source geometry verdicts remain unchanged.",
	]
	for index in lines.size():
		_text(font, Vector2(1040, 507 + index * 22), lines[index], 13, "d8b578" if index >= 12 else "b7c8d9")
	_text(font, Vector2(30, 883), "Gold: original source pivot   /   Green: shared collision   /   Native loops differ in duration; each baseline/calibrated pair stays synchronized.", 13, "93a9bf")


func _text(font: Font, position: Vector2, value: String, size: int, color: String) -> void:
	draw_string(font, position, value, HORIZONTAL_ALIGNMENT_LEFT, -1, size, Color(color))


func _draw_anchor(position: Vector2, color: Color) -> void:
	draw_line(position - Vector2(8, 0), position + Vector2(8, 0), color, 2.0)
	draw_line(position - Vector2(0, 8), position + Vector2(0, 8), color, 2.0)
	draw_circle(position, 3.0, color)


func _panel(color: Color) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = color
	box.set_corner_radius_all(10)
	return box
