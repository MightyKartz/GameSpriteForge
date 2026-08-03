# xAI OAuth commercial integration request for Game Sprite Forge

Status: draft for submission. xAI OAuth remains marked Preview in Forge until written approval is received. API Key authentication is the stable commercial path.

## Product summary

Game Sprite Forge is a local-first game asset generator for Codex, Claude, and other developer agents. A user authorizes xAI in a local CLI, then an agent can plan quality-gated image/video generation without seeing credentials. Forge immediately stores generated media locally, converts it into deterministic 2D Character Packs, and installs neutral resources into Godot projects. Unity and Unreal adapters are planned on the same local pack/provenance layer.

Forge does not embed or depend on Grok Build CLI, does not proxy subscriptions through a Forge service, does not expose tokens to agents, and does not silently switch models within a character job.

## Requested written confirmation

1. May a distributed third-party commercial desktop/CLI application use the shared public client ID `b1a00492-073a-47ea-816f-4c329264a828`, or can xAI issue a Forge-specific public client ID?
2. Which scopes and subscription entitlements authorize Grok Imagine image generation, image editing, image-to-video, and reference-to-video through `https://api.x.ai/v1`?
3. May SuperGrok and X Premium subscribers use those capabilities inside a third-party commercial application, and on which tiers and regions?
4. What rate limits, attribution, branding, end-user terms, acceptable-use disclosures, moderation requirements, and desktop distribution requirements apply?
5. Are temporary generated-media URLs permitted to be downloaded immediately into a user-owned local JobStore and converted into engine assets?
6. Are generated assets commercially usable by the end user, and what provenance or AI-generation disclosures must Forge preserve?

## Security summary

- OIDC Discovery and Device Code Flow only; authorization endpoints are restricted to HTTPS `x.ai` hosts.
- Requested scopes: `openid profile email offline_access grok-cli:access api:access`.
- System Keychain/Credential Manager/Secret Service storage, with explicit opt-in owner-only file fallback.
- Refresh one hour before expiry; support Refresh Token rotation; no blind retry after ambiguous refresh transport failure.
- One refresh retry after HTTP 401; explicit entitlement classification for 403; bounded retry for 429.
- No credential, device code, authorization header, or temporary media URL in jobs, packs, MCP responses, or ordinary logs.
- The agent-facing MCP exposes Provider listing/checking and generation planning, never login or token access.

## Suggested email

Subject: Request for xAI OAuth approval / public client for Game Sprite Forge

Hello xAI team,

We are building Game Sprite Forge, a local-first game asset generator that lets users authorize Grok and then lets developer agents such as Codex or Claude create quality-gated sprite Character Packs for game engines. We would like written guidance on whether Forge may use the shared Grok public OAuth client currently used by approved local agent integrations, or whether xAI can issue a Forge-specific public client.

Forge uses Device Code OAuth, stores credentials only in the user's operating-system credential store, never exposes credentials to the agent-facing MCP, and downloads Imagine output directly into a user-owned local job directory for deterministic processing. API Key authentication remains available.

Could you confirm the permitted OAuth client, Imagine scopes and subscription entitlements, third-party commercial-use terms for SuperGrok/X Premium users, rate limits, attribution/branding, end-user terms, and distribution requirements? A detailed security and data-flow summary is available on request.

Thank you.

## Submission channel

Submit through [xAI Contact Sales](https://x.ai/contact-sales) or `sales@x.ai`. Attach this document and the architecture summary. Record the written response and approved client ID in the release checklist; do not silently promote Preview OAuth based on technical success alone.
