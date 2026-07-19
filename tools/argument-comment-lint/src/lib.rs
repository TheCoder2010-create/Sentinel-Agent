//! Dylint lint library: argument comment convention checker.
//!
//! Provides two lint rules:
//!
//! **`argument_comment_mismatch`** (allow-by-default)
//! Checks that `/*param*/` block-comments at call sites match the resolved
//! parameter name of the function being called.  Stale or incorrect inline
//! documentation is flagged so that comments stay correct as code evolves.
//!
//! **`uncommented_anonymous_literal_argument`** (allow-by-default)
//! Flags literal-like arguments (`None`, `true`, `false`, numeric literals)
//! that are passed **without** a preceding `/*param*/` comment.  Exemptions:
//!   - String / char literals (`"foo"`, `'x'`)
//!   - Self-documenting method calls where the argument name is the same as
//!     the parameter name (e.g. `vec![1, 2, 3]`).

#![feature(rustc_private)]
#![allow(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use dylint_linting::{dylint_lint, declare_dylint_lint};
use rustc_hir::{Expr, ExprKind, HirId, PathSegment, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_session::declare_lint;
use rustc_span::source_map::Span;
use rustc_span::symbol::Ident;

use std::collections::HashMap;

// ============================================================================
// Lint declarations
// ============================================================================

declare_lint! {
    /// **What it does:** Checks that `/*param*/` block-comments at call
    /// sites match the corresponding parameter name.
    ///
    /// **Why is this bad?** When parameter names change but inline comments
    /// are not updated, readers can be misled about which value corresponds
    /// to which parameter.  Stale comments reduce code clarity and can hide
    /// bugs.
    ///
    /// **Known issues:** Only validates direct function and method calls.
    /// Does not yet handle closures, trait-object dispatch, or macro
    /// expansions.
    pub(crate) ARGUMENT_COMMENT_MISMATCH,
    Allow,
    "/*param*/ comment does not match the resolved parameter name",
}

declare_lint! {
    /// **What it does:** Flags literal-like arguments (`None`, `true`,
    /// `false`, integer/float literals) that are passed without a
    /// preceding `/*param*/` comment.
    ///
    /// **Why is this bad?** Bare literals at call sites can be ambiguous.
    /// A `/*param*/` comment makes the intent explicit, especially when
    /// a function has multiple parameters of the same type.
    ///
    /// **Exemptions:**
    ///   - String / character literals (`"foo"`, `'x'`)
    ///   - Self-documenting calls: when the argument expression is a single
    ///     identifier that matches the parameter name (e.g. `count: count`)
    ///
    /// **Known issues:** None.
    pub(crate) UNCOMMENTED_ANONYMOUS_LITERAL_ARGUMENT,
    Allow,
    "literal argument should have a /*param*/ comment",
}

declare_dylint_lint! {
    lints: [
        ARGUMENT_COMMENT_MISMATCH,
        UNCOMMENTED_ANONYMOUS_LITERAL_ARGUMENT,
    ],
}

// ============================================================================
// Lint pass
// ============================================================================

#[derive(Default)]
struct ArgumentCommentLint;

impl_late_pass!(ArgumentCommentLint);

impl LateLintPass<'_> for ArgumentCommentLint {
    fn check_expr(&mut self, cx: &LateContext<'_, '_>, expr: &Expr<'_>) {
        match &expr.kind {
            ExprKind::Call(callee, args) => {
                let param_names = resolve_param_names(cx, callee);
                check_call_site(cx, expr.span, &param_names, args);
            }
            ExprKind::MethodCall(path_segment, args, _span) => {
                // args[0] is self; params start at args[1..]
                if args.len() <= 1 {
                    return;
                }
                let param_names = resolve_method_param_names(cx, expr);
                // Skip `self`
                let call_args = &args[1..];
                check_call_site(cx, expr.span, &param_names, call_args);
            }
            _ => {}
        }
    }
}

// ============================================================================
// Parameter name resolution
// ============================================================================

