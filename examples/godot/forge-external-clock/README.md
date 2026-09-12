# SpriteFrames with an external clock

`forge_sprite_clock.gd` samples a native `SpriteFrames` resource using elapsed
seconds supplied by your simulation. Call `apply_at(sprite, animation, elapsed)`
after loading the installed `forge_sprite_frames.tres`. The sampler pauses the
node's internal playback and assigns its frame and progress. Keep `elapsed`
unchanged while the simulation is paused; reset it to zero when starting an action.

Frame durations are relative Godot weights divided by the animation FPS, so this
works with Forge's uniform FPS and `frameDurationsMs` exports. Looping actions wrap;
nonlooping actions hold their last frame with `finished: true`. Negative time clamps
to the first frame. Missing animations and invalid timing return an `error` field.
The example does not decide attacks, collision, actor mappings, or game time scale.

Run the isolated verifier from the repository root:

```bash
python3 scripts/test-godot-external-clock.py --godot /absolute/path/to/Godot
```

This verifies native resource timing and `AnimatedSprite2D` state. It does not
establish rendered artwork quality or device playback performance.
