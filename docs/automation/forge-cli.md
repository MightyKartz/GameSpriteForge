# `forge` CLI automation protocol

`forge` is the public Forge product and the source of truth for asset generation,
quality evidence, durable jobs, Pack export, and Godot installation. Desktop and MCP
clients are not part of the CLI release.

## Output contract

Every command invoked with `--json` writes exactly one JSON value to stdout:

```json
{
  "schemaVersion": "1",
  "ok": true,
  "data": {}
}
```

Errors set `ok` to `false`, include stable `code` and `message` fields, and exit
non-zero. Diagnostics go to stderr. Credentials, bearer headers, OAuth responses, and
device codes never appear in either stream. Explicitly non-secret authorization
manifests and request ledgers are returned as audit evidence.

## Product commands

```text
forge doctor --json
forge provider list --json
forge provider models --provider xai --json
forge provider login --provider xai --method api-key
forge provider login --provider xai --method oauth
forge provider authorize --provider xai --profile default --id portrait-fix-01 \
  --target happy --target angry --max-requests-per-target 2 --max-requests 4 \
  --max-cost-ticks 4000000000 --cost-reservation-ticks-per-request 1000000000 \
  --model grok-imagine-image --json
forge provider authorization --id portrait-fix-01 --json

forge project init --path /absolute/assets --name "My Game"
forge style create --project /absolute/assets --spec style.json \
  --authorization style-01 --wait --json
forge style inspect --project /absolute/assets --json

forge generate character --project /absolute/assets --spec ranger.json [--wait] --json
forge generate icon-set --project /absolute/assets --spec icons.json [--wait] --json
forge generate prop-set --project /absolute/assets --spec props.json [--wait] --json

forge job get --id JOB --json
forge job report --id JOB --json
forge job cancel --id JOB --json
forge job retry --id JOB --item ITEM_OR_ANIMATION \
  --stage auto|still|video|frame|loop|matting|consistency [--frame 0-7] \
  [--plan-only|--wait] --json
forge job review --id JOB --accept --reason "visual review" --json

forge pack validate --path /absolute/Pack.gsfpack --json
forge pack audit-motion --path /absolute/Character.gsfpack --json
forge godot plan-install --pack /absolute/Pack.gsfpack --project /absolute/game --json
forge plan execute --token TOKEN [--authorization AUTHORIZATION_ID] [--wait] --json
```

When image candidates are authored through a Codex subscription instead of a
Provider API, use the zero-request external-keyframe intake. Replace every
placeholder with an absolute transparent PNG path, then prepare and execute the
normal single-use plan:

```bash
forge plan prepare-character \
  --request examples/cli/character-external-keyframes-v11.json --json
forge plan execute --token TOKEN --wait --json
```

`topdown-external-keyframes@11.0.0` accepts only independent `png_sequence`
inputs: one non-looping idle frame and four authored walk poses per direction.
It rejects sprite sheets and video as content authority, requires identical
square canvases with real Alpha, rejects byte-identical frames, and makes no
Provider request. `rendering.mirrorPolicy` must explicitly choose
`right_only` (require right idle/walk, preserve any declared up/down animations,
and expose no left mapping), `explicit_left` (supply left idle/walk), or
`mirror_right_to_left` (omit left assets and explicitly authorize mirroring).
The Pack records nearest/linear filtering and pixel snapping for the Godot
installer.

External animation sheets whose cells already share one authored coordinate
system should set `normalize.mode` to `preserve_canvas`. This keeps source
dimensions and pixel coordinates unchanged instead of re-centering every frame
from its changing locomotion bounding box. The existing `square_bottom`,
`square_center`, and `auto_width_center` modes remain unchanged and continue to
serve independently framed inputs.

Before paying for a complete 32-frame Character V2 run, one direction can be
generated as a bounded acceptance Job:

```bash
forge generate character \
  --project /absolute/assets \
  --spec character-v2.json \
  --validation-animation walk_right \
  --plan-only --json
```

`--validation-animation` accepts `topdown-video@2.0.0`,
`topdown-keyframes@2.2.0`/`@2.3.0`,
`topdown-keyposes@2.4.0`/`@2.5.0`, `topdown-spritesheet@3.0.0`,
or `topdown-frames@4.0.0`.
The stable video workflow estimates one direction-still edit plus one
image-to-video request and permits at most four requests over two attempts. It
rejects camera, direction, framing, and crop failures before paying for video.
The sprite-sheet workflow estimates one image edit and permits at most two for
the selected action. Key-pose workflows estimate four image edits and permit at most eight;
the older eight-frame workflows estimate eight and permit at most 16. Execution writes
the immutable Job, frame provenance, consistency, hand/equipment contact,
silhouette reports, and debug playback/contact sheets. It deliberately does
not export or catalog a partial Character Pack.

`generate` prepares and immediately consumes the same fingerprinted single-use plan
used by low-level automation. Without `--wait` it returns a Job ID and a detached
worker continues the job.

## Plans and jobs

A plan validates and fingerprints local inputs without generating media or changing a
Godot project. Its token expires after 15 minutes, is consumed once, and refuses to
execute if an input changes. Execution creates an immutable recipe and a durable
JobStore record. Cancellation is cooperative between provider and processing steps.

`job retry --item` creates a new source-linked Job. A static retry calls the provider
only for that icon or prop and copies accepted siblings after checking their source
directory and SHA-256 evidence. A Character retry behaves the same way for
`idle`, `walk_up`, `walk_right`, or `walk_down`. Character stages have explicit
invalidation boundaries:

- `still` regenerates the direction still and invalidates video and all local stages;
- `video` reuses the still and prefers xAI video editing, with same-still
  image-to-video as the recorded fallback;
- `loop` reuses the video and reruns candidate extraction through Pack export with
  zero Provider requests;
- `matting` reuses the video and reruns background processing and downstream stages;
- `auto` selects the earliest stage implied by persisted consistency and loop evidence.
- `frame` is available to top-down keyframe/key-pose workflows and regenerates only the selected
  frame; accepted siblings remain hash-verified and unchanged.

Static assets keep their existing `--item` retry semantics and reject Character-only
stages. Every retry creates a new source-linked Job; it never mutates its parent.

`job report` embeds Provider attempt/usage evidence, Loop Selection reports, and the
non-secret durable authorization manifest/ledger in the JSON response. It includes
`providerRequestOccurred` and the selected source frame range, so an agent can
distinguish free local reprocessing from a paid generation and reconstruct request
consumption across a parent/child/replay lineage.

