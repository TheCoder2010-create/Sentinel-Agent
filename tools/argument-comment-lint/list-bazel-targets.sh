#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Generate a list of Bazel Rust targets suitable for argument-comment linting.
#
# Outputs one label per line, filtering out:
#   - External repositories (@...)
#   - Targets with manual tags
#   - Targets opting out via "no_arg_lint" tag
#
# Usage:
#   ./tools/argument-comment-lint/list-bazel-targets.sh > targets.txt
#   bazel build $(cat targets.txt) \
#       --aspects //tools/argument-comment-lint:lint_aspect.bzl%argument_comment_lint_aspect
# ---------------------------------------------------------------------------
set -euo pipefail

BAZEL="${BAZEL:-bazel}"

echo "[list-bazel-targets] Querying Bazel for Rust targets…" >&2

$BAZEL query '
  kind("rust_(library|binary|test)", //...)
  except
  kind("rust_(library|binary|test)", @...)
' --output=label 2>/dev/null | while IFS= read -r label; do
  # Skip targets with "no_arg_lint" tag
  tags=$($BAZEL query "attr('tags', 'no_arg_lint', $label)" --output=label 2>/dev/null || true)
  if [[ -n "$tags" ]]; then
    echo "[list-bazel-targets] Skipping $label (tagged no_arg_lint)" >&2
    continue
  fi

  # Skip targets with "manual" tag
  tags=$($BAZEL query "attr('tags', 'manual', $label)" --output=label 2>/dev/null || true)
  if [[ -n "$tags" ]]; then
    echo "[list-bazel-targets] Skipping $label (tagged manual)" >&2
    continue
  fi

  echo "$label"
done

echo "[list-bazel-targets] Done — listed above" >&2
