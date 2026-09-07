#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GODOT="${GODOT_BIN:-/Applications/Godot.app/Contents/MacOS/Godot}"

if [[ $# -lt 1 || $# -gt 2 ]]; then
	echo "usage: $0 <accepted-v95-job-dir> [artifact-dir]" >&2
	exit 2
fi

JOB_DIR="$(cd "$1" && pwd)"
ARTIFACT_DIR="${2:-$ROOT/docs/qa/artifacts/forge-topdown-grid-v95-godot-smoke-$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$ARTIFACT_DIR"
ARTIFACT_DIR="$(cd "$ARTIFACT_DIR" && pwd)"

if [[ ! -x "$GODOT" ]]; then
	echo "Godot executable is missing: $GODOT" >&2
	exit 1
fi
if ! jq -e '.lifecycle_state == "succeeded" and .state == "quality_checked"' "$JOB_DIR/job.json" >/dev/null; then
	echo "Source Job is not an accepted V9.5 validation result." >&2
	exit 1
fi
if ! jq -e '.accepted == true and .providerRequestCount == 0' "$JOB_DIR/review-decision.json" >/dev/null; then
	echo "Source Job does not have the zero-Provider native acceptance decision." >&2
	exit 1
fi
if ! jq -e '.profile == "grid-local-footwear-cleanup@1.0.0" and .providerRequestOccurred == false and (.outputFrameSha256s | length) == 4' \
	"$JOB_DIR/source/local-footwear-cleanup/local-cleanup-manifest.json" >/dev/null; then
	echo "Source Job does not have the required hash-closed cleanup manifest." >&2
	exit 1
fi

JOB_ID="$(jq -r '.job_id' "$JOB_DIR/job.json")"
PARENT_JOB_ID="$(jq -r '.parent_job_id' "$JOB_DIR/job.json")"
JOBS_ROOT="$(dirname "$JOB_DIR")"
PARENT_JOB_JSON="$JOBS_ROOT/$PARENT_JOB_ID/job.json"
if [[ ! -f "$PARENT_JOB_JSON" ]]; then
	echo "Accepted cleanup Job parent is missing from its JobStore." >&2
	exit 1
fi
FORMAL_PROJECT="$(jq -r '.recipe.request.projectPath // empty' "$PARENT_JOB_JSON")"
if [[ -z "$FORMAL_PROJECT" || ! -d "$FORMAL_PROJECT" ]]; then
	echo "Accepted cleanup Job does not resolve its formal Forge project." >&2
	exit 1
fi
EXPECTED_SHA256S=(
	"$(jq -r '.outputFrameSha256s[0]' "$JOB_DIR/source/local-footwear-cleanup/local-cleanup-manifest.json")"
	"$(jq -r '.outputFrameSha256s[1]' "$JOB_DIR/source/local-footwear-cleanup/local-cleanup-manifest.json")"
	"$(jq -r '.outputFrameSha256s[2]' "$JOB_DIR/source/local-footwear-cleanup/local-cleanup-manifest.json")"
	"$(jq -r '.outputFrameSha256s[3]' "$JOB_DIR/source/local-footwear-cleanup/local-cleanup-manifest.json")"
)

source_tree_sha256() {
	find "$JOB_DIR" -type f ! -name '.job.lock' -print0 \
		| sort -z \
		| xargs -0 shasum -a 256 \
		| shasum -a 256 \
		| awk '{print $1}'
}

project_tree_sha256() {
	find "$FORMAL_PROJECT" -type f -print0 \
		| sort -z \
		| xargs -0 shasum -a 256 \
		| shasum -a 256 \
		| awk '{print $1}'
}

SOURCE_TREE_BEFORE="$(source_tree_sha256)"
PROJECT_TREE_BEFORE="$(project_tree_sha256)"
TMP_PROJECT="$(mktemp -d "${TMPDIR:-/tmp}/forge-v95-godot-smoke.XXXXXX")"
trap 'rm -rf "$TMP_PROJECT"' EXIT
mkdir -p "$TMP_PROJECT/frames" "$TMP_PROJECT/output"
cp "$ROOT/scripts/godot/v95_walk_down_smoke.project.godot" "$TMP_PROJECT/project.godot"
cp "$ROOT/scripts/godot/verify_v95_walk_down_smoke.gd" "$TMP_PROJECT/verify_v95_walk_down_smoke.gd"

for index in 0 1 2 3; do
	frame_name="frame-$(printf '%02d' "$index").png"
	frame_path="$JOB_DIR/grid-keyframes/walk_down/$frame_name"
	actual_sha256="$(shasum -a 256 "$frame_path" | awk '{print $1}')"
	if [[ "$actual_sha256" != "${EXPECTED_SHA256S[$index]}" ]]; then
		echo "Source frame $index escaped its cleanup-manifest SHA-256." >&2
		exit 1
	fi
	cp "$frame_path" "$TMP_PROJECT/frames/$frame_name"
done

unset XAI_API_KEY XAI_OAUTH_ACCESS_TOKEN FORGE_REAL_PROVIDER_ACCEPT FORGE_PROVIDER_AUTHORIZATION_ID

"$GODOT" --headless --editor --quit --path "$TMP_PROJECT" \
	2>&1 | tee "$ARTIFACT_DIR/godot-import.log"
"$GODOT" --headless --path "$TMP_PROJECT" \
	--script "$TMP_PROJECT/verify_v95_walk_down_smoke.gd" -- \
	"$JOB_ID" 4.0 "${EXPECTED_SHA256S[@]}" \
	2>&1 | tee "$ARTIFACT_DIR/godot-runtime.log"
if ! rg -q '^PASS Forge V9\.5 Godot walk_down smoke:' "$ARTIFACT_DIR/godot-runtime.log"; then
	echo "Godot runtime smoke did not emit its success marker." >&2
	exit 1
fi

mkdir -p "$ARTIFACT_DIR/frames" "$ARTIFACT_DIR/output"
cp "$TMP_PROJECT/project.godot" "$ARTIFACT_DIR/project.godot"
cp "$TMP_PROJECT/verify_v95_walk_down_smoke.gd" "$ARTIFACT_DIR/verify_v95_walk_down_smoke.gd"
cp "$TMP_PROJECT"/frames/frame-*.png "$ARTIFACT_DIR/frames/"
cp "$TMP_PROJECT/output/godot-smoke-report.json" "$ARTIFACT_DIR/output/"
cp "$TMP_PROJECT/output/walk_down-runtime-contact-sheet.png" "$ARTIFACT_DIR/output/"
cp "$TMP_PROJECT/output/walk_down-runtime-strip.png" "$ARTIFACT_DIR/output/"
cp "$TMP_PROJECT/output/walk_down.spriteframes.tres" "$ARTIFACT_DIR/output/"
cp "$TMP_PROJECT/output/walk_down_smoke.tscn" "$ARTIFACT_DIR/output/"

jq -e '
	.verdict == "pass"
	and .sourceJobId == $job
	and .animation.name == "walk_down"
	and .animation.frameCount == 4
	and .animation.fps == 4
	and .animation.loop == true
	and .runtime.nodeType == "AnimatedSprite2D"
	and .runtime.textureFilter == "nearest"
	and .alpha.allFramesHaveTransparentBorder == true
	and .alpha.baselineDriftPx <= 3
' --arg job "$JOB_ID" "$ARTIFACT_DIR/output/godot-smoke-report.json" >/dev/null

for resource in "$ARTIFACT_DIR/output/walk_down.spriteframes.tres" "$ARTIFACT_DIR/output/walk_down_smoke.tscn"; do
	if [[ $(stat -f '%z' "$resource") -ge 1048576 ]]; then
		echo "Godot text resource exceeds 1 MiB: $resource" >&2
		exit 1
	fi
	if rg -q 'PackedByteArray|ImageTexture|create_from_image|\[sub_resource type="Image"' "$resource"; then
		echo "Godot text resource embeds raster data: $resource" >&2
		exit 1
	fi
done
if [[ $(rg -c '^\[ext_resource type="Texture2D" path="res://frames/frame-[0-9]{2}\.png"' "$ARTIFACT_DIR/output/walk_down.spriteframes.tres") -ne 4 ]]; then
	echo "SpriteFrames does not retain four external PNG resources." >&2
	exit 1
fi

SOURCE_TREE_AFTER="$(source_tree_sha256)"
if [[ "$SOURCE_TREE_BEFORE" != "$SOURCE_TREE_AFTER" ]]; then
	echo "Accepted source Job changed during the isolated Godot smoke." >&2
	exit 1
fi
PROJECT_TREE_AFTER="$(project_tree_sha256)"
if [[ "$PROJECT_TREE_BEFORE" != "$PROJECT_TREE_AFTER" ]]; then
	echo "Formal Forge project changed during the isolated Godot smoke." >&2
	exit 1
fi

jq -n \
	--arg profile "forge-godot-source-integrity@1.0.0" \
	--arg sourceJobId "$JOB_ID" \
	--arg sourceTreeSha256Before "$SOURCE_TREE_BEFORE" \
	--arg sourceTreeSha256After "$SOURCE_TREE_AFTER" \
	--arg formalProject "$FORMAL_PROJECT" \
	--arg formalProjectTreeSha256Before "$PROJECT_TREE_BEFORE" \
	--arg formalProjectTreeSha256After "$PROJECT_TREE_AFTER" \
	--arg godotVersion "$($GODOT --version)" \
	'{
		schemaVersion: "1",
		profile: $profile,
		sourceJobId: $sourceJobId,
		sourceTreeSha256Before: $sourceTreeSha256Before,
		sourceTreeSha256After: $sourceTreeSha256After,
		sourceUnchanged: ($sourceTreeSha256Before == $sourceTreeSha256After),
		formalProject: $formalProject,
		formalProjectTreeSha256Before: $formalProjectTreeSha256Before,
		formalProjectTreeSha256After: $formalProjectTreeSha256After,
		formalProjectUnchanged: ($formalProjectTreeSha256Before == $formalProjectTreeSha256After),
		providerRequests: 0,
		packCreated: false,
		formalProjectMutated: false,
		godotVersion: $godotVersion
	}' > "$ARTIFACT_DIR/source-integrity.json"

echo "PASS isolated Godot V9.5 walk_down smoke: $ARTIFACT_DIR"
