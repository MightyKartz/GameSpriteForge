import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  type Tool,
} from "@modelcontextprotocol/sdk/types.js";

import { runForgeCli } from "./cli.js";

const REQUEST_SCHEMA = {
  type: "object",
  additionalProperties: true,
  description: "Versioned Forge request object. Paths must be absolute unless documented otherwise.",
} as const;

const tools: Tool[] = [
  {
    name: "check_environment",
    description: "Check forge-cli, profile, job stores, Godot, and Forge app availability.",
    inputSchema: { type: "object", additionalProperties: false },
  },
  {
    name: "list_assets",
    description: "List recent .gsfpack artifacts created by Forge automation jobs.",
    inputSchema: {
      type: "object",
      properties: { recent: { type: "integer", minimum: 1, maximum: 100, default: 20 } },
      additionalProperties: false,
    },
  },
  {
    name: "inspect_asset",
    description: "Validate and inspect one existing .gsfpack directory.",
    inputSchema: {
      type: "object",
      properties: { pack: { type: "string", description: "Absolute .gsfpack directory path." } },
      required: ["pack"],
      additionalProperties: false,
    },
  },
  {
    name: "inspect_project",
    description:
      "Inspect the Forge project asset manifest in an existing Godot project without changing it.",
    inputSchema: {
      type: "object",
      properties: { project: { type: "string", description: "Absolute Godot project path." } },
      required: ["project"],
      additionalProperties: false,
    },
  },
  {
    name: "list_character_workflows",
    description:
      "List versioned Platformer, Top-down, Isometric, and Custom Character Pack contracts.",
    inputSchema: { type: "object", additionalProperties: false },
  },
  {
    name: "list_providers",
    description:
      "List Forge media-generation providers, authentication status, and capabilities. Credentials are never returned.",
    inputSchema: { type: "object", additionalProperties: false },
  },
  {
    name: "check_provider",
    description:
      "Check one provider profile without returning tokens. Interactive OAuth login must be completed by the user in forge-cli.",
    inputSchema: {
      type: "object",
      properties: {
        provider: { type: "string", description: "Provider ID such as xai or fixture." },
        profile: { type: "string", default: "default" },
      },
      required: ["provider"],
      additionalProperties: false,
    },
  },
  {
    name: "analyze_repair",
    description:
      "Analyze quality evidence for an awaiting-review prepare job and return safe parameter changes plus manual-only actions. This tool does not write outputs.",
    inputSchema: {
      type: "object",
      properties: { id: { type: "string", description: "Source Forge job ID." } },
      required: ["id"],
      additionalProperties: false,
    },
  },
  {
    name: "plan_prepare_asset",
    description:
      "Validate a PNG sequence, sprite sheet, or .gsfpack request and return a 15-minute single-use plan token. This tool does not write asset outputs.",
    inputSchema: {
      type: "object",
      properties: { request: REQUEST_SCHEMA },
      required: ["request"],
      additionalProperties: false,
    },
  },
  {
    name: "plan_prepare_character_pack",
    description:
      "Validate a schema V2 multi-animation Character Pack request and return a 15-minute single-use plan token. Animations share one normalized canvas and anchor.",
    inputSchema: {
      type: "object",
      properties: { request: REQUEST_SCHEMA },
      required: ["request"],
      additionalProperties: false,
    },
  },
  {
    name: "plan_generate_character_pack",
    description:
      "Validate a schema V3 topdown@1.0.0 provider-generation request and return a 15-minute single-use plan token. The job is locked to one provider and exports only after all required animations are game_ready.",
    inputSchema: {
      type: "object",
      properties: { request: REQUEST_SCHEMA },
      required: ["request"],
      additionalProperties: false,
    },
  },
  {
    name: "plan_install_godot",
    description:
      "Validate an existing Godot project installation request and return a 15-minute single-use plan token. This tool does not modify the project.",
    inputSchema: {
      type: "object",
      properties: { request: REQUEST_SCHEMA },
      required: ["request"],
      additionalProperties: false,
    },
  },
  {
    name: "plan_repair_job",
    description:
      "Create a 15-minute single-use plan for the safe automatic changes returned by analyze_repair. The repair executes as a new linked job and never overwrites the source job.",
    inputSchema: {
      type: "object",
      properties: { id: { type: "string", description: "Awaiting-review source job ID." } },
      required: ["id"],
      additionalProperties: false,
    },
  },
  {
    name: "execute_plan",
    description:
      "Consume a prepared plan token exactly once. Returns a background job by default; pass wait=true for foreground completion.",
    inputSchema: {
      type: "object",
      properties: {
        token: { type: "string" },
        wait: { type: "boolean", default: false },
      },
      required: ["token"],
      additionalProperties: false,
    },
  },
  {
    name: "get_job",
    description: "Read durable progress, artifacts, errors, and next actions for one Forge job.",
    inputSchema: {
      type: "object",
      properties: { id: { type: "string" } },
      required: ["id"],
      additionalProperties: false,
    },
  },
  {
    name: "cancel_job",
    description: "Request cooperative cancellation of a queued or running Forge job.",
    inputSchema: {
      type: "object",
      properties: { id: { type: "string" } },
      required: ["id"],
      additionalProperties: false,
    },
  },
  {
    name: "open_job",
    description: "Open a Forge job in the macOS app for human review or recovery.",
    inputSchema: {
      type: "object",
      properties: { id: { type: "string" } },
      required: ["id"],
      additionalProperties: false,
    },
  },
];

