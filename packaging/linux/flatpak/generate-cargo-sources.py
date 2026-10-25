#!/usr/bin/env python3
"""Regenerate cargo-sources.json for the from-source Flatpak manifest.

The upstream flatpak-cargo-generator is downloaded from a pinned commit and
verified before use; the full script is intentionally not vendored here. Two
small, reviewed patches are applied in memory:

1. Key cargo source replacements by the full source URL (including ?rev=...)
   instead of only the repository URL, so two different revisions of the same
   repository stay distinct.
2. Include Cargo.lock's [patch.unused] entries, so crates such as egui_demo_lib
   are vendored even though they are not in the resolved package graph.

This only produces and validates the source inventory; a full Flatpak SDK
build is still required. See README.md in this directory for details.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from urllib.request import urlopen

UPSTREAM_COMMIT = "9d86d336a39b9af38b86f114b6ce5820fe260f91"
UPSTREAM_URL = (
    "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/"
    f"{UPSTREAM_COMMIT}/cargo/flatpak-cargo-generator.py"
)
UPSTREAM_SHA256 = "0a2db6be87d75910facef28ab46d4d6460802e8419ab850d0caa6a364d26b380"
PATCHED_SHA256 = "1e7c61d98d18e88156fb8cd4b91f526e04a6c504b185ae7b12f775ec1dd48512"

PATCHES = (
    (
        "preserve pinned source identities",
        '''    elif branch:
        assert len(branch) == 1
        cargo_vendored_entry[repo_url]["branch"] = branch[0]

    logging.info("Adding package %s from %s", name, repo_url)
''',
        '''    elif branch:
        assert len(branch) == 1
        cargo_vendored_entry[repo_url]["branch"] = branch[0]

    # Preserve distinct pinned source identities from the same repository.
    if rev or tag or branch:
        source_key = source.removeprefix("git+").split("#", 1)[0]
        cargo_vendored_entry = {source_key: cargo_vendored_entry[repo_url]}

    logging.info("Adding package %s from %s", name, repo_url)
''',
    ),
    (
        "include Cargo.lock patch.unused entries",
        '''    pkg_coros = [
        get_package_sources(p, cargo_lock, git_repos) for p in cargo_lock["package"]
    ]
''',
        '''    pkg_coros = [
        get_package_sources(p, cargo_lock, git_repos)
        for p in cargo_lock["package"] + cargo_lock.get("patch", {}).get("unused", [])
    ]
''',
    ),
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def download_upstream() -> bytes:
    with urlopen(UPSTREAM_URL, timeout=60) as response:
        data = response.read(1_000_001)
    if len(data) > 1_000_000:
        raise SystemExit("error: upstream generator is unexpectedly large")
    actual = sha256(data)
    if actual != UPSTREAM_SHA256:
        raise SystemExit(
            "error: upstream generator checksum mismatch\n"
            f"  expected {UPSTREAM_SHA256}\n"
            f"  actual   {actual}\n"
            f"  url      {UPSTREAM_URL}"
        )
    return data


def patched_upstream(data: bytes) -> bytes:
    text = data.decode("utf-8")
    for name, old, new in PATCHES:
        count = text.count(old)
        if count != 1:
            raise SystemExit(
                f"error: patch {name!r} matched {count} times, expected exactly once; "
                "the pinned upstream script or patch set changed"
            )
        text = text.replace(old, new, 1)
    patched = text.encode("utf-8")
    actual = sha256(patched)
    if actual != PATCHED_SHA256:
        raise SystemExit(
            "error: patched generator checksum mismatch\n"
            f"  expected {PATCHED_SHA256}\n"
            f"  actual   {actual}"
        )
    return patched


def default_cache_dir() -> Path:
    base = os.environ.get("XDG_CACHE_HOME")
    if base:
        return Path(base) / "lightcraft-flatpak-cargo"
    return Path.home() / ".cache" / "lightcraft-flatpak-cargo"


def ensure_outside_source(cache: Path, repo_root: Path) -> None:
    cache = cache.resolve()
    if cache == repo_root or repo_root in cache.parents:
        raise SystemExit(f"error: cache directory must be outside the source checkout: {cache}")
    target = os.environ.get("CARGO_TARGET_DIR")
    if target:
        target_path = Path(target)
        # Cargo runs in repo_root, even when this wrapper was invoked elsewhere.
        if not target_path.is_absolute():
            target_path = repo_root / target_path
        target_path = target_path.resolve()
        if cache == target_path or target_path in cache.parents:
            raise SystemExit(f"error: cache directory must be outside CARGO_TARGET_DIR: {cache}")


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "cargo_lock",
        nargs="?",
        default=str(repo_root / "Cargo.lock"),
        help="Cargo.lock to read (default: repository Cargo.lock)",
    )
    parser.add_argument(
        "-o",
        "--output",
        default=str(repo_root / "packaging/linux/flatpak/cargo-sources.json"),
        help="output path (default: packaging/linux/flatpak/cargo-sources.json)",
    )
    parser.add_argument(
        "--cache-dir",
        default=None,
        help=(
            "git clone cache root (default: $XDG_CACHE_HOME/lightcraft-flatpak-cargo, "
            "or ~/.cache/lightcraft-flatpak-cargo)"
        ),
    )
    args = parser.parse_args()

    cargo_lock = Path(args.cargo_lock).resolve()
    output = Path(args.output).resolve()
    cache = Path(args.cache_dir).resolve() if args.cache_dir else default_cache_dir().resolve()
    if not cargo_lock.is_file():
        raise SystemExit(f"error: Cargo.lock not found: {cargo_lock}")
    ensure_outside_source(cache, repo_root)
    cache.mkdir(parents=True, exist_ok=True)
    output.parent.mkdir(parents=True, exist_ok=True)

    generator = patched_upstream(download_upstream())

    with tempfile.TemporaryDirectory(prefix="lightcraft-flatpak-generator-") as tmpdir:
        script = Path(tmpdir) / "flatpak-cargo-generator.py"
        script.write_bytes(generator)

        descriptor, temp_output = tempfile.mkstemp(
            prefix=f".{output.name}.", suffix=".tmp", dir=output.parent
        )
        os.close(descriptor)
        try:
            environment = os.environ.copy()
            environment["XDG_CACHE_HOME"] = str(cache)
            result = subprocess.run(
                [sys.executable, str(script), str(cargo_lock), "-o", temp_output],
                cwd=str(repo_root),
                env=environment,
                check=False,
            )
            if result.returncode != 0:
                return result.returncode
            if os.path.getsize(temp_output) == 0:
                raise SystemExit("error: generator produced an empty file")
            with open(temp_output, "r", encoding="utf-8") as handle:
                sources = json.load(handle)
            if not isinstance(sources, list) or not sources:
                raise SystemExit("error: generated sources are not a non-empty list")
            os.chmod(temp_output, 0o644)
            os.replace(temp_output, output)
        finally:
            if os.path.exists(temp_output):
                os.unlink(temp_output)

    print(f"wrote {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
