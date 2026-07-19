//! Driver binary for the argument-comment-lint dylint plugin.
//!
//! Locates the compiled lint library (`*.dll` / `*.so` / `*.dylib`),
//! configures the environment for dylint execution, normalises nightly
//! toolchain filenames, and invokes `cargo dylint`.
//!
//! Usage:
//!   cargo run --bin argument-comment-lint -- --help
//!   cargo run --bin argument-comment-lint -- --path crates/sentinel-core
//!   cargo run --bin argument-comment-lint -- --all-targets
//!
//! Environment:
//!   DYLINT_LIB_PATH   — override the library search path
//!   CARGO_DYLINT      — alternative `cargo-dylint` binary path
//!   RUSTUP_TOOLCHAIN  — toolchain to use (default: nightly-2025-07-15)

use std::env;
use std::path::PathBuf;
use std::process::{exit, Command};

use anyhow::{Context, Result};
use clap::Parser;

/// Dylint-based linter for Rust argument comments.
#[derive(Parser, Debug)]
#[command(name = "argument-comment-lint", about)]
struct Args {
    /// Path to the crate or package to lint.
    #[arg(long, default_value = ".")]
    path: String,

    /// Lint all targets (lib, bins, tests, examples).
    #[arg(long)]
    all_targets: bool,

    /// Run in check-only mode (no fixes).
    #[arg(long)]
    check: bool,

    /// Only run the specified lint (default: all).
    #[arg(long)]
    lint: Option<String>,

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Locate the dylint library
    let lib_path = find_lint_lib().context("failed to locate argument-comment-lint library")?;
    if args.verbose {
        eprintln!("[driver] Library: {}", lib_path.display());
    }

    // 2. Configure environment
    env::set_var("DYLINT_LIB_PATH", &lib_path);
    env::set_var("CARGO_INCREMENTAL", "0");

    // 3. Normalise nightly toolchain for dylint's library loading
    let toolchain = normalise_toolchain(env::var("RUSTUP_TOOLCHAIN").ok());
    if let Some(ref tc) = toolchain {
        env::set_var("RUSTUP_TOOLCHAIN", tc);
        if args.verbose {
            eprintln!("[driver] Toolchain: {tc}");
        }
    }

    // 4. Set DYLINT_RUSTFLAGS for linking the rustc_private libs
    let dylint_rustflags = env::var("DYLINT_RUSTFLAGS").unwrap_or_else(|_| {
        [
            "-Zalways-encode-mir",
            "-Zcross-crate-linting",
            "-Zunstable-options",
        ]
        .join(" ")
    });
    env::set_var("DYLINT_RUSTFLAGS", &dylint_rustflags);

    // 5. Build cargo-dylint command
    let cargo_dylint = env::var("CARGO_DYLINT").unwrap_or_else(|_| "cargo".to_string());

    let mut cmd = Command::new(&cargo_dylint);
    cmd.arg("dylint");

    // Lint selection
    if let Some(ref name) = args.lint {
        cmd.arg(name);
    } else {
        cmd.arg("argument-comment-lint");
    }

    cmd.arg("--lib").arg(&lib_path);
    cmd.arg("--path").arg(&args.path);

    if args.all_targets {
        cmd.arg("--all-targets");
    }

    if args.check {
        cmd.arg("--check");
    }

    // Forward rustflags
    cmd.env("RUSTFLAGS", &dylint_rustflags);

    if args.verbose {
        eprintln!("[driver] Running: {cmd:?}");
    }

    let status = cmd.status().context("failed to execute cargo-dylint")?;
    if !status.success() {
        eprintln!("[driver] cargo-dylint exited with code {}", status.code().unwrap_or(-1));
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}

/// Locate the compiled dylint library (cdylib) on disk.
///
/// Search order:
///   1. `DYLINT_LIB_PATH` env var
///   2. `target/release/libargument_comment_lint.*`
///   3. `target/debug/libargument_comment_lint.*`
///   4. Adjacent to the driver binary
fn find_lint_lib() -> Result<PathBuf> {
    if let Ok(path) = env::var("DYLINT_LIB_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
    }

    let search_paths = [
        // Relative to manifest dir
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug"),
    ];

    let lib_name = if cfg!(target_os = "windows") {
        "argument_comment_lint.dll"
    } else if cfg!(target_os = "macos") {
        "libargument_comment_lint.dylib"
    } else {
        "libargument_comment_lint.so"
    };

    for base in &search_paths {
        let candidate = base.join(lib_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "argument-comment-lint library not found at any of: {paths:?}/{lib_name}",
        paths = search_paths,
        lib_name = lib_name,
    )
}

/// Normalise nightly toolchain names for dylint compatibility.
///
/// dylint expects the toolchain name to match the directory name under
/// `$HOME/.rustup/toolchains/`.  Some setups use `nightly-YYYY-MM-DD`
/// while others use just `nightly`.  This function resolves the latter
/// to the pinned date.
fn normalise_toolchain(current: Option<String>) -> Option<String> {
    let current = current?;

    // If it already has a date suffix, return as-is
    if current.contains('-') {
        return Some(current);
    }

    // If it's just "nightly", try to resolve from rust-toolchain.toml
    if current == "nightly" {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let toolchain_file = manifest_dir.join("rust-toolchain.toml");
        if toolchain_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&toolchain_file) {
                for line in content.lines() {
                    if line.trim().starts_with("channel") {
                        let channel = line.split('=').nth(1)?.trim().trim_matches('"');
                        if channel.contains('-') {
                            return Some(channel.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fallback: return whatever was given
    Some(current)
}
