#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

python3 - <<'PY'
from __future__ import annotations

import json
import re
import sys
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path.cwd()
VERSION_RE = re.compile(r"^(?P<prefix>[=^~]?)(?P<version>\d+\.\d+\.\d+)$")
SECTION_NAMES = ("dependencies", "dev-dependencies", "build-dependencies")


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def workspace_manifests() -> list[tuple[str, dict]]:
    root_doc = load_toml(ROOT / "Cargo.toml")
    manifests: list[tuple[str, dict]] = [("workspace", root_doc)]

    for member in root_doc.get("workspace", {}).get("members", []):
        manifests.append((member, load_toml(ROOT / member / "Cargo.toml")))

    return manifests


def dependency_sections(doc: dict) -> list[tuple[str, dict]]:
    sections: list[tuple[str, dict]] = []

    for section_name in SECTION_NAMES:
        section = doc.get(section_name)
        if isinstance(section, dict):
            sections.append((section_name, section))

    targets = doc.get("target", {})
    if isinstance(targets, dict):
        for target_name, target_doc in targets.items():
            if not isinstance(target_doc, dict):
                continue
            for section_name in SECTION_NAMES:
                section = target_doc.get(section_name)
                if isinstance(section, dict):
                    sections.append((f"target.{target_name}.{section_name}", section))

    return sections


def parse_dependency(alias: str, spec: object) -> tuple[str, str, str] | None:
    if isinstance(spec, str):
        return alias, alias, spec

    if not isinstance(spec, dict):
        return None
    if spec.get("workspace") is True:
        return None

    version = spec.get("version")
    if not isinstance(version, str):
        return None

    package_name = spec.get("package", alias)
    if not isinstance(package_name, str):
        package_name = alias

    return alias, package_name, version


def direct_dependencies(manifest_label: str, doc: dict) -> list[tuple[str, str, str, str]]:
    deps: list[tuple[str, str, str, str]] = []

    if manifest_label == "workspace":
        workspace_deps = doc.get("workspace", {}).get("dependencies", {})
        if isinstance(workspace_deps, dict):
            for alias, spec in workspace_deps.items():
                parsed = parse_dependency(alias, spec)
                if parsed is not None:
                    deps.append(("workspace.dependencies", *parsed))

    for section_name, section in dependency_sections(doc):
        for alias, spec in section.items():
            parsed = parse_dependency(alias, spec)
            if parsed is not None:
                deps.append((section_name, *parsed))

    return deps


def parse_pinned_requirement(requirement: str) -> tuple[str, tuple[int, int, int]] | None:
    match = VERSION_RE.fullmatch(requirement.strip())
    if match is None:
        return None

    prefix = match.group("prefix")
    if prefix in {"^", "~"}:
        return None

    version = tuple(int(part) for part in match.group("version").split("."))
    return requirement.strip(), version


def fetch_latest_stable(crate_name: str) -> tuple[str, tuple[int, int, int]]:
    url = f"https://crates.io/api/v1/crates/{urllib.parse.quote(crate_name)}"
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "venom-dependency-freshness-check"},
    )

    with urllib.request.urlopen(request, timeout=20) as response:
        payload = json.load(response)

    latest = payload["crate"]["max_stable_version"]
    return latest, tuple(int(part) for part in latest.split("."))


def is_non_breaking_newer(current: tuple[int, int, int], latest: tuple[int, int, int]) -> bool:
    if latest <= current:
        return False
    if latest[0] > current[0]:
        return False
    if current[0] == 0 and latest[1] > current[1]:
        return False
    return True


updates: list[tuple[str, str, str, str, str, str]] = []

try:
    for manifest_label, doc in workspace_manifests():
        for section_name, alias, package_name, requirement in direct_dependencies(manifest_label, doc):
            pinned = parse_pinned_requirement(requirement)
            if pinned is None:
                continue

            current_display, current_version = pinned
            latest_display, latest_version = fetch_latest_stable(package_name)

            if is_non_breaking_newer(current_version, latest_version):
                updates.append(
                    (
                        manifest_label,
                        section_name,
                        alias,
                        package_name,
                        current_display,
                        latest_display,
                    )
                )
except urllib.error.URLError as error:
    print(f"ERROR: crates.io query failed: {error}", file=sys.stderr)
    sys.exit(1)

if updates:
    print("Non-breaking direct dependency updates available:")
    for manifest_label, section_name, alias, package_name, current_display, latest_display in updates:
        package_suffix = f" ({package_name})" if package_name != alias else ""
        print(
            f"- {manifest_label} [{section_name}]: {alias}{package_suffix} "
            f"{current_display} -> {latest_display}"
        )
    sys.exit(1)
PY

trap - ERR
echo "RESULT: PASS"
