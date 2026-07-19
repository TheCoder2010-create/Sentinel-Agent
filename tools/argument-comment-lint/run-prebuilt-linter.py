#!/usr/bin/env python3
"""Python wrapper for the argument-comment-lint dylint plugin (pre-built).

Uses a pre-compiled lint library (`.so` / `.dylib` / `.dll`) instead of
building from source.  Useful in CI or when distributing the linter as
a binary artifact.

Usage:
    # Use a pre-built library from a known path
    python3 tools/argument-comment-lint/run-prebuilt-linter.py \\
        --lib tools/argument-comment-lint/target/release/libargument_comment_lint.so

    # Use the default search (target/release or target/debug)
    python3 tools/argument-comment-lint/run-prebuilt-linter.py --path crates/sentinel-core
"""

import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path


def find_prebuilt_lib(lint_dir: Path) -> Path:
    """Search standard locations for a pre-built dylint library."""
    lib_name = {
        "Windows": "argument_comment_lint.dll",
        "Darwin": "libargument_comment_lint.dylib",
    }.get(platform.system(), "libargument_comment_lint.so")

    candidates = [
        lint_dir / "target" / "release" / lib_name,
        lint_dir / "target" / "debug" / lib_name,
        lint_dir / lib_name,
    ]

    for c in candidates:
        if c.exists():
            return c

    print(
        f"[run-prebuilt] ERROR: pre-built library not found; searched:",
        file=sys.stderr,
    )
    for c in candidates:
        print(f"  {c}", file=sys.stderr)
    print("[run-prebuilt] Build it with: cd tools/argument-comment-lint && cargo build --release", file=sys.stderr)
    sys.exit(1)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Argument-comment-lint runner (pre-built)",
    )
    parser.add_argument("--lib", help="Path to pre-built dylint library")
    parser.add_argument("--path", default=os.getcwd(), help="Path to lint")
    parser.add_argument("--all-targets", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--lint", help="Specific lint to run")
    args = parser.parse_args()

    lint_dir = Path(__file__).resolve().parent.parent.parent / "tools" / "argument-comment-lint"

    # Locate pre-built library
    lib_path = args.lib if args.lib else find_prebuilt_lib(lint_dir)
    if not Path(lib_path).exists():
        print(f"[run-prebuilt] ERROR: library not found at {lib_path}", file=sys.stderr)
        sys.exit(1)
    print(f"[run-prebuilt] Library: {lib_path}")

    # Ensure cargo-dylint is available
    cargo_dylint = shutil.which("cargo-dylint")
    if not cargo_dylint:
        print("[run-prebuilt] Installing cargo-dylint…")
        subprocess.run(["cargo", "install", "cargo-dylint"], check=True)
        cargo_dylint = shutil.which("cargo-dylint")

    # Build command
    cmd = [cargo_dylint, "dylint", "argument-comment-lint"]
    cmd += ["--lib", str(lib_path)]
    cmd += ["--path", args.path]
    if args.all_targets:
        cmd.append("--all-targets")
    if args.check:
        cmd.append("--check")
    if args.lint:
        cmd += ["--lint", args.lint]

    env = os.environ.copy()
    env["CARGO_INCREMENTAL"] = "0"
    env["DYLINT_RUSTFLAGS"] = env.get(
        "DYLINT_RUSTFLAGS",
        "-Zalways-encode-mir -Zcross-crate-linting -Zunstable-options",
    )

    print(f"[run-prebuilt] Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, env=env)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
