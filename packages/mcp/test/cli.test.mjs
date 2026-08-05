import assert from "node:assert/strict";
import { chmod, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

test("MCP bundle forwards a CLI JSON envelope without adding UI state", async () => {
  const directory = await mkdtemp(join(tmpdir(), "forge-mcp-"));
  const cli = join(directory, "forge-cli");
  await writeFile(
    cli,
    "#!/bin/sh\nprintf '%s\\n' '{\"schemaVersion\":\"1\",\"ok\":true,\"data\":{\"profileId\":\"godot-pixel-art\"}}'\n",
  );
  await chmod(cli, 0o755);
  process.env.FORGE_CLI = cli;

  const { runForgeCli } = await import("../dist/cli.js");
  const result = await runForgeCli(["doctor", "--json"]);

  assert.equal(result.ok, true);
  assert.equal(result.data.profileId, "godot-pixel-art");
});

test("MCP bundle exposes project assets and the executable repair loop", async () => {
  const bundle = await readFile(new URL("../dist/index.js", import.meta.url), "utf8");

  assert.match(bundle, /plan_prepare_character_pack/);
  assert.match(bundle, /prepare-character/);
  assert.match(bundle, /list_character_workflows/);
  assert.match(bundle, /character-workflows/);
  assert.match(bundle, /inspect_project/);
  assert.match(bundle, /project", "inspect/);
  assert.match(bundle, /analyze_repair/);
  assert.match(bundle, /repair", "analyze/);
  assert.match(bundle, /plan_repair_job/);
  assert.match(bundle, /repair-job/);
  assert.match(bundle, /list_providers/);
  assert.match(bundle, /provider", "list/);
  assert.match(bundle, /check_provider/);
  assert.match(bundle, /"provider",\s*"doctor"/);
  assert.match(bundle, /plan_generate_character_pack/);
  assert.match(bundle, /generate-character/);
  assert.match(bundle, /version: "0\.4\.0"/);
});