`job review` may promote only an `awaiting_review` gray-band result. It cannot bypass
missing/corrupt media, invalid Alpha, crop, canvas, frame, or other hard gates.

## Project, style, and asset contracts

- `ForgeProjectV1` locks the Provider Profile, output directory, and current immutable
  Style revision.
- `StyleSpecV1` accepts zero to three references and canvas/perspective/lighting intent.
- `StyleLockV1` records SHA-256 references, Provider/model identity, style board,
  palette, edge density, foreground scale, and background baseline.
- `AssetSpecV1` covers `character`, `icon_set`, and `prop_set`. Paths are resolved
  relative to the spec before planning.
- `ConsistencyReportV1` records each direction/item attempt, metrics, threshold profile,
  verdict, and review reasons.
- Portrait V1 remains the implicit `dialogue_bust@1.0.0` contract. Portrait V2 must
  explicitly select `dialogue_bust@1.0.0` or `full_body@1.0.0`; the latter requires a
  feet-grounded Collection Lock and disables bust reframing. V2 also requires `neutral`
  as the first expression. Public V2 generation is two-phase: Forge first creates an
  immutable `portrait-base-lock@1.0.0` from neutral and stops for a hash-bound
  `portrait-base-approval@1.0.0`; only then does it send that neutral image as the edit
  target for every other
  expression. Final pixels outside the deterministic face scope are copied exactly from
  neutral; skin-tone, protected-cheek artifact, face-detail, and identity metrics are
  recorded in `portrait-consistency-report.json`.
- Character animation frames use the separate `character_sprite@1.0.0` geometry
  contract. Reports use an honest `lowerBodyPresenceProxy`; Forge does not claim
  anatomical leg recognition without an audited semantic vision component.
- `.gsfpack` V2 adds `assetType`, static `items`, consistency evidence, and Style
  provenance. The reader remains compatible with V1 Character Packs.

All jobs lock one Provider, Profile, model selection, and Style revision. Forge rejects
missing capabilities instead of silently switching Provider or degrading to unrelated
text-to-image calls.

## Consistency profile

`consistency@1.2.0` evaluates perceptually matched palette overlap, longest-extent
foreground scale, reference-normalized edge density, major-subject count, anchor
drift, and optional foreground identity similarity. Character direction palettes and
edges are compared to the canonical character reference; Style Lock still governs
generation, but a mixed character/icon/prop style board is not treated as the
character's literal color palette. It has three
outcomes: `game_ready`, `awaiting_review`, and `regenerate`/`blocked`. Each generated
direction or item gets at most two automatic attempts before the job pauses.

The stable Character V2 `topdown-video@2.0.0` path derives one canonical reference,
four direction stills, and four image-to-video clips. It requires an explicit
`cameraProfile`, executes `direction-still-preflight@1.0.0` before every video request,
and records the camera profile in the Provider manifest, Pack provenance, and
`forge_usage.json`. The legacy `topdown@1.0.0` path remains readable for V1 Jobs.
The opt-in `topdown-keyframes@2.3.0` production candidate
uses frame 0 to establish an explicit front, rear, or right DirectionLock. Phase anchors
2/4/6 use that immutable direction target plus a transparent compact Pose guide; Style
is a text/JSON descriptor and is never sent as an image reference. Optional equipment
must be explicitly locked as `none` or `staff_like`. Every Provider output is stored raw,
then deterministically cleaned by `keyframe-background-cleanup@1.3.0`; only the cleaned,
normalized frame can become a DirectionLock or an adjacent reference. In-betweens
1/3/5/7 use the two approved cleaned neighbors + Pose. Separate Provider, cleanup, and
normalization WorkflowGraph nodes preserve hashes, retry boundaries, and zero-cost local
replay. Godot flips `walk_right` for left-facing playback. Icon and prop sets first
establish an anchor item, then derive the remaining items from the Style and anchor.

The experimental `topdown-keyposes@2.4.0` path replaces the eight-frame
anchor/in-between schedule with four authored poses at 6 FPS. Frame 0 establishes the
DirectionLock. Frames 1–3 use only that lock, the previous accepted frame for appearance
continuity, and the current direction-specific Pose guide; the previous pose must not be
blended into the requested joints. `motion-semantics@1.5.0` is a hard Pack gate for gait
energy, pose diversity, contact order, stable-region flicker, lower-edge ghosts, and extra
foot lobes. `forge pack audit-motion` applies the same deterministic report to historical
packs without Provider requests.
For side-facing walks, V1.5 also reports proximal-leg, knee/shin, and foot Alpha dynamics
using upper-body alignment. It blocks a cycle when knee/shin motion is below `0.14`,
above `0.80`, or foot motion is more than `3.5x` the knee/shin motion. The lower bounds
prevent a boots-only animation from passing merely because its bottom silhouette changes;
the upper knee/shin envelope catches marching-biased or excessively redrawn passing poses.
This raster metric is a proxy and does not replace native Godot visual gait review.
Four-frame `walk_left`/`walk_right` additionally use a contact/passing cadence score
instead of treating top-down Alpha contact polarity as anatomical laterality. The report
sets `sideLateralityReviewRequired`; a hash-locked
`side-walk-laterality-approval@1.0.0` document records the human near/far-leg decision.
V1.5 aligns stable-upper-body comparisons with the upper body itself instead of the
stride-dependent whole-character bounding box, and separates upper Alpha drift from
interior RGB color flicker. This prevents a long contact stride from manufacturing a
false upper-body shift while retaining independent geometry and texture gates.

`topdown-keyposes@2.5.0` is the isolated follow-up. Frames 1–3 no longer receive a
previous-frame image reference: each edit uses only the immutable DirectionLock and the
current Pose guide. The guide marks character-left and character-right limbs separately,
and the Provider manifest records `direction_lock_plus_pose_only`. Motion failures are
non-reviewable, return `character_motion_semantics_failed`, and include
`recommendedRetryFrames`; V2.4 remains available for immutable replay.

