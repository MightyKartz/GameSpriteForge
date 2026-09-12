# Local WAV audio → audio Pack → Godot

Use this route for music, sound effects and ambience already exported by the
user's chosen tools. Follow the [toolchain checks](../SKILL.md) (`forge guide
overview` in the embedded bundle). These commands are source-build additions,
outside the published v0.3.2 release. Require `local_audio_import`,
`audio_pack_validation` and `audio_godot_delivery` for import and engine delivery;
`optional_audio_tool_discovery` exposes the external-tool catalog. Check the
actual executable and preserve existing consumer pins until an upgrade is verified.

## Optional external generation

Keep the user's selected source workflow. Existing WAV files need no generation
tool. If external generation is requested, installation and model execution are
separate opt-in work. Forge does not bundle or install ACE-Step, Stable Audio 3,
Python environments or weights, and these commands never invoke model inference,
authentication or network access:

```bash
"$FORGE_BIN" audio tools list --json
"$FORGE_BIN" audio tools doctor --tool acestep --path /absolute/ACE-Step-1.5 --json
"$FORGE_BIN" audio tools doctor --tool stable-audio --path /absolute/stable-audio-3 --json
```

The catalog links [ACE-Step 1.5](https://github.com/ace-step/ACE-Step-1.5) and
[Stable Audio 3](https://github.com/Stability-AI/stable-audio-3). Their upstream
code is labeled MIT in their respective [ACE-Step code license](https://github.com/ace-step/ACE-Step-1.5/blob/main/LICENSE)
and [Stable Audio code license](https://github.com/Stability-AI/stable-audio-3/blob/main/LICENSE).
Review the selected [ACE-Step model card](https://huggingface.co/ACE-Step/acestep-v15-turbo)
or [Stability AI model terms](https://stability.ai/license) for model use. The
catalog does not verify the local license, user eligibility or rights to outputs.

`doctor` observes only fixed source-file metadata under the supplied checkout.
Omitting `--path` returns `not_configured` without searching the machine. A valid
directory reports `missing`, `partial_source_files` or `source_files_observed`;
an invalid supplied path fails. Symlinks below the selected root are reported
without following them. None of these statuses establishes runtime readiness,
dependency versions, available weights, hardware compatibility or successful
generation. Follow upstream setup separately only within the user's authorization.

## Preserve sources and prepare a request

Keep original WAV files, their source/tool identity and rights information.
Export other audio formats to PCM WAV with the chosen external tool before
intake. Forge's `audio inspect` reads local WAV format, duration and SHA-256
without generation. Audio inspection and import use the CLI's FFprobe/FFmpeg
toolchain; check its availability with `doctor`:

```bash
"$FORGE_BIN" audio inspect --path /absolute/sources/forest_ambience.wav --json
"$FORGE_BIN" guide audio-example > /absolute/asset-specs/local-audio.json
```

Confirm the guide command succeeded before editing its output. An installed
bundle also provides [the local audio example](../examples/local-audio.json).
Replace names, IDs, paths and any origin assertions with real source information.
Example paths assume `asset-specs/local-audio.json` and sibling `sources/` files.
Item paths and optional `sourceLocks` resolve relative to the request file, or
the working directory for `--stdin`.

Requests use `schemaVersion:"1"`, a set `id`, a nonempty `name` and 1–64 `items`.
IDs start with an ASCII letter or digit, contain only ASCII letters, digits,
`_` or `-`, and are at most 80 bytes. Item IDs must be unique ignoring case.
Each item needs `id`, `path` and `role` (`music`, `sfx` or `ambience`); its optional
name defaults to its ID. Local inputs must be regular WAV files, not symlinks,
at most 512 MiB and 3600 seconds, with 8000–192000 Hz and 1–8 channels.

Output is PCM16 WAV. Request-level `sampleRate` defaults to `48000` (8000–96000
allowed) and `channels` to `2` (`1` or `2` allowed). Processing controls belong
directly on each item:

| Field | Default and meaning |
| --- | --- |
| `loop` | `false`; engine playback intent |
| `trimStartSeconds` | `0`; source-relative start |
| `trimEndSeconds` | Omitted; source end |
| `gainDb` | `0`; explicit gain from −60 to +24 dB |
| `fadeInMs`, `fadeOutMs` | `0`; each fade must fit the processed duration |
| `crossfadeMs` | `0`; optional end/start blend, at most 30000 ms |

A positive crossfade requires `loop:true`, must be shorter than half the trimmed
audio, and shortens the output by one crossfade duration. Trim endpoints must fit
the source. Choose fades, gain and crossfade after inspecting the actual clip;
there is no automatic loudness target or listening approval.

Origin fields are optional strings retained as user-asserted provenance (for
example, `"seed":"42"`). Supply `tool`, `model`,
`prompt`, `seed` or `license` only when known; do not invent provider execution
or license verification. Source locks can bind the exact reviewed files: when
present, cover every distinct local source with its actual SHA-256. See
[delivery evidence](delivery.md) (`forge guide delivery`) for that contract.

## Plan, import and review

Use dedicated work stores and review the Plan before consuming its token:

```bash
export FORGE_JOB_STORE="/absolute/asset-work/jobs"
export FORGE_PLAN_STORE="/absolute/asset-work/plans"
"$FORGE_BIN" plan prepare-audio --request /absolute/asset-specs/local-audio.json --json
"$FORGE_BIN" plan execute --token TOKEN_FROM_PLAN --wait --json
"$FORGE_BIN" job report --id JOB_FROM_EXECUTION --json
```

Both `data.estimate.providerRequestEstimate` and
`data.estimate.maximumProviderRequests` must be zero. Require a successful
envelope and a succeeded execution Job; inspect the report's
`providerRequestOccurred:false` and `providerRequestCount:0`.
As a shortcut, `audio import --request /absolute/asset-specs/local-audio.json
--wait --json` prepares and consumes the same Plan in one command. Without
`--wait`, follow the detached Job with `job get --id JOB --json`.

Use returned Job artifacts to locate the `gsfpack` and `quality_report`:

```bash
"$FORGE_BIN" pack validate --path PACK_FROM_JOB_ARTIFACTS --json
"$FORGE_BIN" asset inspect --pack PACK_FROM_JOB_ARTIFACTS --json
```

Audio Packs retain original source bytes, processed WAVs, SHA-256 hashes and
processing/origin records. A `technical_pass` report validates technical evidence;
it reports no generation, verified origin, verified license, listening approval
or verified seamless loop. Listen to the complete output and repeated loop joins
at the game's intended volume. Review clipping, fade shape, transitions and
audibility in context. A loop flag or crossfade does not establish a seamless loop.
Revise with a new request and Job while preserving earlier evidence.

## Deliver and retain evidence

Use the verified Godot 4.6.x executable and the intended project. Supply stable
asset-key and target values so later imports update the same Forge-owned location:

```bash
"$FORGE_BIN" godot plan-install \
  --pack PACK_FROM_JOB_ARTIFACTS --project /absolute/game \
  --asset-key forest_audio --target addons/forge_assets/forest_audio --json
"$FORGE_BIN" plan execute --token TOKEN_FROM_INSTALL_PLAN --wait --json
"$FORGE_BIN" godot verify-install --project /absolute/game --asset-key forest_audio --json
```

Require a succeeded install Job and inspect its native verification evidence.
Read `forge_usage.json` and consume its `audioPaths` mapping by stable item ID;
the installed native streams are `streams/<id>.res`. Assign a returned stream
to the appropriate Godot audio player for music, UI sounds or positional audio.
The usage data records each item's role and loop intent. Installation is
transactional and manages only its owned target; keep game playback logic outside it.

Export and verify a portable receipt before cleaning the Job stores:

```bash
"$FORGE_BIN" receipt export --job AUDIO_JOB --install-job INSTALL_JOB \
  --out /absolute/evidence/forest-audio-receipt.json --json
"$FORGE_BIN" receipt verify --path /absolute/evidence/forest-audio-receipt.json \
  --pack PACK_FROM_JOB_ARTIFACTS --json
```

Keep the receipt, Pack, original source context and actual listening decisions.
Receipt verification checks hashes and retained evidence; its `listeningReview`
remains `not_assessed`, and it does not infer audio approval from `visualReview`.
Native load success also does not establish audible quality or device performance.
