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

For a one-shot attack tied to a simulation event, define the impact point in game
logic and map elapsed simulation time to the clip's anticipation/impact/recovery
segments. Read each duration from SpriteFrames; do not assume four equally long
frames. Changing attack speed can remap presentation time without changing source
frames or the authoritative damage event. Pause and seek by supplying the same
simulation time, including after rebuilding the view. A completed action holds
the last frame here; returning to an idle action is the caller's decision.

Keep one canvas/anchor transform for the whole action. Per-frame centering can
hide source drift and move the feet. Weapon-family routing and optional opponent
reflection belong in game code; reflection is not evidence of directional
animation support. Check these mappings in an isolated test scene before using
them in a production flow.

Run the isolated verifier from the repository root:

```bash
python3 scripts/test-godot-external-clock.py --godot /absolute/path/to/Godot
```

This verifies native resource timing and `AnimatedSprite2D` state. It does not
establish rendered artwork quality or device playback performance.
