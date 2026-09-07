# Forge xAI Image 2.0 candidate offline acceptance — 2026-08-14

Result: accepted offline. Real image generation requests: 0.

## Availability and routing

- An authenticated read-only xAI model-catalog query returned
  `grok-imagine-image-2.0` for the current team. The detailed image-model
  record accepts text/image input and image output.
- `forge provider models --provider xai --json` now distinguishes the Image
  Quality incumbent, Image 2.0 candidate, Video 1.5 incumbent and edit-only
  video route without reading credentials or making a network request.
- The default xAI image model remains `grok-imagine-image-quality`.
- Image 2.0 is explicit opt-in and cannot be selected by an ordinary new or
  full Character request.

## Accepted offline contract

The exact candidate command shape is:

```bash
forge job retry \
  --id <approved-v9-direction-grid-job> \
  --item direction_grid \
  --stage still \
  --image-model grok-imagine-image-2.0 \
  --plan-only --json
```

Against approved real source Job
`9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`, read-only Plan preparation returned
`providerRequestEstimate=1`, `maximumProviderRequests=1`, provider/profile
`xai/default`, workflow `topdown-grid@9.0.0`, and model
`grok-imagine-image-2.0`. It created only an isolated pending Plan; no Job,
authorization, Provider request, or network generation occurred.

The fixture end-to-end contract proves:

- the child inherits the approved lineage but not the source authorization;
- exact authorization scope is checked before the first request;
- an xAI-like Provider without an explicit scope proof makes zero requests;
- the Provider receives exactly one image edit with target `direction_grid`;
- the outgoing payload names `grok-imagine-image-2.0` exactly;
- comparison evidence records incumbent and candidate model IDs separately;
- the incumbent retry's hash-bound correction codes are preserved in the
  candidate prompt, preventing a prompt change from being mistaken for a model
  improvement;
- the approved source directory hash is unchanged;
- output stops at `direction_grid_review_required` with no Pack or Godot.

## Focused evidence

```text
cargo test -p providers --features grid-generation --test grid_generation_contract \
  fixture_image2_candidate_is_one_direction_grid_edit_and_stops_for_review -- --exact
  1 passed

cargo test -p providers \
  image2_candidate_is_explicit_in_the_edit_payload_without_replacing_the_default
  1 passed

cargo test -p providers \
  xai_model_routes_keep_image2_candidate_opt_in_and_quality_as_default
  1 passed

cargo test -p forge-cli --features grid-generation \
  image2_direction_grid_candidate_authorization_is_exactly_one_edit
  1 passed

cargo test -p providers --features grid-generation --test grid_generation_contract
  32 passed

cargo test --workspace --all-features
  passed

cargo clippy --workspace --all-targets --all-features -- -D warnings
  passed

scripts/test-cli-product.sh
  passed
```

## Non-claims

This report does not claim Image 2.0 visual superiority and does not authorize
the real 1/1 comparison. No production model route, Pack, catalog, source Job,
Godot project, or credential store was changed. A real comparison requires a
new user authorization after reviewing the pending Plan and exact durable
grant.
