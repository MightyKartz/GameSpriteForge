# Forge CLI resource demo — 2026-08-02

## Result

A fresh isolated fixture run completed through the public `forge` CLI using project
root `docs/qa/artifacts/forge-cli-demo-20260802`. The Style Lock, Character Pack,
Icon Set, and Prop Set jobs all reached `succeeded`; all three exported `.gsfpack`
directories passed `forge pack validate`.

## Generated jobs

- Style Lock: `cfd59854-b4e4-4bc0-8750-a88242b70425`
- Character: `11e927ed-d684-40c3-b515-a9ae57d04e35`
- Icon Set: `b32181e2-79a1-40bf-a02b-f8f7c7ec4ff9`
- Prop Set: `9bfacbc9-89eb-43ed-8ce0-a460fd5c6638`

The fixture Provider deliberately renders deterministic purple geometry. This run is
evidence for Style locking, animation extraction, consistency reports, Pack export,
and reproducibility; it is not evidence of production visual quality.

## xAI status

The explicit xAI credential check waited on macOS Keychain approval and was cancelled.
No credential or authorization output was read. The preserved real-xAI ranger preview
from the earlier successful Provider QA is shown separately when presenting this demo.
