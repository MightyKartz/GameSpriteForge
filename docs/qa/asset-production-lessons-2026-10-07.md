# Asset-production guidance verification

2026-10-07. This change adds guidance and inline requests for one-shot effects
and scene music, plus complete-silhouette and tool-identity advice. Processing,
Pack formats, commands and consumer locks are unchanged. The embedded source
guide is updated; already published v0.7.5 binaries and separately installed
skills are not changed by this checkout edit.

## Verification

- Rebuilt the selected checkout with `cargo build --locked -p forge-cli
  --no-default-features`; three existing guide unit tests passed.
- `scripts/test-cli-skill.py` passed 33 cases / 167 commands against the final
  rebuilt absolute binary, with isolated projects/home and zero Provider requests.
  The four edited guide resources match their source bytes exactly.
- The skill validator and local-link checks passed; `git diff --check` passed.
- Extracted the new inline requests and bound actual synthetic source hashes.
  Both prepare Jobs, Pack validations, Godot installation Jobs and final installed
  resource verification succeeded in an isolated project.
- Godot 4.7.2 loaded the saved four-frame SpriteFrames, preserved the explicit
  60/90/120/160 ms timings and non-looping state, and completed its installed
  player once. The effective drawing origin matched the `[32,32]` usage anchor.
- Three native AudioStreamWAV resources loaded as 44100 Hz stereo PCM16 with
  battle/camp looping and victory non-looping. Synthetic 440 Hz tones are format
  fixtures; they are not music or listening evidence.

The source examples ran under the development binary before a subsequent
SpriteFrames-versus-anchor wording correction. Processing code did not change.
The guide was rebuilt and its integration/byte checks repeated afterward. Both
identities, dirty source commits, features and SHA-256 values are recorded in
[the verification artifact](artifacts/asset-production-lessons-2026-10-07.json).
This is source verification on macOS Apple Silicon, not new release-package,
Windows, target-device or artistic certification.

The first local anchor probe assumed an uncentered node and failed. It was
corrected to measure the actual top-left drawing origin, including Godot's
centering/offset properties; the original probe remains in local evidence.
Raw requests, reports, native probes and logs remain under the ignored
`release-candidates/asset-production-lessons-20261007/` directory. No private
Sanguo art, music, game source or generated Job store is committed.

**中文：**更新仅涉及指导与示例，不修改加工协议、Pack、游戏锁或已发布安装包。
指南测试 33 项 / 167 条命令通过；特效与音频示例均完成零 Provider 加工、Pack 校验
及隔离 Godot 安装。原生检查确认特效时长、单次完成、锚点和音频循环声明；音乐试听、
美术验收、游戏事件与暂停/重开流程、Windows 和真机性能仍属独立验证。
