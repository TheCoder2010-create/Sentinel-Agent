#!/usr/bin/env python3
"""Python wrapper for the argument-comment-lint dylint plugin (source build).

Builds the lint library from source (if needed) and invokes it via
`cargo-dylint` on the specified targets.  Ensures `rustup` shims are
prioritised on PATH so that the nightly toolchain with `rustc_private`
is used.

Usage:
    python3 tools/argument-comment-lint/run.py --path crates/sentinel-core
    python3 tools/argument-comment-lint/run.py --all-targets
    python3 tools/argument-comment-lint/run.py --check --lint argument_comment_mismatch
"""

import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path


def find_cargo_dylint() -> str:
    """Locate cargo-dylint or install it on demand."""
    cargo_dylint = shutil.which("cargo-dylint")
    if cargo_dylint:
        return cargo_dylint

    # Try installing
    print("[run] cargo-dylint not found; installing via cargo…")
    subprocess.run(
        ["cargo", "install", "cargo-dylint"],
        check=True,
    )
    cargo_dylint = shutil.which("cargo-dylint")
    if not cargo_dylint:
        print("[run] ERROR: cargo-dylint still not found after install", file=sys.stderr)
        sys.exit(1)
    return cargo_dylint


def ensure_rustup_shims_on_path() -> None:
    """Prepend rustup's bin directory to PATH so the right toolchain is used."""
    rustup_home = os.environ.get("RUSTUP_HOME", Path.home() / ".rustup")
    toolchains_dir = Path(rustup_home) / "toolchains"

    # Find the nightly toolchain
    nightly = None
    if toolchains_dir.exists():
        for tc in toolchains_dir.iterdir():
            if tc.name.startswith("nightly"):
                nightly = tc
                break

    if nightly:
        bin_dir = nightly / "bin"
        if bin_dir.exists():
            current_path = os.environ.get("PATH", "")
            os.environ["PATH"] = f"{bin_dir}{os.pathsep}{current_path}"
            print(f"[run] Prepended rustup nightly shims: {bin_dir}")
            return

    print("[run] WARNING: no nightly toolchain found under {toolchains_dir}")


def build_lint_lib(lint_dir: Path) -> Path:
    """Build the dylint library and return the path to the cdylib."""
    print("[run] Building argument-comment-lint library…")

    # Use nightly toolchain
    env = os.environ.copy()
    env["RUSTUP_TOOLCHAIN"] = "nightly-2025-07-15"

    subprocess.run(
        ["cargo", "build", "--release", "--lib"],
        cwd=str(lint_dir),
        env=env,
        check=True,
    )

    lib_name = {
        "Windows": "argument_comment_lint.dll",
        "Darwin": "libargument_comment_lint.dylib",
    }.get(platform.system(), "libargument_comment_lint.so")

    lib_path = lint_dir / "target" / "release" / lib_name
    if not lib_path.exists():
        print(f"[run] ERROR: library not found at {lib_path}", file=sys.stderr)
        sys.exit(1)

    print(f"[run] Library: {lib_path}")
    return lib_path


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Argument-comment-lint runner (source build)",
    )
    parser.add_argument("--path", default=os.getcwd(), help="Path to lint")
    parser.add_argument("--all-targets", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--lint", help="Specific lint to run")
    parser.add_argument("--no-build", action="store_true", help="Skip building")
    args = parser.parse_args()

    lint_dir = Path(__file__).resolve().parent.parent.parent / "tools" / "argument-comment-lint"

    # Ensure nightly toolchain is on PATH
    ensure_rustup_shims_on_path()

    # Build the lint library
    lib_path = build_lint_lib(lint_dir) if not args.no_build else None

    # Ensure cargo-dylint is available
    find_cargo_dylint()

    # Build the cargo-dylint command
    cmd = ["cargo", "dylint", "argument-comment-lint"]
    if lib_path:
        cmd += ["--lib", str(lib_path)]
    cmd += ["--path", args.path]
    if args.all_targets:
        cmd.append("--all-targets")
    if args.check:
        cmd.append("--check")
    if args.lint:
        cmd.append("--lint")
        cmd.append(args.lint)

    # Set env vars for dylint
    env = os.environ.copy()
    env["CARGO_INCREMENTAL"] = "0"
    env["RUSTUP_TOOLCHAIN"] = env.get("RUSTUP_TOOLCHAIN", "nightly-2025-07-15")
    env["DYLINT_RUSTFLAGS"] = env.get(
        "DYLINT_RUSTFLAGS",
        "-Zalways-encode-mir -Zcross-crate-linting -Zunstable-options",
    )

    print(f"[run] Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, env=env)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