`topdown-spritesheet@3.0.0` is experimental and asks the image Provider for one fixed 2×2 sheet per
action and never calls a video Provider. Core owns row-major splitting, chroma/Alpha
cleanup, alignment and all Character hard gates. Full generation is 4 expected / 8
maximum image edits. An animation-level `--stage still` child requests only that
sheet; `matting`, `loop`, and `consistency` replay verified frames at zero cost.
Real xAI acceptance produced near-duplicate locomotion cells, so this route is
retained for comparison/contact sheets rather than default walking motion. Idle
playback is 4 FPS and walk playback is 5 FPS. Godot uses only native
`AnimatedSprite2D`, `SpriteFrames`, and clipped `AtlasTexture` regions.

`topdown-keyframes@2.2.0` remains readable and executable for historical Jobs. It keeps
the reference-isolation and explicit-equipment contract but does not retroactively gain
DirectionLock or the separately reported cleanup node.

`topdown-keyframes@2.1.0` remains readable for historical Jobs but is not accepted by
the single-direction paid validation command. Its Style-board image reference and
opaque horizontal-arm Pose guide are retained only for provenance-compatible replay.

`loop@2.0.0` samples the complete generated video at no more than 12 FPS and 96
candidates. After matting and provisional alignment it searches a closed interval,
uses the boundary frame only as closure proof, and exports evenly sampled frames from
`[start, end)` without duplicating the first frame. Its fixed score combines Mask IoU
(30%), soft palette overlap (20%), edge overlap (20%), anchor closure (15%), and wrap
transition continuity (15%). Walk motion energy must be at least 1%; idle motion must
be at least 0.2%. `regenerate` and `blocked` loop results cannot be manually promoted.
`animation-timing@1.0.0` first selects the shortest strong fundamental period from
closure and self-similarity evidence, then maps its relative frame intervals into a
versioned gameplay cadence. It records exact source timestamps plus one playback
duration per exported frame and feeds the same durations to GIF and Godot
`SpriteFrames`. The default walk window is 800–1250 ms. Retiming outside a safe 0.5–2.0
ratio is blocked instead of forcing an implausible animation into a hard-coded 12 FPS.

Generated Character Jobs also write `workflow-stage-manifest.json`. Each stage records
its implementation version, input/output SHA-256 values, invalidated descendants, and
whether it made a Provider request. `.gsfpack` V2 carries additive
`quality/loops.json` evidence; older readers remain compatible.

## xAI authentication

API-key login reads a hidden TTY value and stores it in the operating system credential
store; an API key is never accepted as a CLI argument. Device Code OAuth is Preview.
Both modes feed the same direct xAI REST Provider; Forge has no Grok Build CLI
dependency. `fixture` implements the same contract offline for deterministic tests.

Forge stores a separate non-secret auth profile containing only the selected method and
storage backend, so generation reads one credential entry instead of probing API Key
and OAuth entries on every process launch. Production defaults to Keychain. Developers
whose rebuilt ad-hoc binaries would repeatedly trigger macOS approval can explicitly
use owner-only OAuth file storage:

```bash
forge provider login --provider xai --method oauth --credential-store file
```

Official macOS binaries use the fixed code-signing identifier
`dev.gamespriteforge.cli` and one Developer ID team across releases.

## Durable real-provider authorization

Credential login answers “who may authenticate”; `provider authorize` answers “which
targets, how many model attempts, Provider operations, and how much cost may this
operation consume.” The
authorization manifest contains no secret. Before each xAI network request Forge takes
an atomic reservation from a persistent ledger, then records `submitted`, `settled`, or
`ambiguous`. A transport failure after submission remains consumed. Per-target and
total limits therefore survive process restarts, detached workers, child retries, and
workflow replay. Private upload/cleanup entries are classified as transport operations:
they remain visible in the ledger but do not consume a target's image/video model-attempt
cap or reserve model cost. Upload + video edit capacity is reserved atomically before the
upload begins. A newly supplied `--authorization` may replace an inherited grant;
otherwise child/replay Jobs inherit the parent grant and lineage root.

The previous environment-only real-provider budget guard remains a compatibility path
for older automation, but new paid acceptance runs should use a durable authorization.
The ledger stores target IDs, models, Job IDs, cost ticks, and timestamps only—never
prompts, credentials, authorization headers, temporary URLs, or media.

## Portrait geometry contracts

Dialogue busts intentionally do not require legs. Full-body portraits are assessed on
the matted provider image before crop/scale normalization, preventing a bust from being
made to look valid merely by filling the output canvas. Obvious missing lower-body
silhouettes are `blocked`; robe/occlusion ambiguity is `awaiting_review`; neither state
is mislabeled as anatomically verified. A blocked item cannot be manually promoted or
included in a Pack.

Portrait V2 uses a neutral-star topology rather than independently regenerating every
complete character. The Base phase establishes neutral once; `happy`, `angry`, `hurt`,
and `surprised` are independent children of that approved immutable base and never
children of each other. A Base Job may retry only `neutral` before approval. An
Expressions Job cannot retry `neutral`; replacing the base requires a new Base Job and a
new approval. A single expression retry reuses neutral and its accepted siblings.
`--stage consistency` rebuilds the lock, face-scoped composites, and local report from
source artifacts with zero Provider requests. `job review` cannot override `regenerate`
or `blocked` Portrait-local findings.

The paid request boundary is explicit:

```bash
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json \
  --phase base --neutral-reference-policy subject-style --plan-only --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json \
  --phase base --neutral-reference-policy subject-style --wait --json
forge job review --id <base-job-id> --accept \
  --reason "neutral identity and equipment approved" --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json \
  --phase expressions --base-job <base-job-id> --plan-only --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json \
  --phase expressions --base-job <base-job-id> --wait --json
```

Base plans estimate 1 request and cap at 2. Expression plans estimate 4 and cap at 8.
The default neutral policy is `subject-style@1.0.0`: ordered Subject identity then Style
board, with Collection constraints in the prompt instead of a conflicting anchor image.
`subject-edit@1.0.0` uses only Subject canonical as the edit target.
`legacy-three-reference@1.0.0` exists only for explicit comparison and old behavior.
Expression plans verify the approved Job, Lock, neutral PNG, policy, Style, Subject,
Collection, Provider/profile/model, and every SHA before any Provider request.

`portrait-local@1.1.0` makes face review expression-aware. Skin-tone comparison excludes
versioned eye/brow/mouth zones, requires skin on both sides of a correspondence, and uses
a 10% trimmed mean. `skinCorrespondenceRatio` prevents invalid colors from disappearing
from the metric merely because they no longer classify as skin. Non-severe skin-only
findings may enter `awaiting_review`; severe marks, low correspondence, identity drift,
and any changed pixel outside the face scope remain non-overridable hard failures. The
schema continues to read `portrait-local@1.0.0` reports.

