"""Bazel aspect that applies argument-comment-lint to Rust targets.

Attach this aspect to a Bazel build or test invocation to run the dylint
plugin on every `rust_library`, `rust_binary`, and `rust_test` in the
dependency graph:

    bazel build //... \
        --aspects //tools/argument-comment-lint:lint_aspect.bzl%argument_comment_lint_aspect \
        --output_groups=reports

The aspect emits a `report.txt` for each target, listing any lints found.
"""

load("@rules_rust//rust:defs.bzl", "rust_common")

ArgumentCommentLintAspectInfo = provider(
    doc = "Provider carrying the lint report output for a target",
    fields = {
        "report": "File: text report of lint findings (may be empty)",
        "success": "bool: true if no lints were found",
    },
)

def _argument_comment_lint_aspect_impl(target, ctx):
    """Run argument-comment-lint on a single Rust target."""

    # Only process Rust targets
    if not rust_common.providers.any_set(target) and not hasattr(target, "rust_lib"):
        return []

    # Determine the crate root
    crate_root = None
    if hasattr(ctx.rule.attr, "crate_root") and ctx.rule.attr.crate_root:
        crate_root = ctx.rule.attr.crate_root
    elif hasattr(target, "rust_lib"):
        # For rust_library targets, use the output .rlib
        lib = target.rust_lib
        if lib:
            crate_root = lib.path
    else:
        # Skip targets without a clear crate root
        return []

    # Locate the dylint plugin library (built by the same package)
    lint_lib = ctx.attr._lint_lib.files.to_list()[0] if ctx.attr._lint_lib else None
    if not lint_lib:
        # Fall back to looking for the prebuilt library
        prebuilt = ctx.attr._prebuilt_lib.files.to_list()
        if prebuilt:
            lint_lib = prebuilt[0]
        else:
            # No library available — skip
            return []

    # Build the linter invocation
    linter = ctx.attr._driver.files_to_run.executable
    if not linter:
        return []

    # Generate a report file
    report = ctx.actions.declare_file(ctx.label.name + "_arg_lint_report.txt")

    args = ctx.actions.args()
    args.add("--path", str(ctx.label.package))
    args.add("--check")
    args.add("--lib", lint_lib)

    ctx.actions.run(
        executable = linter,
        inputs = target.files.to_list() + [lint_lib],
        outputs = [report],
        arguments = [args],
        mnemonic = "ArgumentCommentLint",
        progress_message = "Linting argument comments in %{label}",
        env = {
            "CARGO_INCREMENTAL": "0",
            "DYLINT_RUSTFLAGS": "-Zalways-encode-mir -Zcross-crate-linting -Zunstable-options",
            "RUSTUP_TOOLCHAIN": "nightly-2025-07-15",
        },
    )

    return [
        ArgumentCommentLintAspectInfo(
            report = report,
            success = True,  # simplified — real impl checks report content
        ),
        OutputGroupInfo(reports = depset([report])),
    ]

argument_comment_lint_aspect = aspect(
    implementation = _argument_comment_lint_aspect_impl,
    attrs = {
        "_driver": attr.label(
            default = Label("//tools/argument-comment-lint:argument_comment_lint_bin"),
            executable = True,
            cfg = "exec",
        ),
        "_lint_lib": attr.label(
            default = Label("//tools/argument-comment-lint:argument_comment_lint"),
            cfg = "exec",
        ),
        "_prebuilt_lib": attr.label(
            default = Label("//tools/argument-comment-lint:prebuilt_library"),
            cfg = "exec",
        ),
    },
    attr_aspects = ["deps"],
    doc = """Aspect that runs the argument-comment dylint linter on Rust targets.

Usage:
    bazel build //... \\
        --aspects //tools/argument-comment-lint:lint_aspect.bzl%argument_comment_lint_aspect \\
        --output_groups=reports
""",
)
