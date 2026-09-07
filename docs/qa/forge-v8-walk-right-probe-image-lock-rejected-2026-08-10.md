# Forge V8 Walk Right Probe Image Lock Rejected

Date: 2026-08-10

Status: blocked by human visual review. No video request was authorized or
made. No Pack was exported. The rejection is now recorded in the source Job as
`manual_rejected`; it remains immutable evidence for a targeted child retry.

## Result

- Style Job: `843e2bb3-6c2f-4540-8d0e-8cde289cc8bc`
- Image-lock Job: `733f5bf9-8060-4840-a098-513b672afb04`
- Workflow: `topdown-direction-motion@8.0.0`
- Image model: `grok-imagine-image-quality`
- Image requests: 8 expected, 8 actual, 0 retries
- Video requests: 0
- Observed image cost: 5,100,000,000 cost ticks
- Approval file: absent
- Video authorization: not created

## Blocking finding

The generated `back_idle` view loses the permanent shoulder cape visible in
`front_idle`. This is not a video, extraction, or Godot playback failure. The
failure occurs at the image-lock lineage stage before any video work.

The lock remains in `awaiting_review`; `forge job review` without `--accept`
records rejection, and absence of `direction-motion-approval.json` keeps the V8
complete stage fail-closed.

Update: `forge job review --id 733f5bf9-8060-4840-a098-513b672afb04 --reason
"front_idle is stepping instead of a stable planted idle, and back_idle loses
the permanent shoulder cape visible in front_idle"` was executed without
`--accept`. The Job now contains a signed review decision artifact and remains
unapproved.

## Evidence

- Contact sheet:
  `generated-assets/forge-v8-walk-right-probe-20260810/jobs/733f5bf9-8060-4840-a098-513b672afb04/direction-motion-lock/contact-sheet.png`
- Lock:
  `generated-assets/forge-v8-walk-right-probe-20260810/jobs/733f5bf9-8060-4840-a098-513b672afb04/source/direction-motion-lock.json`
- Provider manifest:
  `generated-assets/forge-v8-walk-right-probe-20260810/jobs/733f5bf9-8060-4840-a098-513b672afb04/source/direction-motion-provider-manifest.json`
- Usage:
  `generated-assets/forge-v8-walk-right-probe-20260810/jobs/733f5bf9-8060-4840-a098-513b672afb04/provider-usage.json`

## Implemented correction

Do not tune chroma, loop, or playback thresholds for this failure. Repair the
V8 image-lock gate:

1. The failed Job is immutable and supports targeted `image_locks` retry:
   `idle_up` regenerates only `back_idle` and dependent `back_walk`;
   `idle_down` regenerates every derived node because it replaces the canonical
   reference.
2. Retry edits use the current canonical parent as the edit target plus the
   rejected prior image as negative evidence and receive the human review note.
3. Provider-neutral structural feedback now covers a stepping canonical biped
   idle and parent permanent-structure/component loss. It does not hard-code a
   cape, species, costume, limb count, or prop.
4. The next real xAI gate is a separately authorized `idle_up` image-lock
   retry, estimated 2 and maximum 4 image edits. Video remains forbidden until
   a new eight-image lock is reviewed and approved.
