# Forge v0.7.3 release verification

Published on 2026-09-27 from clean default-feature commit
`f0efaf5a2fe5465b4c14a333535d416d632381b7`.

[Release](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.7.3) ·
[successful tag workflow](https://github.com/MightyKartz/GameSpriteForge/actions/runs/36292968662) ·
[build identities and binary/archive hashes](release-v0.7.3.json).

## Native checks

- macOS Apple Silicon CI: workspace formatting, clippy and tests; release identity;
  pinned LGPL FFmpeg build; CLI product and embedded skill checks; native Godot
  static/animation delivery; installed fresh/upgrade/reinstall checks; packaged
  Agent workflows; SBOM and release attestation.
- Windows x64 CI: clean default MSVC release and pinned helpers; native MP4 encoding;
  fresh installation and historical/v0.6.4 upgrades; PowerShell 5.1 compatibility;
  static/layered/audio/library/native Godot workflows through the installed launcher.
- Assembly: both package identities, full payload manifests and installer bundle
  consistency passed before the eight download assets were published.
- Local integrity review: the downloaded macOS artifact checksum/build identity
  matches the published asset digest. The downloaded published Windows archive,
  every payload manifest entry and executable hash were checked. These integrity
  checks are separate from native runtime tests on the corresponding CI OS.

Godot 4.6.3 is used by the release workflow. Builds have `features: []`,
`dirty: false`, and `profile: release`. Exact identities are in the JSON record.

## Scope and limits

No processing API or Pack format changes in this patch. The guide and accepted
courier example preserve reviewed continuous cycles and original video timing.
PR #78 remains the artistic acceptance record; this release run does not create a
new artistic, device or four-direction animation approval. Packages are unsigned;
macOS is not notarized. Consumer Forge pins and private source media were not changed.

Forge PRO's private beta has its own verification and compatibility lock. Its
account-blocked Actions jobs are not evidence against or for this public release.

中文：macOS 与 Windows 各自在 GitHub 原生运行器完成发行包、安装升级和 Godot 检查，
合并发布任务全部成功。已核对正式附件与构建身份，详细哈希见 JSON。未改动既有游戏
项目的工具锁或私有视频。技术验证与既有美术认可分开记录，安装包仍未签名/公证。