Paid execution should use separate least-privilege authorizations for the two phases:

```bash
forge provider authorize --provider xai --profile default --id portrait-base-01 \
  --target neutral --max-requests-per-target 2 --max-requests 2 \
  --max-cost-ticks 2000000000 --cost-reservation-ticks-per-request 1000000000 \
  --model grok-imagine-image --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json \
  --phase base --neutral-reference-policy subject-style \
  --authorization portrait-base-01 --wait --json

forge provider authorize --provider xai --profile default --id portrait-expressions-01 \
  --target happy --target angry --target hurt --target surprised \
  --max-requests-per-target 2 --max-requests 8 \
  --max-cost-ticks 8000000000 --cost-reservation-ticks-per-request 1000000000 \
  --model grok-imagine-image --source-job <base-job-id> --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json \
  --phase expressions --base-job <base-job-id> \
  --authorization portrait-expressions-01 --wait --json
```

The numeric cost caps above are examples matching the current one-billion-tick request
reservation. Users should set them from the Provider/model price information available
at authorization time. The Expressions grant cannot authorize `neutral` or any unrelated
asset.

## Godot 4.6.x delivery

Godot installation is a separate single-use plan. Forge copies PNG textures first,
runs a headless import, and then creates resources with `ResourceLoader`. It rejects
text `.tres`/`.tscn` files at or above 1 MiB and any embedded Image
`PackedByteArray`.

- Characters receive external atlas textures, `SpriteFrames`, an
  `AnimatedSprite2D` scene, and directional playback metadata.
- Icon sets receive one external PNG per item and an item-to-`res://` mapping.
- Prop sets receive one external PNG and one `Sprite2D` scene per item.

Every install writes `forge_usage.json`, registers atomically in
`.forge/assets.json`, replaces only Forge-owned targets, and restores the previous
installation after failure. Character usage includes the loop profile, selected
boundaries/frame indices, and the recorded Provider retry method.
Portrait V2 usage includes the base/profile SHA and local-report SHA; it never includes
local source paths or Provider authentication material. It also includes the neutral
reference policy and approval source/hash provenance.

## Static consistency replay

```bash
forge style create --project /absolute/assets --spec /absolute/style.json --wait --json
forge job retry --id <icon-or-prop-job> --stage consistency --wait --json
```

`style-baseline@2.3.0` removes the dominant border-connected background before extracting
the Style palette and records a new immutable revision. If the prior revision was created
from the same spec, Provider, profile, and references, Forge copies its SHA-256-verified
style board into the new revision instead of generating another image. `--stage consistency`
copies the source Job's normalized static images, recalculates `consistency@1.5.0`, and makes
zero Provider requests. An optional `--item` limits the explicit target while all reused
items are also rechecked against the current baseline.

## Character release gate

Character direction and equipment effects are local hard gates:

```bash
forge job retry --id <character-job> --stage consistency --wait --json

forge job assemble-character \
  --base <character-job> \
  --source idle=<idle-job> \
  --source walk_up=<up-job> \
  --source walk_right=<right-job> \
  --source walk_down=<down-job> \
  --wait --json
```

The command reuses normalized frames and makes zero Provider requests for both video and keyframe
Character workflows. `topdown-3q-orthographic@1.0.0` locks the camera to one three-quarter
orthographic gameplay view. `body-framing@1.0.0` normalizes the body center, body scale, top
margin, and foot anchor on a 256×256 canvas while fitting held equipment inside the cell.
`direction-quality@1.2.0` rejects a front-facing `walk_up`, a rear-facing front animation, or direction drift inside the selected interval.
`equipment-effect-consistency@1.1.0` rejects detached sparks and translucent halos attached to
bright equipment when the asset did not explicitly request baked emission. These failures cannot
be accepted with `forge job review`; retry `still` for direction failures, `video` for generated
detached effects, or `matting`/`consistency` for locally repairable alpha halos and framing.
Legacy and keyframe Character reports retain `consistency@1.6.0`. `topdown-video-locked@5.0.0`
uses `consistency@1.7.0`, recomputed from the exported action frames against each DirectionLock
anchor. Static assets remain on `consistency@1.5.0`.

Successful Character Packs include `character-semantic-quality-report.json`, and Godot
`forge_usage.json` records Camera, framing, direction, and equipment-effect profiles. A failed
replay exports no Pack and quarantines a matching generated-asset catalog entry.

`job assemble-character` verifies the immutable lineage, Provider/profile, models, prompt,
source paths, and media SHA-256 values before creating a local child. It performs no Provider
request and records each selected sibling in `character-assembly-manifest.json`.

`direction-quality@1.2.0` requires a minimum density of enclosed eye/mouth features before
declaring a face visible. This prevents hands, orange scarves, and costume folds in a valid rear
view from being misclassified as a face; reports created by `direction-quality@1.0.0` remain
schema-readable.

`v0.2.0-cli.1` must not be tagged until three clean xAI Style → Character → Pack →
Godot runs finish without review, all four actions are `game_ready`, provenance and
usage can be reconstructed, and credential/temporary-URL scans are clean. The offline
fixture and CI contracts are necessary but do not replace this real-model gate.

## Development store overrides

```bash
export FORGE_JOB_STORE="/absolute/test/jobs"
export FORGE_PLAN_STORE="/absolute/test/plans"
```
# Image-only locked-frame Character workflow

`topdown-frames@4.0.0` is the experimental image-only Character path. A new full Character plans five image-edit requests (maximum ten): one four-view DirectionLock sheet and one four-frame sheet for each of `idle`, `walk_up`, `walk_right`, and `walk_down`.

Use `examples/cli/character-v2-frames.json` as the spec template. A targeted repair keeps the source Job immutable:

```bash
forge job retry \
  --id <source-job-id> \
  --item walk_right \
  --frame 2 \
  --stage frame \
  --wait --json
```

The child plan authorizes only `walk_right:frame:2`; the other three frame PNGs are reused byte-for-byte. Local `matting`, `loop`, and `consistency` replay remains zero-cost.

# Direction-locked video Character workflow

