#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo build -p forge-cli --release
npm --workspace packages/mcp run build
mkdir -p plugins/forge-assets/mcp
cp packages/mcp/dist/index.js plugins/forge-assets/mcp/server.bundle.mjs

echo "Packaged Forge MCP bundle at plugins/forge-assets/mcp/server.bundle.mjs"