const server = new Server(
  { name: "forge-assets", version: "0.4.0" },
  { capabilities: { tools: {} } },
);

server.setRequestHandler(ListToolsRequestSchema, async () => ({ tools }));
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const input = (request.params.arguments ?? {}) as Record<string, unknown>;
  const envelope = await callTool(request.params.name, input);
  return {
    content: [{ type: "text", text: JSON.stringify(envelope, null, 2) }],
    structuredContent: envelope as Record<string, unknown>,
  };
});

async function callTool(name: string, input: Record<string, unknown>) {
  switch (name) {
    case "check_environment":
      return runForgeCli(["doctor", "--json"]);
    case "list_assets":
      return runForgeCli(["asset", "list", "--recent", String(input.recent ?? 20), "--json"]);
    case "inspect_asset":
      return runForgeCli(["asset", "inspect", "--pack", requireString(input, "pack"), "--json"]);
    case "inspect_project":
      return runForgeCli(["project", "inspect", "--project", requireString(input, "project"), "--json"]);
    case "list_character_workflows":
      return runForgeCli(["profile", "character-workflows", "--json"]);
    case "list_providers":
      return runForgeCli(["provider", "list", "--json"]);
    case "check_provider":
      return runForgeCli([
        "provider",
        "doctor",
        "--provider",
        requireString(input, "provider"),
        "--profile",
        typeof input.profile === "string" && input.profile.length > 0 ? input.profile : "default",
        "--json",
      ]);
    case "analyze_repair":
      return runForgeCli(["repair", "analyze", "--job", requireString(input, "id"), "--json"]);
    case "plan_prepare_asset":
      return runForgeCli(["plan", "prepare-asset", "--stdin", "--json"], requireObject(input, "request"));
    case "plan_prepare_character_pack":
      return runForgeCli(["plan", "prepare-character", "--stdin", "--json"], requireObject(input, "request"));
    case "plan_generate_character_pack":
      return runForgeCli(["plan", "generate-character", "--stdin", "--json"], requireObject(input, "request"));
    case "plan_install_godot":
      return runForgeCli(["plan", "install-godot", "--stdin", "--json"], requireObject(input, "request"));
    case "plan_repair_job":
      return runForgeCli(["plan", "repair-job", "--job", requireString(input, "id"), "--json"]);
    case "execute_plan": {
      const args = ["plan", "execute", "--token", requireString(input, "token"), "--json"];
      if (input.wait === true) args.push("--wait");
      return runForgeCli(args);
    }
    case "get_job":
      return runForgeCli(["job", "get", "--id", requireString(input, "id"), "--json"]);
    case "cancel_job":
      return runForgeCli(["job", "cancel", "--id", requireString(input, "id"), "--json"]);
    case "open_job":
      return runForgeCli(["open-job", "--id", requireString(input, "id"), "--json"]);
    default:
      throw new Error(`Unknown Forge tool: ${name}`);
  }
}

function requireString(input: Record<string, unknown>, key: string): string {
  const value = input[key];
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${key} must be a non-empty string`);
  }
  return value;
}

function requireObject(input: Record<string, unknown>, key: string): Record<string, unknown> {
  const value = input[key];
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${key} must be an object`);
  }
  return value as Record<string, unknown>;
}

const transport = new StdioServerTransport();
await server.connect(transport);