`topdown-video-locked@5.0.0` combines the V4 four-view DirectionLock with the
existing deterministic video candidate, loop-selection, Pack, and Godot paths.
Use `examples/cli/character-v2-locked-video.json`. A new full Character plans one
`direction_lock` image-edit target and `idle:video`, `walk_up:video`,
`walk_right:video`, and `walk_down:video`; no `*:still` or video-edit target is
authorized. Each accepted loop exports exactly eight frames. Retry keeps the
DirectionLock immutable and accepts `auto`, `video`, `loop`, `matting`, or
`consistency`; the last three make zero Provider requests.

# Repeated-cycle video Character workflow

`topdown-video-cycle@6.0.0` is an experimental correction for V5 input-detail,
gait, and playback-speed failures. It keeps the 256px DirectionLock images as
delivery assets while sending separate 512px-or-larger generation masters to
720p image-to-video. Each video prompt requests three or four identical cycles;
Forge then requires two opposing contacts, two passing poses, one 700–1200ms
source walk period, and a closed boundary before exporting eight semantic
phases. All walks play as 8×100ms (800ms, 10 FPS); idle plays as 8×200ms
(1600ms, 5 FPS). Debug GIFs, Pack manifests, and Godot `SpriteFrames` consume
the same `frameDurationsMs` values.

Use `examples/cli/character-v2-video-cycle.json`. A new full Character plans
one `direction_lock` image edit plus four videos (five expected, ten maximum).
Reusing a DirectionLock plans four/eight; `loop`, `matting`, and `consistency`
replay plans zero requests and does not require Provider credentials. V6 remains
experimental until frozen-video and separately authorized real-xAI gates pass.

# Direction-motion Character workflow

`topdown-direction-motion@8.0.0` implements an explicit visual lineage instead
of deriving every action directly from one ambiguous sheet. Start from
`examples/cli/character-v2-direction-motion.json`:

```bash
forge generate character \
  --project /absolute/game-art \
  --spec /absolute/character-v2-direction-motion.json \
  --direction-motion-stage image-locks \
  --wait --json

forge job review \
  --id <image-lock-job-id> \
  --accept \
  --reason "four direction idles and four direction motion poses approved" \
  --json

forge generate character \
  --project /absolute/game-art \
  --spec /absolute/character-v2-direction-motion.json \
  --direction-motion-stage complete \
  --image-lock-job <image-lock-job-id> \
  --wait --json

# Minimal approved-video probe: one video request, maximum two, and no Pack.
forge generate character \
  --project /absolute/game-art \
  --spec /absolute/character-v2-direction-motion.json \
  --direction-motion-stage complete \
  --image-lock-job <image-lock-job-id> \
  --validation-animation walk_right \
  --wait --json
```

The first plan is 8 expected / 16 maximum image requests. It cannot request a
video or export a Pack. The second is 4/8 video requests and cannot regenerate
an image. With `--validation-animation walk_right`, the complete-stage plan is
1/2 video requests, contains no idle or Pack export, and uses only the typed
`right_walk` generation master. The full second stage validates the
approval-bound Lock and generates `walk_down`,
`walk_up`, `walk_right`, and `walk_left` from their corresponding motion-pose
nodes. Four static direction idles are exported alongside the walks.

If the image-lock contact sheet is visually rejected, do not accept it and do
not authorize video. Record the rejection and retry only the failed lineage:

```bash
forge job review \
  --id <image-lock-job-id> \
  --reason "back_idle loses a permanent structure visible in front_idle" \
  --json

forge job retry \
  --id <image-lock-job-id> \
  --item idle_up \
  --stage still \
  --wait --json
```

`idle_up` regenerates only `back_idle` and `back_walk`; `walk_right`
regenerates only `right_walk`; `idle_down` regenerates every derived node
because it replaces the canonical front reference. The child Job receives the
human review note and the rejected image as negative evidence, then stops for
review again before any video authorization.

V8 loop/matting/consistency retry is a zero-request child Job. V8.1 retains
native decoded PTS up to 24 FPS / 120 frames, searches the full video before
reducing the chosen cycle, and exports only 8, 10, or 12 original poses. A
twelve-frame cycle that still exceeds the reconstruction budget fails closed.
Godot receives exact PTS-derived per-frame durations, and Pack validation binds
the four sampling reports by path and SHA-256. See the
[V8 architecture and release gates](../architecture/forge-character-direction-motion-v8-plan.md).

# Importing an approved Subject without a Provider

When an existing canonical image has already been visually approved, the
`subject-import` feature can bind it into a fresh immutable SubjectLock without
calling a model or reading Provider credentials:

```bash
forge subject import \
  --project /absolute/game-art \
  --spec /absolute/subject.json \
  --canonical /absolute/approved-character.png \
  --approval-note "approved canonical recovered by SHA-256" \
  --plan-only --json
```

The plan must report zero expected and maximum Provider requests. The input
must be a transparent, single-subject PNG and the Subject spec must not also
declare reference images. Forge preserves an already delivery-sized canonical
byte-for-byte; otherwise it normalizes locally. The resulting SubjectLock
records `subject-import@1.0.0`, the input/canonical/mask SHA-256 values, the
human approval note, and the exact StyleLock revision and board SHA-256.
Identity measurements remain diagnostic for an explicitly approved import;
structural media failures still fail closed. A character plan rejects a
SubjectLock if either its Style revision or Style board hash differs from the
current StyleLock.

# Two-stage grid Character workflow

`topdown-grid@9.0.0` is an experimental image-only workflow. Build Forge with
the `grid-generation` feature and start from
`examples/cli/character-v2-grid.json`. The first stage makes one 2x2 direction
grid request and stops before action generation or Pack export:

```bash
forge generate character \
  --project /absolute/game-art \
  --spec /absolute/character-v2-grid.json \
  --direction-motion-stage image-locks \
  --wait --json

forge job review \
  --id <direction-grid-job-id> \
  --accept \
  --reason "front, back, right, and left direction idles approved" \
  --json
```

Approval is bound to the source Job, DirectionGridLock SHA-256, and every cell
hash. A missing, rejected, or modified approval fails before any action-grid
Provider request. After approval, the second stage makes four independent 2x2
action-grid requests and exports four static idles plus four walks:

```bash
forge generate character \
  --project /absolute/game-art \
  --spec /absolute/character-v2-grid.json \
  --direction-motion-stage complete \
  --image-lock-job <direction-grid-job-id> \
  --wait --json
```

The stages are authorized separately: direction grid is 1 expected / 2
maximum image requests; action grids are 4 expected / 8 maximum image
requests. The workflow resolves no video model and never authorizes a video
target. A failed action cell can be retried with `job retry --item <walk> \
--frame <0-3> --stage frame`; the other three cells are reused byte-for-byte
in the child Job.

