# Versioned layered runtimes

`layered-player-v1.gd` and `alpha-multiply-v1.gdshader` are the trusted runtime snapshots for
`godot-layered@1.0.0`. Freeze this file when the profile ships. Subsequent behavior
changes require a new profile and snapshot while retaining V1 validation and
scene generation. Do not replace a Pack's script with the current generic
`forge_layered_player.gd` or trust a script solely because a Pack declares its
hash. The generic controller may continue to evolve independently.
