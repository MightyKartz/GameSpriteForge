extends SceneTree

var failures: Array[String] = []

func check(ok: bool, label: String) -> void:
    if not ok:
        failures.append(label)
        printerr("FAIL: " + label)

func _initialize() -> void:
    call_deferred("verify")

func verify() -> void:
    var observations: Array[Dictionary] = []
    for id in ["music", "cue"]:
        var stream = load("res://addons/forge_assets/" + id + "/streams/" + id + ".res") as AudioStreamWAV
        check(stream != null, id + " native stream")
        if stream == null:
            continue
        var music: bool = id == "music"
        check(stream.format == AudioStreamWAV.FORMAT_16_BITS, id + " PCM16")
        check(stream.mix_rate == (22050 if music else 48000), id + " rate")
        check(stream.stereo == (not music), id + " channels")
        check(absf(stream.get_length() - 1.0) < 0.00001, id + " duration")
        check(stream.loop_mode == (AudioStreamWAV.LOOP_FORWARD if music else AudioStreamWAV.LOOP_DISABLED), id + " loop")
        check(stream.loop_begin == 0 and stream.loop_end == (22050 if music else 0), id + " loop range")
        # Advance Godot's native stream mixer explicitly. A headless CI host may
        # not advance its device clock with SceneTree's wall-clock timers.
        var playback := stream.instantiate_playback()
        playback.start()
        var requested := int(AudioServer.get_mix_rate() * 1.5)
        var mixed := playback.mix_audio(1.0, requested)
        var peak := 0.0
        var tail_peak := 0.0
        for i in range(mixed.size()):
            var value := maxf(absf(mixed[i].x), absf(mixed[i].y))
            peak = maxf(peak, value)
            if i >= int(requested * 0.8):
                tail_peak = maxf(tail_peak, value)
        check(peak > 0.01, id + " native mixed samples")
        if music:
            check(mixed.size() == requested and playback.is_playing(), "music crosses loop boundary")
            check(tail_peak > 0.01, "music audible samples beyond first duration")
        else:
            check(mixed.size() < requested and not playback.is_playing(), "one-shot mixer finished")
        observations.append({"id": id, "mixRate": AudioServer.get_mix_rate(),
            "requestedFrames": requested, "mixedFrames": mixed.size(), "peak": peak,
            "tailPeak": tail_peak, "playingAfterMix": playback.is_playing()})
        playback.stop()
        playback = null
        stream = null
    print("M3_NATIVE_MIX " + JSON.stringify(observations))
    await create_timer(0.1).timeout
    var usage = JSON.parse_string(FileAccess.get_file_as_string("res://addons/forge_assets/props/forge_usage.json"))
    check(usage != null, "static usage")
    if usage != null:
        var scene = load(usage.scenePath + "/prop.tscn") as PackedScene
        check(scene != null, "prop scene")
        if scene != null:
            var node = scene.instantiate()
            var sprite = node.get_node("Sprite2D") as Sprite2D
            check(sprite != null and sprite.texture != null, "prop texture")
            if sprite != null:
                check(sprite.position == -Vector2(32, 60), "prop ground anchor")
                check(sprite.texture_filter == CanvasItem.TEXTURE_FILTER_NEAREST, "prop nearest")
                check(sprite.texture.get_size() == Vector2(64, 64), "prop canvas")
            node.free()
    if failures.is_empty():
        print("M3_NATIVE_PASS")
    call_deferred("quit", 0 if failures.is_empty() else 1)