`topdown-grid@9.1.0` keeps the approved V9.0 DirectionGridLock but replaces
the Action Grid with four independent single-frame image edits per direction.
Use `examples/cli/character-v2-grid-keyframes.json`; `poseGuidance` must stay
`disabled`, the workflow is complete-only, and `--image-lock-job` must name an
explicitly approved V9.0 Direction Grid Job:

```bash
forge generate character \
  --project /absolute/game-art \
  --spec /absolute/character-v2-grid-keyframes.json \
  --direction-motion-stage complete \
  --image-lock-job <approved-direction-grid-job-id> \
  --validation-animation walk_down \
  --wait --json
```

The validation probe generates ordered left-contact, left-passing,
right-contact, and right-passing frames with 4 expected / 8 maximum image
requests, then stops for native review without a Pack. Omit
`--validation-animation` for all four directions (16 expected / 32 maximum).
Video requests and video authorization targets are always zero. Fresh and
static-retry frames reference only the corresponding DirectionAnchor; a
motion diagnostic edit may additionally reference its own failed frame. A
child retry uses `job retry --item <walk> --frame <0-3> --stage frame` and
reuses the other three frame bytes unchanged.

`topdown-grid@9.2.0` is the structured-gait successor to the rejected V9.1
real probe. It remains validation-only and accepts exactly `walk_down`. Use
`examples/cli/character-v2-grid-structured-gait.json`; `poseGuidance` must be
`grayscale`:

```bash
forge generate character \
  --project /absolute/game-art \
  --spec /absolute/character-v2-grid-structured-gait.json \
  --direction-motion-stage complete \
  --image-lock-job <approved-v9.0-direction-grid-job-id> \
  --validation-animation walk_down \
  --wait --json
```

Fresh frames reference the approved `DirectionAnchor` and a transparent
grayscale `PoseStructure`; diagnostic retries add the immutable `EditTarget`
first. `gait-laterality@1.0.0` requires screen-left contact/support in frames
0/1 and screen-right contact/support in frames 2/3. Same-side collapse returns
`walk_laterality_not_alternating` and retries only frames 2/3. The Plan is 4
expected / 8 maximum image edits and exposes only `walk_down:frame:0..3`; it
cannot generate another direction, video, partial Pack, catalog entry, or
Godot mutation. Real-model execution still requires separate authorization.

If a failed V9.2 Job has `motionVerdict: game_ready`, stable
`walk_laterality_not_alternating`, and its immutable reports recommend exactly
one frame, create a child validation with:

```bash
forge job retry \
  --id <failed-v9.2-job-id> \
  --item walk_down \
  --frame <recommended-frame> \
  --stage frame \
  --authorization <new-one-frame-authorization-id> \
  --wait --json
```

The Plan is exactly 1 expected / 1 maximum image edit and authorization exposes
only that frame. The request is a fresh `DirectionAnchor + PoseStructure`
edit: it does not carry the rejected frame, `EditTarget`, `inputFrameSha256`,
or `replacesFrameSha256`. The other three frames are reused byte-for-byte.
Any frame other than the report's unique recommendation is rejected before a
Provider request. A new real child run still requires separate authorization.

`topdown-grid@9.3.0` is the asymmetric-guide remediation for the real V9.2
case where frame 2 remained screen-left contact after its one ordinary fresh
retry. It preserves the approved DirectionGridLock and frames 0/1/3. The new
PoseStructure gives the grounded leg bright thick ink and a wide horizontal
sole, while the raised leg uses dark thin ink and a compact lifted boot. Its
Provider prompt uses only `screen_left_*` / `screen_right_*` phase names and
explicitly defines them as image-space halves, never anatomical sides.

To upgrade a stable V9.2 laterality failure into the V9.3 one-frame child,
first prepare (but do not execute) the exact V9.3 Plan and retain its
`recipeHash` and `inputFingerprint`:

```bash
forge plan generate-character \
  --request /absolute/immutable-v9.2-derived-v93-request.json --json
```

Then create a new, empty-ledger authorization whose scope and reviewed Plan
identity are exact (no superset or unbound grant is accepted):

```bash
forge provider authorize \
  --provider xai --profile default \
  --id <new-v9.3-authorization-id> \
  --target walk_down:frame:2 \
  --max-requests-per-target 1 \
  --max-requests 1 \
  --max-provider-operations 1 \
  --max-cost-ticks 1400000000 \
  --cost-reservation-ticks-per-request 1400000000 \
  --model <resolved-image-model> \
  --source-job <failed-v9.2-job-id> \
  --recipe-hash <pending-plan-recipe-hash> \
  --input-fingerprint <pending-plan-input-fingerprint> \
  --expires-minutes 60 --json
```

The source must be the original validation-only `topdown-grid@9.2.0`
`walk_down` Job failed with `walk_laterality_not_alternating`, not its already
consumed ordinary-fresh child. The grant must name exactly one model, the
source lineage root, one request, one Provider operation, and the single
frame-2 target. It must also bind the pending Plan's exact recipe/input hashes,
and its ledger must still be physically empty at execution preflight. When the
grant is attached, Forge records the SHA-256 of the exact reviewed manifest in
the Job. Provider binding, reservation, HTTP retry/fallback decisions, and
ledger transitions all use that immutable snapshot and fail closed if the
durable manifest bytes change.

```bash
forge plan execute \
  --token <pending-plan-token> \
  --authorization <new-one-frame-authorization-id> \
  --wait --json
```

`topdown-grid@9.4.0` is the platform-safe successor for a V9.3 candidate whose
frame 2 passes gait/identity/equipment gates but contains a platform or skate
under the planted boot. It never reopens or mutates V9.3. Its source closure
requires the immutable V9.3 `awaiting_review` Job, hash-closed Action Report,
Provider manifest and WorkflowGraph, and the current silhouette gate must
block exactly frame 2.

The child reuses frames 0/1/3 byte-for-byte and sends exactly two references
for frame 2: `DirectionAnchor`, then `grid-pose-structure@1.2.0`. V1.2 keeps
bright/thick versus dark/thin support roles but removes the wide horizontal
sole. `footwear-platform@1.0.0` hard-blocks a thin shelf at least twice as wide
as the leg stem and persists per-frame metrics. The Plan and independent
authorization are still exact 1 request / 1 Provider operation / 1.4B ticks;
use [the V9.4 example](../../examples/cli/character-v2-grid-platform-safe-gait.json)
only as a request shape and derive real paths/identity from the immutable V9.3
recipe. Preparing or running this request does not inherit the consumed V9.3
authorization.