fn resolve_param_names<'tcx>(
    cx: &LateContext<'_, 'tcx>,
    callee: &Expr<'_>,
) -> Vec<String> {
    let ty = cx.typeck_results().expr_ty_adjusted(callee);
    match ty.kind() {
        ty::FnDef(def_id, _) => {
            let sig = cx.tcx.fn_sig(*def_id).skip_binder();
            sig.inputs()
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    // Try to get the parameter name from the function definition
                    cx.tcx
                        .fn_arg_names(*def_id)
                        .get(i)
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| format!("_{}", i))
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn resolve_method_param_names<'tcx>(
    cx: &LateContext<'_, 'tcx>,
    expr: &Expr<'_>,
) -> Vec<String> {
    let ty = cx.typeck_results().expr_ty_adjusted(expr);
    match ty.kind() {
        ty::FnDef(def_id, _) => {
            let sig = cx.tcx.fn_sig(*def_id).skip_binder();
            // Method parameters include `self` — skip it for the caller
            sig.inputs()
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, _)| {
                    cx.tcx
                        .fn_arg_names(*def_id)
                        .get(i)
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| format!("_{}", i))
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

// ============================================================================
// Call-site linting logic
// ============================================================================

/// A parsed `/* param */` comment with its position in the source.
struct ArgComment {
    /// Parameter name extracted from the comment body.
    name: String,
    /// Byte offset of the comment's start within the call expression snippet.
    offset: usize,
    /// Byte offset of the comment's end (after `*/`).
    end: usize,
}

/// Parse `/*...*/` comments from a source snippet.
/// Returns comments in order of appearance.
fn parse_arg_comments(snippet: &str) -> Vec<ArgComment> {
    let mut comments = Vec::new();
    let mut pos = 0;
    while let Some(start) = snippet[pos..].find("/*") {
        let abs_start = pos + start;
        if let Some(end) = snippet[abs_start + 2..].find("*/") {
            let abs_end = abs_start + 2 + end + 2;
            let body = snippet[abs_start + 2..abs_end - 2].trim();
            if !body.is_empty() && !body.contains(' ') && !body.contains('\n') {
                comments.push(ArgComment {
                    name: body.to_string(),
                    offset: abs_start,
                    end: abs_end,
                });
            }
            pos = abs_end;
        } else {
            break;
        }
    }
    comments
}

/// Determine whether an argument expression is a "literal-like" anonymous
/// value that should have a `/*param*/` comment.
fn is_anonymous_literal(expr: &Expr<'_>) -> bool {
    match &expr.kind {
        // Explicitly anonymous: these are always flagged
        ExprKind::Lit(_) if !is_string_or_char_lit(expr) => true,
        // `None` paths
        ExprKind::Path(QPath::Resolved(_, path)) => {
            let segs: Vec<&str> = path.segments.iter().map(|s| s.ident.name.as_str()).collect();
            matches!(segs.as_slice(), ["None"] | ["std", "option", "Option", "None"])
        }
        // Boolean literals
        ExprKind::Lit(_) => true,
        _ => false,
    }
}

fn is_string_or_char_lit(expr: &Expr<'_>) -> bool {
    if let ExprKind::Lit(lit) = &expr.kind {
        matches!(
            lit.node,
            rustc_hir::LitKind::Str(..) | rustc_hir::LitKind::Char(..)
        )
    } else {
        false
    }
}

/// Check if the argument expression is "self-documenting" — a single
/// identifier whose name matches the expected parameter name.
fn is_self_documenting(expr: &Expr<'_>, param_name: &str) -> bool {
    if let ExprKind::Path(QPath::Resolved(_, path)) = &expr.kind {
        if path.segments.len() == 1 {
            let ident = path.segments[0].ident.name.as_str();
            return ident == param_name;
        }
    }
    false
}

/// Map argument positions to their byte ranges in the source snippet.
/// This is a best-effort parser that counts commas at the top level
/// (skipping nested brackets, parens, and strings).
fn argument_ranges(snippet: &str) -> Vec<(usize, usize)> {
    let paren_start = match snippet.find('(') {
        Some(p) => p + 1,
        None => return Vec::new(),
    };

    let mut ranges = Vec::new();
    let mut depth_paren = 0u32;
    let mut depth_angle = 0u32;
    let mut depth_brace = 0u32;
    let mut depth_bracket = 0u32;
    let mut in_string = false;
    let mut in_char = false;
    let mut arg_start = paren_start;
    let mut chars = snippet[paren_start..].char_indices().peekable();

    while let Some((rel_offset, ch)) = chars.next() {
        let abs_offset = paren_start + rel_offset;

        if in_string {
            if ch == '\\' {
                chars.next(); // skip escaped char
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if in_char {
            if ch == '\'' {
                in_char = false;
            }
            continue;
        }

        match ch {
            '(' => depth_paren += 1,
            ')' => {
                if depth_paren == 0 {
                    // End of argument list
                    let end = abs_offset;
                    if abs_offset > arg_start {
                        ranges.push((arg_start, end));
                    }
                    return ranges;
                }
                depth_paren -= 1;
            }
            '<' => depth_angle += 1,
            '>' => if depth_angle > 0 { depth_angle -= 1; }
            '{' => depth_brace += 1,
            '}' => if depth_brace > 0 { depth_brace -= 1; }
            '[' => depth_bracket += 1,
            ']' => if depth_bracket > 0 { depth_bracket -= 1; }
            '"' => in_string = true,
            '\'' => in_char = true,
            ',' if depth_paren == 0 && depth_angle == 0 && depth_brace == 0 && depth_bracket == 0 => {
                let end = abs_offset;
                if abs_offset > arg_start {
                    ranges.push((arg_start, end));
                }
                arg_start = abs_offset + 1;
            }
            _ => {}
        }
    }

    ranges
}

/// Extract the text of a single argument from the source snippet.
fn arg_text<'a>(snippet: &'a str, range: (usize, usize)) -> &'a str {
    snippet[range.0..range.1].trim()
}

/// Main call-site checker: validates both lint rules for a single call.
fn check_call_site(
    cx: &LateContext<'_, '_>,
    call_span: Span,
    param_names: &[String],
    args: &[Expr<'_>],
) {
    if param_names.is_empty() || args.is_empty() {
        return;
    }

    let Ok(snippet) = cx.sess().source_map().span_to_snippet(call_span) else {
        return;
    };

    let comments = parse_arg_comments(&snippet);
    let arg_ranges = argument_ranges(&snippet);

    for (i, arg) in args.iter().enumerate() {
        let param_name = match param_names.get(i) {
            Some(n) => n.as_str(),
            None => continue,
        };

        // Find the closest /* comment */ preceding this argument
        let arg_range = arg_ranges.get(i).copied().unwrap_or((0, 0));
        let preceding_comment = comments
            .iter()
            .filter(|c| c.end <= arg_range.0 || arg_range.0 == 0)
            .last();

        // --- Rule 1: ARGUMENT_COMMENT_MISMATCH ---
        if let Some(comment) = preceding_comment {
            if comment.name != *param_name {
                cx.lint(
                    ARGUMENT_COMMENT_MISMATCH,
                    arg.span,
                    format!(
                        "argument comment `/*{}*/` does not match parameter `{}`",
                        comment.name, param_name,
                    ),
                );
            }
        }

        // --- Rule 2: UNCOMMENTED_ANONYMOUS_LITERAL_ARGUMENT ---
        if preceding_comment.is_none() && !is_string_or_char_lit(arg) {
            if is_anonymous_literal(arg) && !is_self_documenting(arg, param_name) {
                cx.lint(
                    UNCOMMENTED_ANONYMOUS_LITERAL_ARGUMENT,
                    arg.span,
                    format!(
                        "literal argument at position {} should have a `/*{}*/` comment",
                        i + 1,
                        param_name,
                    ),
                );
            }
        }
    }
}

// ============================================================================
// Macro registration
// ============================================================================

dylint_lint!("/ => argument-comment-lint" => ARGUMENT_COMMENT_MISMATCH, UNCOMMENTED_ANONYMOUS_LITERAL_ARGUMENT);
