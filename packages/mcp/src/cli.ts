import { access } from "node:fs/promises";
import { constants } from "node:fs";
import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export type ForgeEnvelope = {
  schemaVersion: string;
  ok: boolean;
  data?: unknown;
  error?: { code: string; message: string };
};

export async function resolveForgeCli(): Promise<string> {
  if (process.env.FORGE_CLI) {
    await requireExecutable(process.env.FORGE_CLI);
    return process.env.FORGE_CLI;
  }

  const packageDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const candidates = [
    resolve(packageDirectory, "../../target/release/forge-cli"),
    resolve(packageDirectory, "../../target/debug/forge-cli"),
  ];
  for (const candidate of candidates) {
    try {
      await requireExecutable(candidate);
      return candidate;
    } catch {
      // Try the next local build before falling back to PATH.
    }
  }
  return "forge-cli";
}

export async function runForgeCli(
  args: string[],
  input?: unknown,
): Promise<ForgeEnvelope> {
  const executable = await resolveForgeCli();
  return new Promise((resolvePromise, reject) => {
    const child = spawn(executable, args, {
      env: process.env,
      stdio: ["pipe", "pipe", "pipe"],
    });
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    child.stdout.on("data", (chunk: Buffer) => stdout.push(chunk));
    child.stderr.on("data", (chunk: Buffer) => stderr.push(chunk));
    child.on("error", (error) => reject(error));
    child.on("close", (status) => {
      const text = Buffer.concat(stdout).toString("utf8").trim();
      let envelope: ForgeEnvelope;
      try {
        envelope = JSON.parse(text) as ForgeEnvelope;
      } catch {
        reject(
          new Error(
            `forge-cli returned non-JSON output (status ${status}): ${text || Buffer.concat(stderr).toString("utf8")}`,
          ),
        );
        return;
      }
      if (!envelope.ok) {
        reject(
          new Error(
            envelope.error?.message ??
              Buffer.concat(stderr).toString("utf8") ??
              `forge-cli exited with status ${status}`,
          ),
        );
        return;
      }
      resolvePromise(envelope);
    });
    if (input === undefined) {
      child.stdin.end();
    } else {
      child.stdin.end(JSON.stringify(input));
    }
  });
}

async function requireExecutable(path: string): Promise<void> {
  await access(path, constants.X_OK);
}