If a manually rejected V9.4 frame already has the correct gait pose but the
current `footwear-platform@1.1.0` gate uniquely reproduces a neutral-gray
matte/guide component in `walk_down` frame 2, use the zero-Provider local
cleanup instead of asking the image model to redraw the leg:

```bash
forge job cleanup-footwear --id <manually-rejected-v9.4-job-id> --json
```

The command creates one immutable child Job. It byte-reuses frames 0/1/3 and
changes only the strict neutral-gray component plus one directly adjacent
antialiasing ring in frame 2; recursive expansion into boot shadows is
forbidden. The child records before/after frame hashes, repair bounds, modified
pixel count, and proof that every non-mask pixel is byte-identical. It then
re-runs motion, viewer-space laterality, equipment/empty-hands and footwear
gates and writes a 2x2 review sheet. Provider requests, Provider operations,
cost, video, Pack, catalog and Godot mutations are all zero. A native review is
still required:

```bash
forge job review --id <cleanup-child-job-id> --accept \
  --reason "native frame and contact sheet accepted" --json
```

Acceptance re-hashes every artifact, reloads all four PNGs, re-runs the four
gates, rechecks the zero-request usage report, and cross-validates the repair
report against the local cleanup manifest. A different source shape, multiple
leaked frames, a changed gait pose, or any Provider-backed cleanup must use a
separately versioned workflow rather than this command.

### External DirectionGrid candidate import

An external 2×2 sheet can be evaluated without pretending that its pixels came
from the configured media Provider:

```bash
forge job import-direction-grid \
  --id <approved-v9-direction-grid-job> \
  --sheet /absolute/external-front-rear-right-left.png \
  --generator codex_builtin_image_gen \
  --note "external candidate for native DirectionGrid review" \
  --wait --json
```

Alternatively, import four independently generated files with explicit names:

```bash
forge job import-direction-grid \
  --id <approved-v9-direction-grid-job> \
  --front /absolute/front.png \
  --rear /absolute/rear.png \
  --right /absolute/right.png \
  --left /absolute/left.png \
  --generator codex_builtin_image_gen \
  --note "four direction candidates for native review" \
  --wait --json
```

`--sheet` and the four named inputs are mutually exclusive. The four files
must be equally-sized square PNGs; their declared fields define order and every
path/SHA is part of the single-use Plan fingerprint.

The source must be a succeeded and approved `topdown-grid@9.0.0` DirectionGrid
Job. The sheet must be an even square PNG ordered front, rear, screen-right and
screen-left. It must live outside the immutable source Job. `--generator` is a
provenance label, not a claim that Forge called or identified that model.

The Plan always estimates 0 Provider requests. Execution clears inherited
authorization, copies and hashes the external input, removes only
border-connected light checkerboard pixels, reconstructs and decontaminates a
soft Alpha edge, and rejects neutral/dark halo residue using final white,
black, and magenta composites. It extracts exactly four subjects, performs
one shared scale plus per-direction center/foot alignment against the approved
same-direction authority, and then runs absolute appearance plus
`direction-grid-import-consistency@1.0.0` identity/detail gates.

New output records `direction-grid-import@1.1.0`,
`checkerboard-sheet-matting@1.1.0`, `alpha-edge-halo@1.0.0`, and
`direction-grid-authority-alignment@1.0.0`. The
`direction-grid-lock@1.2.0` separates the real `external_import` producer from
the xAI/profile/model `downstreamBinding`; the latter is not a claim that xAI
produced the pixels. Every imported node remains
`providerRequestOccurred=false`. The Job stops in `awaiting_review`, never
inherits approval, and writes no Pack, catalog entry, or Godot asset.

The native review package is a fixed four-row layout: approved source,
candidate, magenta Alpha composite, and difference overlay. Red/cyan marks
silhouette removal/addition; yellow marks changed foreground detail. A new
strap, prop, identity-bearing detail, or source-relative size/baseline drift is
a hard failure and cannot be manually accepted by lowering a global threshold.

Use `--plan-only` to review the zero-request operation before staging. A changed
sheet, source Lock, Subject/Style input or approval after Plan creation is
rejected before processing.

### Front-authoritative DirectionGrid regeneration

When an approved `front_idle` has the correct identity and cape topology but
the rotated views invent a skirt-like lower cape hem, prepare a new bounded
DirectionGrid child:

```bash
forge job retry \
  --id <approved-v9-direction-grid-job> \
  --item direction_grid \
  --stage still \
  --front-authoritative-grid \
  --plan-only --json
```

This is not four independent direction generations. Forge sends exactly one
image reference: the approved source `front_idle`. A single image edit must
return one 2×2 sheet ordered front, rear, screen-right, screen-left. The front
cell reproduces the authority; the other cells are rotations only. Style is
text metadata and cannot contribute clothing. The bound
`front_authoritative_no_skirt_hem` contract rejects gold piping, a decorative
lower border, or a flared skirt/apron topology that is absent from the front.

The source may be an approved generated V9 DirectionGrid or an approved local
DirectionGrid import. It is never mutated. The Plan is exactly 1 expected / 1
maximum Provider request, permits no hidden retry, stops at native review, and
cannot export a Pack, catalog entry, or Godot asset.

For xAI execution, review the pending Plan and create a new independent grant
whose model equals the Plan model:

```bash
forge provider authorize \
  --provider xai --profile default \
  --id <new-front-authority-authorization> \
  --target direction_grid \
  --max-requests-per-target 1 \
  --max-requests 1 \
  --max-provider-operations 1 \
  --max-cost-ticks 1400000000 \
  --cost-reservation-ticks-per-request 1400000000 \
  --model <pending-plan-image-model> \
  --source-job <approved-v9-direction-grid-job> \
  --recipe-hash <pending-plan-recipe-hash> \
  --input-fingerprint <pending-plan-input-fingerprint> \
  --expires-minutes 60 --json

forge plan execute \
  --token <pending-plan-token> \
  --authorization <new-front-authority-authorization> \
  --wait --json
```

The authorization must be unexpired, physically unused, and bound to one
model, one `direction_grid` target, one Provider operation, the approved-source
lineage, and the exact reviewed Plan hashes. The source authorization cannot be
reused.

