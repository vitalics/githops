#!/usr/bin/env python3
"""
Generate the GitHub Release body (release-body.md).

Usage (called by .github/workflows/release.yml):
    python3 scripts/release.py

Required environment variables:
    RELEASE_VERSION   — e.g. "1.2.3"
    RELEASE_TAG       — e.g. "v1.2.3"
    RELEASE_REPO      — e.g. "vitalics/githops"
    CHANGELOG         — multiline list of commits (may be empty)

Output:
    release-body.md written to the current working directory.
"""

import os
import sys


def main() -> None:
    version   = os.environ["RELEASE_VERSION"]
    tag       = os.environ["RELEASE_TAG"]
    repo      = os.environ["RELEASE_REPO"]
    changelog = os.environ.get("CHANGELOG", "").strip()

    base = f"https://github.com/{repo}/releases/download/{tag}"

    lines = [
        "## What's Changed",
        "",
        changelog or "_No commits found._",
        "",
        "---",
        "",
        "## Installation",
        "",
        "### Already installed — update in one command",
        "```sh",
        "githops self-update",
        "```",
        "",
        "### macOS",
        "Run the `.pkg` installer — places `githops` in `/usr/local/bin` automatically:",
        "```sh",
        "# Apple Silicon",
        f"curl -fsSL {base}/githops-{tag}-aarch64-apple-darwin.pkg -o githops.pkg",
        "sudo installer -pkg githops.pkg -target /",
        "",
        "# Intel",
        f"curl -fsSL {base}/githops-{tag}-x86_64-apple-darwin.pkg -o githops.pkg",
        "sudo installer -pkg githops.pkg -target /",
        "```",
        "",
        "### Linux",
        "```sh",
        "# x86_64",
        f"curl -fsSL {base}/githops-{tag}-x86_64-unknown-linux-gnu.tar.gz | tar xz",
        "sudo mv githops /usr/local/bin/",
        "",
        "# ARM64",
        f"curl -fsSL {base}/githops-{tag}-aarch64-unknown-linux-gnu.tar.gz | tar xz",
        "sudo mv githops /usr/local/bin/",
        "```",
        "",
        "### Windows",
        f"Download and run **`githops-{tag}-x86_64-pc-windows-msvc.msi`** —",
        "installs `githops` and adds it to `PATH` automatically.",
    ]

    out = "release-body.md"
    with open(out, "w") as f:
        f.write("\n".join(lines) + "\n")

    print(f"Written {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
