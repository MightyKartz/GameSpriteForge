# Forge xAI Image 2.0 DirectionGrid real comparison — 2026-08-15

Result: inconclusive / rate limited. The independently authorized probe was
consumed and must not be retried without a new user authorization.

## Scope

- approved source Job: `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`;
- comparison Job: `686d40f4-3f18-4101-8780-fa1132ac7552`;
- workflow and target: `topdown-grid@9.0.0`, `direction_grid`;
- candidate model: `grok-imagine-image-2.0`;
- expected / maximum Provider requests: `1 / 1`;
- maximum Provider operations: `1`;
- reserved cost ceiling: `1,400,000,000` ticks;
- retries, model fallback, Pack and Godot delivery: forbidden.

Immediately before execution, an authenticated read-only xAI model-catalog
query still returned `grok-imagine-image-2.0` for the current team. The Plan
was freshly prepared from the approved source and returned exact `1 / 1`
request bounds. A new grant was bound to the Plan recipe hash and input
fingerprint; its ledger contained zero entries before execution.

## Execution result

The single permitted `edit_image` operation received HTTP 429 and the Job
failed with `provider_rate_limited`. The durable ledger contains exactly one
entry, `request-0000000000000001`, for model
`grok-imagine-image-2.0` and target `direction_grid`. It is conservatively
recorded as `ambiguous` with the full `1,400,000,000`-tick reservation because
the Provider did not return a usable completion or cost record.

No second request was made. The xAI adapter's exact-one authorization branch
does not perform 401 refresh retries, 429 retries or URL fallback after the
model operation begins. The focused regression
`exact_one_operation_authorization_does_not_retry_rate_limited_image_request`
passed after the real failure. The surfaced phrase “after three bounded
retries” is a generic 429 error message and is inaccurate for this exact-one
branch; it is not evidence of three requests.

## Artifacts and immutability

- Provider usage records `requests=0` and `generatedImages=0` because no
  successful response was parsed.
- The comparison Job contains no PNG, JPEG, WebP, video, Pack or Godot file.
- No native visual comparison was possible and no approval was written.
- The approved source tree SHA-256 remained
  `785da20140de9aff17be792f870dd010ec564f289e8baa60eef9eb7fe5caddbc`.
- The source DirectionGrid lock and approval hashes remained respectively
  `9cf407379b2f8032d2c1abb3daecd79bb16c00a39f0ee556df98e1cbab1f1d78`
  and
  `4e0754e489cc65edb0f1b9785780b6c89f5e16a01a8d46c87b8cc58fba460a4e`.

Machine-readable evidence is in
`docs/qa/artifacts/forge-xai-image2-candidate-real-20260815/probe-summary.json`.

## Decision

This run provides no evidence that Image 2.0 is better or worse than Image
Quality for Forge DirectionGrid generation. Keep
`grok-imagine-image-quality` as the production default. A later Image 2.0
comparison requires a new Plan, a new physically empty exact authorization and
separate user approval; this consumed authorization must not be reused.