### xAI Image 2.0 candidate comparison

`forge provider models --provider xai --json` reports Forge's static routing,
not live team entitlement. `grok-imagine-image-quality` remains the image
default; `grok-imagine-image-2.0` is an explicit evaluation candidate.

Prepare its only enabled comparison route from an approved xAI V9 DirectionGrid:

```bash
forge job retry \
  --id <approved-v9-direction-grid-job> \
  --item direction_grid \
  --stage still \
  --image-model grok-imagine-image-2.0 \
  --plan-only --json
```

The source must be a succeeded, approved `topdown-grid@9.0.0` Image Quality
Job for the same Subject, Style, camera and equipment. The Plan is exactly one
expected / one maximum image edit, has no hidden retry or fallback, and leaves
the approved source immutable. Any other Image 2.0 scope is rejected.

After reviewing the pending Plan, create a new independent exact grant:

```bash
forge provider authorize \
  --provider xai --profile default \
  --id <new-image2-candidate-authorization> \
  --target direction_grid \
  --max-requests-per-target 1 \
  --max-requests 1 \
  --max-provider-operations 1 \
  --max-cost-ticks 1400000000 \
  --cost-reservation-ticks-per-request 1400000000 \
  --model grok-imagine-image-2.0 \
  --source-job <approved-v9-direction-grid-job> \
  --recipe-hash <pending-plan-recipe-hash> \
  --input-fingerprint <pending-plan-input-fingerprint> \
  --expires-minutes 60 --json

forge plan execute \
  --token <pending-plan-token> \
  --authorization <new-image2-candidate-authorization> \
  --wait --json
```

Automatic success stops at native DirectionGrid review. It never exports a
Pack or Godot asset, and one accepted comparison does not switch the default.

### Continuous four-direction cycle (V10)

`topdown-cycle@10.0.0` keeps an approved V9 DirectionGridLock as the immutable
identity/direction source. It byte-reuses the four direction idles and creates
each walk as one continuous in-place media request, then selects a complete
8/10/12-frame gait cycle from native source timestamps. It does not regenerate
the direction stills or independently resize generated frames.

Use [the V10 example](../../examples/cli/character-v2-cycle.json) as a request
shape. A validation request is exactly `walk_down` with 1 expected / 1 maximum
video generation and no Pack. A complete fixture request uses 4 expected / 4
maximum videos and delivers four idles plus `walk_down`, `walk_up`,
`walk_right`, and `walk_left`.

The only enabled real route is independently authorized `xai/default` with
`grok-imagine-video-1.5`, 480p, 4 seconds, target `walk_down:video`, one model
generation request, one model operation and 3.3B cost ticks. The authorization
must be new, unexpired, physically empty, bound to the approved V9 lineage and
the exact pending Plan recipe/input hashes. A successful automatic evaluation
stops in `awaiting_review`; it never exports a validation Pack. Other
directions, full real generation, retries, longer duration, a different model,
or a different Provider remain forbidden.

The complete fixture Pack retains the approved V9 lock and approval, records native
cycle timing, and adds `character-scale-lock@1.0.0`. That lock is calculated
over the complete eight-animation set and rejects body/torso scale, center, or
foot-baseline drift. A real `walk_down` probe requires a separately versioned
provider route and a new independent 1/1 authorization after native visual
review criteria are frozen; fixture acceptance alone does not authorize it.

### Cape topology audit and V10.1 local replay

For a character whose cape must have one continuous gold lower hem in every
direction, use the named four-file import and opt in to the topology contract:

```bash
forge job import-direction-grid \
  --id <approved-direction-grid-job> \
  --front <front.png> --rear <rear.png> \
  --right <right.png> --left <left.png> \
  --cape-hem continuous-gold-lower-hem \
  --note "four-direction cape hem review" --wait --json
```

The operation is local-only and records `cape-hem-consistency@1.0.0`. A missing
front hem blocks approval even when the other identity and Alpha gates pass.

To replay the one eligible failed V10 walk without a model request:

```bash
forge job retry \
  --id <failed-v10-scale-lock-job> \
  --item walk_down --stage loop \
  --plan-only --json
```

The resulting `topdown-cycle@10.1.0` Plan must show 0 expected / 0 maximum.
Execution reuses the source video byte-for-byte, performs integer-translation
anchor stabilization, and reruns every quality gate. It never accepts an
authorization, generates media, exports a partial Pack, or changes Godot.

The Plan remains exactly 1 expected / 1 maximum image edit. References are
exactly `DirectionAnchor`, then the V1.1 `PoseStructure`; the rejected raster
is absent. New reports use `grid-keyframe-action-report@1.3.0` and record a
per-frame `poseStructureProfile`, so byte-reused V1.0 guides and the selected
V1.1 guide remain distinguishable. A second asymmetric/laterality fresh child
is rejected atomically when staging, including concurrent Plan executions from
the same source. Each reused graph node records inputs in the exact order
`source sprite path/SHA`, `DirectionAnchor`, `legacy PoseStructure`, with
`cacheHit=true` and `providerRequest=false`. V9.3 is still validation-only: no other direction, video, Pack,
catalog entry, or Godot mutation is permitted. Use
`examples/cli/character-v2-grid-asymmetric-gait.json` only as a request-shape
reference. It is not a directly executable real request: derive the real
request from the immutable eligible V9.2 recipe and preserve its
`reuseFromJobDir`, validation-only `walk_down`, retry stage `frame`, and retry
frame `2` fields. V9.3 cannot start a fresh 4-frame Job and must be created
from the evidence-selected V9.2 child route above.

Retained V6/V7 videos can use the same native-PTS sampler without falsifying a
V8 DirectionMotionLock. Submit a normal `plan prepare-character` request with:

```json
{
  "sourceCycleSamplingProfile": "source-cycle-sampling@1.0.0",
  "sourceCycleSamplingPreview": true
}
```

The plan estimate is 0/0 and the source videos remain fingerprinted local
inputs. Preview mode is fail-closed delivery: if the 8/10/12-frame budget or a
motion gate fails, Forge keeps a maximum-12-frame diagnostic contact sheet and
PTS-timed GIFs, skips Pack export, and never registers the result in Godot's
production asset catalog. A non-preview local request may export only when all
normal quality gates pass; its Pack records the original source workflow,
source video hashes, processing upgrade, and zero Provider requests.
