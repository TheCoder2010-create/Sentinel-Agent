#!/usr/bin/env python3
"""Post-installation script for Sentinel AI dev containers.

Handles:
- Persistent shell history setup (mounted volume)
- Directory ownership fixes for bind-mounted workspaces
- Git configuration for the monorepo
- Cargo registry directory fix
- Rustup component validation
- Codex CLI installation (if pre-built binary available)
"""
import os
import pwd
import shutil
import subprocess
import stat

HOME = os.environ.get("HOME", "/home/vscode")
USER = os.environ.get("USER", "vscode")


def fix_ownership(path: str) -> None:
    """Ensure the given path is owned by the container user."""
    try:
        uid = pwd.getpwnam(USER).pw_uid
        gid = pwd.getpwnam(USER).pw_gid
        os.chown(path, uid, gid)
    except (KeyError, PermissionError, OSError):
        pass


def setup_history() -> None:
    """Configure persistent shell history via mounted volume."""
    history_dir = "/commandhistory"
    if os.path.isdir(history_dir):
        bash_history = os.path.join(history_dir, ".bash_history")
        for rc_file, hist_file, hist_var in [
            (os.path.join(HOME, ".bashrc"), bash_history, "HISTFILE"),
            (os.path.join(HOME, ".zshrc"), bash_history, "HISTFILE"),
        ]:
            if os.path.isfile(rc_file):
                with open(rc_file, "a") as f:
                    f.write(f"\nexport {hist_var}={hist_file}\n")
                    f.write("export HISTSIZE=100000\n")
                    f.write("export HISTFILESIZE=100000\n")
                    f.write("export HISTCONTROL=ignoreboth:erasedups\n")
        fix_ownership(history_dir)
        print("[post_install] Shell history configured")


def setup_git() -> None:
    """Configure Git for the monorepo."""
    git_config = {
        "core.autocrlf": "input",
        "core.symlinks": "true",
        "core.fsmonitor": "true",
        "pull.rebase": "true",
        "fetch.prune": "true",
        "diff.renameLimit": "9999",
        "safe.directory": "/workspaces/*",
    }
    for key, value in git_config.items():
        subprocess.run(["git", "config", "--global", key, value], capture_output=True)
    if shutil.which("git-lfs"):
        subprocess.run(["git", "lfs", "install", "--skip-repo"], capture_output=True)
    print("[post_install] Git configured")


def setup_cargo() -> None:
    """Ensure Cargo registry directory exists and is writable."""
    cargo_registry = os.path.join(HOME, ".cargo", "registry")
    os.makedirs(cargo_registry, exist_ok=True)
    fix_ownership(os.path.join(HOME, ".cargo"))
    fix_ownership(cargo_registry)

    # Create Cargo config if missing
    cargo_config = os.path.join(HOME, ".cargo", "config.toml")
    if not os.path.isfile(cargo_config):
        config_content = """
[net]
git-fetch-with-cli = true

[registry]
global-credential-providers = ["cargo:token"]
"""
        with open(cargo_config, "w") as f:
            f.write(config_content)
        fix_ownership(cargo_config)
    print("[post_install] Cargo configured")


def validate_rustup() -> None:
    """Ensure required Rust components are installed."""
    try:
        result = subprocess.run(
            ["rustup", "component", "list", "--installed"],
            capture_output=True, text=True, check=True,
        )
        installed = result.stdout.splitlines()
        required = ["clippy", "rustfmt", "rust-analyzer"]
        for component in required:
            if component not in installed:
                print(f"[post_install] Installing missing Rust component: {component}")
                subprocess.run(
                    ["rustup", "component", "add", component],
                    check=True, capture_output=True,
                )
        # Ensure musl targets
        result = subprocess.run(
            ["rustup", "target", "list", "--installed"],
            capture_output=True, text=True, check=True,
        )
        targets = result.stdout.splitlines()
        if "x86_64-unknown-linux-musl" not in targets:
            print("[post_install] Adding musl cross-compile target")
            subprocess.run(
                ["rustup", "target", "add", "x86_64-unknown-linux-musl"],
                check=True, capture_output=True,
            )
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("[post_install] WARNING: rustup not available — skipping component validation")


def install_codex_cli() -> None:
    """Install pre-built Sentinel CLI binary if available."""
    search_paths = [
        "/workspaces/sentinel/target/release/sentinel",
        "/workspaces/sentinel/target/debug/sentinel",
        os.path.join(HOME, "sentinel/target/release/sentinel"),
        os.path.join(HOME, "sentinel/target/debug/sentinel"),
    ]
    for binary_path in search_paths:
        if os.path.isfile(binary_path):
            install_path = "/usr/local/bin/sentinel"
            try:
                shutil.copy2(binary_path, install_path)
                os.chmod(install_path, stat.S_IRWXU | stat.S_IRGRP | stat.S_IXGRP | stat.S_IROTH | stat.S_IXOTH)
                print(f"[post_install] Sentinel CLI installed from {binary_path}")
                return
            except (OSError, PermissionError):
                pass
    print("[post_install] Sentinel CLI not pre-built — skip (build with 'cargo build --release')")


def main() -> None:
    print("[post_install] Starting…")
    setup_history()
    setup_git()
    setup_cargo()
    validate_rustup()
    install_codex_cli()
    for path in [HOME, os.path.join(HOME, ".cargo"), os.path.join(HOME, ".config")]:
        if os.path.exists(path):
            fix_ownership(path)
    print("[post_install] Done")


if __name__ == "__main__":
    main()
