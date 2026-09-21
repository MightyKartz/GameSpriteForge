#!/usr/bin/env python3
"""Explicit local preparation, native installation and portable evidence.

Requires Python 3.10+, a verified Forge executable, an existing Godot project,
reviewed sourceLocks, and a NEW output directory with an existing parent.
No Provider calls, automatic upgrades, project copies or invented review approval.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest() if hasattr(hashlib, "file_digest") else hashlib.sha256(stream.read()).hexdigest()


def save(path, value):
    # Only the example's own progress file is replaced; receipts remain exclusive.
    with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, delete=False) as stream:
        temporary = Path(stream.name)
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")
    try:
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def deliver(args):
    launcher = Path(args.forge).absolute()  # Keep the public launcher, including symlinks.
    request = Path(args.request).resolve(strict=True)
    project = Path(args.project).resolve(strict=True)
    output = Path(args.out).absolute()
    parent = output.parent.resolve(strict=True)
    output = parent / output.name
    if output.exists() or output.is_symlink():
        raise ValueError("--out must be new; retain prior evidence and choose another directory")
    if output.is_relative_to(project):
        raise ValueError("--out must be outside the Godot project")
    if not (project / "project.godot").is_file():
        raise ValueError("--project must contain project.godot")
    request_text = request.read_text(encoding="utf-8")
    spec = json.loads(request_text)
    if not spec.get("sourceLocks"):
        raise ValueError("Add reviewed sourceLocks to the request before delivery; this example never invents approval")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", args.expected_binary_sha256):
        raise ValueError("--expected-binary-sha256 must be the reviewed executable SHA-256")
    if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_-]{0,79}", args.asset_key):
        raise ValueError("--asset-key must be an engine-safe identifier")
    if args.operation == "prepare-audio" and not {"sampleRate", "channels"}.issubset(spec):
        raise ValueError("Audio delivery requires explicit sampleRate and channels; inspect sources first, preserve supported formats or explicitly choose conversion")
    launcher_sha = digest(launcher)
    state = {"schemaVersion": 1, "completed": False, "phase": "preflight", "commands": [],
             "visualReview": "not_recorded", "listeningReview": "not_assessed",
             "licenseReview": "not_assessed", "operation": args.operation, "project": str(project), "assetKey": args.asset_key}
    owned_output = False
    with tempfile.TemporaryDirectory(prefix="forge-delivery-preflight-") as scratch:
        env = dict(os.environ, FORGE_JOB_STORE=str(Path(scratch) / "jobs"),
                   FORGE_PLAN_STORE=str(Path(scratch) / "plans"), FORGE_REAL_PROVIDER_MAX_REQUESTS="0")
        env.pop("FORGE_REAL_PROVIDER_ACCEPT", None)
        payload = None

        def call(*arguments, input=None, cwd=None):
            if digest(launcher) != launcher_sha or (payload and digest(payload) != args.expected_binary_sha256.lower()):
                raise ValueError("Forge executable changed; stop and explicitly verify the selected toolchain")
            result = subprocess.run([str(launcher), *map(str, arguments), "--json"], env=env,
                                    capture_output=True, encoding="utf-8", input=input, cwd=cwd, timeout=120)
            try:
                envelope = json.loads(result.stdout)
            except json.JSONDecodeError as error:
                raise RuntimeError(f"Forge did not return JSON (exit {result.returncode}): {result.stderr}") from error
            state["commands"].append({"arguments": list(map(str, arguments)), "exitCode": result.returncode,
                                      "response": envelope})
            if owned_output:
                save(output / "progress.json", state)
            if result.returncode or not envelope.get("ok"):
                raise RuntimeError(str(envelope.get("error", envelope)))
            return envelope["data"]

        doctor = call("doctor")
        payload = Path(doctor["cliPath"])
        if digest(payload) != args.expected_binary_sha256.lower():
            raise ValueError("Forge binary hash mismatch; no production was started")
        state["toolchain"] = {"executable": str(payload), "binarySha256": digest(payload),
                              "build": doctor["build"], "version": doctor["cliVersion"]}
        required = {"filesystem_write_probe", "delivery_receipts", "godot_install_verification", "reviewed_source_hashes"}
        if args.operation == "prepare-audio":
            required.update({"local_audio_import", "audio_pack_validation", "audio_godot_delivery"})
        if not required.issubset(doctor["capabilities"]):
            raise ValueError("Selected Forge lacks required capabilities; do not change a consumer pin implicitly")
        for directory in dict.fromkeys([project, parent]):
            report = call("storage", "check", "--path", directory)
            if not report["supported"]:
                raise ValueError(f"Unsupported output/project filesystem at {directory}: {report}; select a supported location, do not copy an installation back")
        output.mkdir()  # Exclusive; a concurrent creator is not overwritten.
        owned_output = True
        env.update(FORGE_JOB_STORE=str(output / "jobs"), FORGE_PLAN_STORE=str(output / "plans"))
        save(output / "progress.json", state)

        def execute(plan, phase):
            state["phase"] = phase
            job = call("plan", "execute", "--token", plan["token"])
            state[phase + "Job"] = job["job_id"]
            save(output / "progress.json", state)
            deadline = time.monotonic() + args.timeout
            while True:
                report = call("job", "report", "--id", job["job_id"])
                status = report["job"]["lifecycle_state"]
                if status == "succeeded":
                    if report["providerRequestCount"] != 0 or report["providerRequestOccurred"]:
                        raise RuntimeError("Local delivery unexpectedly recorded Provider requests")
                    return report["job"]
                if status in {"failed", "cancelled", "awaiting_review"}:
                    raise RuntimeError(f"{phase} Job {job['job_id']} {status}: {report['job'].get('error_summary')}; inspect progress.json and next_actions, retain all available evidence")
                if time.monotonic() >= deadline:
                    raise TimeoutError(f"Job {job['job_id']} still {status}; retain this directory and use job report/cancel; do not start a duplicate")
                time.sleep(1)

        try:
            # Execute the checked in-memory snapshot. --stdin resolves relative
            # sources, locks and assetProject against the original request root.
            plan = call("plan", args.operation, "--stdin", input=request_text, cwd=request.parent)
            estimate = plan["estimate"]
            if estimate["providerRequestEstimate"] != 0 or estimate["maximumProviderRequests"] != 0:
                raise ValueError("Expected zero-Provider local preparation")
            job = execute(plan, "prepare")
            source_pack = Path(next(a["path"] for a in job["artifacts"] if a["kind"] == "gsfpack"))
            pack = output / "retained.gsfpack"
            shutil.copytree(source_pack, pack)
            if not call("pack", "validate", "--path", pack)["valid"]:
                raise RuntimeError("Retained Pack validation failed")
            state["pack"] = str(pack)
            prepared = call("receipt", "export", "--job", job["job_id"], "--out", output / "prepared-receipt.json")
            state["preparedReceipt"] = prepared
            call("receipt", "verify", "--path", output / "prepared-receipt.json", "--pack", pack,
                 "--expected-sha256", digest(output / "prepared-receipt.json"))
            plan = call("godot", "plan-install", "--pack", pack, "--project", project,
                        "--asset-key", args.asset_key, "--target", "addons/forge_assets/" + args.asset_key)
            installation = execute(plan, "install")
            call("godot", "verify-install", "--project", project, "--asset-key", args.asset_key, "--pack", pack)
            receipt = output / "delivery-receipt.json"
            call("receipt", "export", "--job", job["job_id"], "--install-job", installation["job_id"], "--out", receipt)
            state["deliveryReceiptSha256"] = digest(receipt)
            verified = call("receipt", "verify", "--path", receipt, "--pack", pack, "--project", project,
                            "--expected-sha256", state["deliveryReceiptSha256"])
            if verified.get("verified") is not True or verified.get("installationVerified") is not True:
                raise RuntimeError("Delivery evidence did not verify")
            state.update(completed=True, phase="complete")
        except Exception as error:
            state["error"] = str(error)
            raise
        finally:
            save(output / "progress.json", state)
    return {"completed": True, "output": str(output), "receiptSha256": state["deliveryReceiptSha256"],
            "visualReview": "not_recorded", "listeningReview": "not_assessed", "licenseReview": "not_assessed"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ["forge", "expected-binary-sha256", "request", "project", "asset-key", "out"]:
        parser.add_argument("--" + flag, required=True)
    parser.add_argument("--operation", choices=["prepare-static", "prepare-asset", "prepare-character", "prepare-audio"], required=True)
    parser.add_argument("--timeout", type=float, default=300, help="Seconds to poll each Job; timeout leaves the Job and evidence intact")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    try:
        print(json.dumps(deliver(args)))
    except Exception as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
