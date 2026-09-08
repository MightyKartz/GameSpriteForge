#!/usr/bin/env python3
"""Exercise the embedded Forge skill and safe installation through the CLI, offline.

Only the supplied executable is used; its public symlink is not resolved before
execution. Every project, user home, source image and store belongs to a new test
directory. No Provider generation or Godot installation is performed.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import struct
import subprocess
import tempfile
from urllib.parse import unquote, urlsplit
import zlib


MANIFEST = ".forge-skill-manifest.json"
SKILL = "forge-use"


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def sha256(value):
    return hashlib.sha256(value).hexdigest()


def content_hash(files):
    return sha256(json.dumps(files, sort_keys=True, separators=(",", ":")).encode())


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def snapshot(root):
    """Capture content and modification metadata without following symlinks."""
    result = {}

    def visit(path, relative):
        info = path.lstat()
        mode = stat.S_IMODE(info.st_mode)
        if path.is_symlink():
            result[relative] = ("symlink", os.readlink(path), mode, info.st_mtime_ns)
        elif path.is_dir():
            # A correctly acquired/released install lock can change a parent's
            # directory mtime while leaving its contents and managed files intact.
            result[relative] = ("directory", mode)
            for child in sorted(path.iterdir()):
                visit(child, f"{relative}/{child.name}")
        else:
            require(path.is_file(), f"Unexpected special file in test tree: {path}")
            result[relative] = ("file", sha256(path.read_bytes()), mode, info.st_mtime_ns)

    if root.exists() or root.is_symlink():
        visit(root, ".")
    return result


def byte_inventory(root):
    """Compare backups independently of copy/move timestamps."""
    return {key: value[:3] for key, value in snapshot(root).items()}


def make_png(path, index):
    """Write a tiny deterministic RGBA fixture using only the standard library."""
    def chunk(kind, data):
        return (struct.pack(">I", len(data)) + kind + data
                + struct.pack(">I", zlib.crc32(kind + data)))

    rows = bytearray()
    color = ((37 + 31 * index) % 256, (139 + 19 * index) % 256, 75, 255)
    for y in range(64):
        rows.append(0)
        for x in range(64):
            rows.extend(color if 20 <= x < 44 and 8 <= y < 56 else (0, 0, 0, 0))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"\x89PNG\r\n\x1a\n"
                     + chunk(b"IHDR", struct.pack(">IIBBBBB", 64, 64, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


class Harness:
    def __init__(self, forge, root):
        self.forge = forge
        self.root = root
        self.calls = []
        self.cases = []
        self.bundle = None
        self.expected = {}
        self.home = root / "user-home"
        self.empty_tools = root / "empty-tools"
        self.logs = root / "commands"
        for path in (self.home, self.empty_tools, self.logs):
            path.mkdir()
        self.env = os.environ.copy()
        # Keep user configuration and external media tools out of the test.
        for key in list(self.env):
            if key.startswith(("FORGE_", "XAI_", "OPENAI_")):
                self.env.pop(key)
        self.env.update({
            "HOME": str(self.home),
            "USERPROFILE": str(self.home),
            "CODEX_HOME": str(self.home / ".codex"),
            "XDG_CONFIG_HOME": str(self.home / ".config"),
            "XDG_DATA_HOME": str(self.home / ".local/share"),
            "XDG_CACHE_HOME": str(self.home / ".cache"),
            "FORGE_JOB_STORE": str(root / "jobs"),
            "FORGE_PLAN_STORE": str(root / "plans"),
            "PATH": str(self.empty_tools),
            "GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS": str(self.empty_tools),
            "GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS": "1",
        })

    def call(self, args, success=True, parse_error=False, forge=None):
        command = [str(forge or self.forge), *map(str, args), "--json"]
        result = subprocess.run(command, cwd=self.root, env=self.env,
                                capture_output=True, text=True, timeout=60)
        record = {"argv": command, "exitCode": result.returncode,
                  "stdout": result.stdout, "stderr": result.stderr}
        self.calls.append({"argv": command, "exitCode": result.returncode})
        write_json(self.logs / f"{len(self.calls):03d}.json", record)
        if parse_error and not result.stdout.strip():
            require(not success and result.returncode != 0 and result.stderr.strip(),
                    f"Invalid arguments were not rejected: {args}: {record}")
            return None
        try:
            envelope = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            raise AssertionError(f"CLI did not emit one JSON envelope: {args}: {record}") from error
        require(envelope.get("schemaVersion") == "1", f"Invalid envelope: {record}")
        require(envelope.get("ok") is success, f"Unexpected success state: {args}: {record}")
        require((result.returncode == 0) is success, f"Unexpected exit status: {args}: {record}")
        if success:
            require(isinstance(envelope.get("data"), dict), f"Missing JSON data: {record}")
            return envelope["data"]
        require(isinstance(envelope.get("error", {}).get("code"), str),
                f"Missing structured error: {record}")
        return envelope["error"]

    def completed(self, name, **evidence):
        self.cases.append({"name": name, "passed": True, **evidence})

    def project(self, name):
        project = self.root / name
        project.mkdir()
        (project / "project.godot").write_text(
            'config_version=5\n[application]\nconfig/name="Forge skill test"\n',
            encoding="utf-8")
        return project

    @staticmethod
    def target(project):
        return project / ".agents/skills" / SKILL

    def install(self, project, action="installed", forge=None):
        value = self.call(["skill", "install", "--project", project], forge=forge)
        self.assert_result(value, self.target(project), "project", "current")
        require(value.get("action") == action, f"Wrong install action: {value}")
        self.assert_install(self.target(project))
        return value

    def assert_result(self, value, target, scope, status):
        require(value.get("name") == SKILL and value.get("scope") == scope,
                f"Wrong scope/name: {value}")
        require(Path(value.get("target", "")).is_absolute(), f"Nonabsolute target: {value}")
        require(Path(value["target"]) == target, f"Wrong installation target: {value}")
        require(value.get("status") == status, f"Expected {status}: {value}")
        require(isinstance(value.get("issues"), list), f"Missing issues: {value}")
        bundle = value.get("bundle", {})
        for key in ("name", "schemaVersion", "cliVersion", "build", "contentHash"):
            require(bundle.get(key) == self.bundle[key], f"Wrong bundle {key}: {value}")

    def assert_install(self, target):
        manifest = read_json(target / MANIFEST)
        require(manifest.get("managedBy") == "forge-cli", "Wrong manifest owner")
        for key in ("name", "schemaVersion", "cliVersion", "build", "contentHash"):
            require(manifest.get(key) == self.bundle[key], f"Wrong manifest {key}")
        require(manifest.get("files") == self.expected, "Manifest file inventory differs from show")
        actual = set()
        for path in target.rglob("*"):
            require(not path.is_symlink(), f"Installed symlink: {path}")
            if path.is_file():
                actual.add(path.relative_to(target).as_posix())
        require(actual == set(self.expected) | {MANIFEST}, f"Unexpected installed files: {actual}")
        for relative, expected in self.expected.items():
            require(sha256((target / relative).read_bytes()) == expected,
                    f"Installed content differs: {relative}")

    def show(self):
        value = self.call(["skill", "show"])
        require(value.get("name") == SKILL and value.get("schemaVersion") == 1,
                f"Invalid skill identity: {value}")
        require(isinstance(value.get("cliVersion"), str) and isinstance(value.get("build"), dict),
                "show must include compiled CLI identity")
        require(isinstance(value.get("files"), list) and value["files"], "Empty bundle")
        files = {}
        for entry in value["files"]:
            relative = entry["path"]
            parts = PurePosixPath(relative).parts
            require(relative and not relative.startswith("/") and ".." not in parts
                    and "\\" not in relative and relative == PurePosixPath(relative).as_posix(),
                    f"Unsafe bundle path: {relative}")
            require(relative not in files and relative != MANIFEST, f"Duplicate/reserved file: {relative}")
            require(isinstance(entry.get("content"), str), f"Missing full text: {relative}")
            require(sha256(entry["content"].encode()) == entry.get("sha256"),
                    f"show content hash mismatch: {relative}")
            require(not re.search(r"(?:/Users/[^/\s]+/|/home/[^/\s]+/|[A-Za-z]:\\Users\\|file://)",
                                  entry["content"]), f"Private absolute path in bundle: {relative}")
            files[relative] = entry["content"]
        require("SKILL.md" in files, "Bundle has no skill entry point")
        self.expected = {entry["path"]: entry["sha256"] for entry in value["files"]}
        require(content_hash(self.expected) == value.get("contentHash"), "Bundle digest differs")
        self.bundle = value
        # Validate actual relative links against the complete embedded file set.
        links = 0
        for relative, content in files.items():
            if not relative.endswith(".md"):
                continue
            destinations = re.findall(r"\[[^\]]*\]\(([^)]+)\)", content)
            destinations += re.findall(r"^\s*\[[^\]]+\]:\s*(\S+)", content, re.MULTILINE)
            for destination in destinations:
                destination = destination.strip()
                destination = (destination[1:destination.index(">")]
                               if destination.startswith("<") else destination.split()[0])
                parsed = urlsplit(destination)
                if parsed.scheme or parsed.netloc or not parsed.path:
                    continue
                linked = os.path.normpath(str(PurePosixPath(relative).parent / unquote(parsed.path)))
                require(linked in files or any(path.startswith(linked.rstrip("/") + "/") for path in files),
                        f"Broken embedded link: {relative} -> {destination}")
                links += 1
        self.completed("show_complete_bundle", files=len(files), relativeLinks=links,
                       contentHash=value["contentHash"])

    def basic_scopes(self):
        project = self.project("project-scope")
        before = snapshot(project)
        missing = self.call(["skill", "check", "--project", project])
        self.assert_result(missing, self.target(project), "project", "missing")
        require(snapshot(project) == before, "check created project files")
        self.install(project)
        current = self.call(["skill", "check", "--project", project])
        self.assert_result(current, self.target(project), "project", "current")
        before = snapshot(project)
        self.install(project, action="unchanged")
        require(snapshot(project) == before, "Same-version reinstall changed project bytes or mtimes")
        self.completed("project_install_check_and_idempotent_reinstall")

        user_target = self.home / ".agents/skills" / SKILL
        missing = self.call(["skill", "check", "--user"])
        self.assert_result(missing, user_target, "user", "missing")
        installed = self.call(["skill", "install", "--user"])
        self.assert_result(installed, user_target, "user", "current")
        require(installed.get("action") == "installed", f"Wrong user action: {installed}")
        self.assert_install(user_target)
        before = snapshot(self.home)
        again = self.call(["skill", "install", "--user"])
        require(again.get("action") == "unchanged", f"Wrong user reinstall action: {again}")
        self.assert_result(self.call(["skill", "check", "--user"]), user_target, "user", "current")
        require(snapshot(self.home) == before, "User reinstall/check modified existing files")
        self.completed("isolated_user_install_check_and_idempotent_reinstall")

    def blocked_install(self, project, expected_status=None, protected=(), code=None):
        trees = (project, *protected)
        before = [snapshot(tree) for tree in trees]
        if expected_status:
            checked = self.call(["skill", "check", "--project", project])
            self.assert_result(checked, self.target(project), "project", expected_status)
        error = self.call(["skill", "install", "--project", project], success=False)
        if code:
            require(error["code"] == code, f"Wrong refusal: {error}")
        require([snapshot(tree) for tree in trees] == before,
                f"Rejected installation changed files: {project}")

    def modified_and_unmanaged(self):
        for mutation in ("edited", "deleted", "extra", "extra-directory"):
            project = self.project(f"managed-{mutation}")
            self.install(project)
            target = self.target(project)
            if mutation == "edited":
                with (target / "SKILL.md").open("a", encoding="utf-8") as stream:
                    stream.write("\nUser-owned local change.\n")
            elif mutation == "deleted":
                (target / "SKILL.md").unlink()
            elif mutation == "extra":
                (target / "user-notes.md").write_text("Do not overwrite my notes.\n", encoding="utf-8")
            else:
                (target / "user-assets").mkdir()
            self.blocked_install(project, "modified", code="skill_modified")
            self.completed(f"protect_managed_{mutation}")

        project = self.project("unmanaged-directory")
        target = self.target(project)
        target.mkdir(parents=True)
        (target / "SKILL.md").write_text("A user-maintained skill.\n", encoding="utf-8")
        self.blocked_install(project, "unmanaged", code="skill_unmanaged")
        self.completed("protect_unmanaged_directory")

        for mutation in ("invalid-json", "wrong-owner", "wrong-content-hash"):
            project = self.project(f"manifest-{mutation}")
            self.install(project)
            path = self.target(project) / MANIFEST
            if mutation == "invalid-json":
                path.write_text("{not valid json", encoding="utf-8")
            else:
                manifest = read_json(path)
                manifest["managedBy" if mutation == "wrong-owner" else "contentHash"] = "invalid"
                write_json(path, manifest)
            # An invalid receipt may be classified as unmanaged or modified;
            # either way it must never grant permission to overwrite the tree.
            self.blocked_install(project)
            self.completed(f"protect_manifest_{mutation}")

        project = self.project("manifest-traversal")
        self.install(project)
        target = self.target(project)
        manifest = read_json(target / MANIFEST)
        outside = project / "sentinel.txt"
        outside.write_text("Not owned by the skill.\n", encoding="utf-8")
        manifest["files"]["../../../sentinel.txt"] = sha256(outside.read_bytes())
        manifest["contentHash"] = content_hash(manifest["files"])
        write_json(target / MANIFEST, manifest)
        self.blocked_install(project)
        self.completed("protect_manifest_path_traversal")

    def symlinks(self):
        for component in (".agents", ".agents/skills", ".agents/skills/forge-use"):
            slug = component.replace("/", "-").lstrip(".")
            project = self.project(f"symlink-{slug}")
            external = self.root / f"outside-{slug}"
            external.mkdir()
            (external / "sentinel.txt").write_text("Must remain untouched.\n", encoding="utf-8")
            link = project / component
            link.parent.mkdir(parents=True, exist_ok=True)
            link.symlink_to(external, target_is_directory=True)
            self.blocked_install(project, protected=(external,))
            self.completed(f"protect_symlink_{slug}")

        real_project = self.project("real-project-root")
        link = self.root / "symlink-project-root"
        link.symlink_to(real_project, target_is_directory=True)
        self.blocked_install(link, protected=(real_project,))
        self.completed("protect_symlink_project_root")

        for relative in (MANIFEST, "SKILL.md"):
            slug = "manifest" if relative == MANIFEST else "managed-file"
            project = self.project(f"symlink-{slug}")
            self.install(project)
            managed = self.target(project) / relative
            external = self.root / f"outside-{slug}.txt"
            external.write_bytes(managed.read_bytes())
            managed.unlink()
            managed.symlink_to(external)
            self.blocked_install(project, "unmanaged" if relative == MANIFEST else "modified",
                                 protected=(external,))
            self.completed(f"protect_symlink_{slug}")

        directory = next((PurePosixPath(path).parts[0] for path in self.expected if "/" in path), None)
        require(directory is not None, "Bundle has no reference/example directory to protect")
        project = self.project("symlink-managed-directory")
        self.install(project)
        managed = self.target(project) / directory
        external = self.root / "outside-managed-directory"
        shutil.move(str(managed), external)
        managed.symlink_to(external, target_is_directory=True)
        self.blocked_install(project, "modified", protected=(external,))
        self.completed("protect_symlink_managed_directory")

        for component in (".forge-skill-staging", ".forge-skill-backups", ".forge-use.install.lock"):
            project = self.project(f"symlink-{component.lstrip('.')}")
            self.install(project)
            self.synthetic_old(self.target(project))
            agents = project / ".agents"
            protected_path = agents / component
            if protected_path.is_dir():
                # Successful installs leave at most an empty staging parent.
                protected_path.rmdir()
            external = self.root / f"outside-{component.lstrip('.')}"
            if component.endswith(".lock"):
                external.write_text("Existing lock target must not change.\n", encoding="utf-8")
                protected_path.symlink_to(external)
            else:
                external.mkdir()
                (external / "sentinel.txt").write_text("Keep backup/staging targets safe.\n", encoding="utf-8")
                protected_path.symlink_to(external, target_is_directory=True)
            self.blocked_install(project, protected=(external,))
            self.completed(f"protect_symlink_{component.lstrip('.')}")

    @staticmethod
    def synthetic_old(target):
        manifest = read_json(target / MANIFEST)
        skill_file = target / "SKILL.md"
        skill_file.write_text(skill_file.read_text(encoding="utf-8")
                              + "\nSynthetic earlier bundled instructions for upgrade testing.\n", encoding="utf-8")
        manifest["cliVersion"] = "0.0.0-test-previous"
        manifest["files"]["SKILL.md"] = sha256(skill_file.read_bytes())
        manifest["contentHash"] = content_hash(manifest["files"])
        write_json(target / MANIFEST, manifest)
        return manifest

    def upgrade(self):
        project = self.project("older-managed-bundle")
        self.install(project)
        target = self.target(project)
        manifest = self.synthetic_old(target)
        previous = byte_inventory(target)
        checked = self.call(["skill", "check", "--project", project])
        self.assert_result(checked, target, "project", "outdated")
        require(checked.get("installed") == manifest, "check did not report the real older manifest")
        installed = self.install(project, action="updated")
        backup = Path(installed.get("backupPath", ""))
        require(backup.is_absolute() and backup.is_dir() and backup != target,
                f"Update did not retain an explicit backup: {installed}")
        require(backup.is_relative_to(project), "Upgrade backup escaped its project")
        require(byte_inventory(backup) == previous, "Upgrade backup lost prior files or manifest")
        before = snapshot(project)
        self.install(project, action="unchanged")
        require(snapshot(project) == before, "Post-upgrade reinstall changed current files or backup")
        self.completed("upgrade_real_older_content_and_preserve_backup",
                       previousContentHash=manifest["contentHash"], backupPath=str(backup))

    def invalid_arguments(self):
        project = self.project("invalid-arguments")
        invalid = [
            ["skill", "install"],
            ["skill", "check"],
            ["skill", "install", "--project", project, "--user"],
            ["skill", "check", "--project", project, "--user"],
            ["skill", "show", "--not-a-skill-option"],
            ["skill", "install", "--project", project, "--force"],
            ["skill", "install", "--project", project / "does-not-exist"],
            ["skill", "install", "--project", project / "project.godot"],
        ]
        before = snapshot(project)
        for args in invalid:
            self.call(args, success=False, parse_error=True)
        require(snapshot(project) == before, "Invalid arguments modified the project")
        self.completed("invalid_arguments_have_no_side_effects", requests=len(invalid))

    def standalone_and_example(self):
        portable = self.root / "standalone"
        portable.mkdir()
        binary = portable / self.forge.name
        shutil.copyfile(self.forge, binary)
        binary.chmod(0o755)
        require(sha256(binary.read_bytes()) == sha256(self.forge.read_bytes()), "Portable binary copy differs")
        require(list(portable.iterdir()) == [binary], "Portable test accidentally includes bundled resources")
        shown = self.call(["skill", "show"], forge=binary)
        require(shown == self.bundle, "Standalone binary cannot reproduce its embedded bundle")
        project = self.project("standalone-game")
        self.install(project, forge=binary)
        self.assert_result(self.call(["skill", "check", "--project", project], forge=binary),
                           self.target(project), "project", "current")
        self.completed("standalone_binary_install_without_repository_or_media_helpers")

        # Exercise an actual installed request example unchanged. Only its local
        # input files are synthesized; no Provider/Style Lock is invented.
        examples = []
        for relative in sorted(self.expected):
            if not relative.endswith(".json"):
                continue
            try:
                value = read_json(self.target(project) / relative)
            except json.JSONDecodeError:
                continue
            if (isinstance(value, dict) and value.get("kind") in ("icon_set", "prop_set")
                    and value.get("items") and all(isinstance(item.get("path"), str) for item in value["items"])):
                examples.append((relative, value))
        require(examples, "Embedded bundle contains no runnable local prepare-static JSON example")
        verified = []
        for number, (relative, request) in enumerate(examples):
            example_root = self.root / f"example-{number}"
            example_root.mkdir()
            specs = example_root / "asset-specs"
            specs.mkdir()
            copied = specs / PurePosixPath(relative).name
            shutil.copyfile(self.target(project) / relative, copied)
            for index, item in enumerate(request["items"]):
                source = Path(item["path"])
                require(not source.is_absolute(), f"Example requires an absolute user path: {relative}")
                source = (copied.parent / source).resolve()
                require(source.is_relative_to(example_root), f"Example source escapes isolated fixture: {relative}")
                require(source.suffix.lower() == ".png", f"Unexpected example source type: {relative}")
                make_png(source, index)
            plan = self.call(["plan", "prepare-static", "--request", copied], forge=binary)
            require(isinstance(plan.get("token"), str) and plan["token"], "Example did not create a real plan")
            estimate = plan["estimate"]
            require(estimate["providerRequestEstimate"] == 0 and estimate["maximumProviderRequests"] == 0,
                    f"Local example can request a Provider: {plan}")
            verified.append({"path": relative, "kind": request["kind"], "providerRequestEstimate": 0})
        self.assert_install(self.target(project))
        self.completed("installed_local_examples_create_real_zero_provider_plans", examples=verified)

    def run(self):
        try:
            self.show()
            self.basic_scopes()
            self.modified_and_unmanaged()
            self.symlinks()
            self.upgrade()
            self.invalid_arguments()
            self.standalone_and_example()
        except Exception as error:
            write_json(self.root / "summary.json", self.summary(False, str(error)))
            raise
        summary = self.summary(True)
        write_json(self.root / "summary.json", summary)
        print(json.dumps(summary, indent=2))

    def summary(self, ok, error=None):
        value = {"ok": ok, "forge": str(self.forge), "binarySha256": sha256(self.forge.read_bytes()),
                 "build": self.bundle.get("build") if self.bundle else None,
                 "contentHash": self.bundle.get("contentHash") if self.bundle else None,
                 "cases": self.cases, "commandCount": len(self.calls),
                 "providerRequestsExecuted": 0,
                 "scope": "temporary projects/home; synthetic local PNGs; plans only; no Godot or Provider execution"}
        if error:
            value["error"] = error
        return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True,
                        help="Absolute executable path; preserve the public installer symlink")
    parser.add_argument("--output", type=Path, help="Optional new directory retaining test evidence")
    args = parser.parse_args()
    if not args.forge.is_absolute() or not args.forge.is_file():
        parser.error("--forge must name an existing absolute executable path")
    # Deliberately do not call resolve() on args.forge: launcher discovery is part
    # of installed-package acceptance. Test directories may normalize /tmp.
    if args.output:
        args.output.mkdir(parents=True, exist_ok=False)
        Harness(args.forge, args.output.resolve()).run()
    else:
        with tempfile.TemporaryDirectory(prefix="forge-cli-skill-") as temporary:
            Harness(args.forge, Path(temporary).resolve()).run()


if __name__ == "__main__":
    main()
