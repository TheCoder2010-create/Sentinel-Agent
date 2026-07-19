#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Sentinel AI — Post-start script for secure dev container profile.
#
# Validates the OPENAI_ALLOWED_DOMAINS environment variable and applies
# the strict outbound firewall via init-firewall.sh.
#
# Designed to run as the postStartCommand in devcontainer.secure.json.
# Requires NET_ADMIN and NET_RAW capabilities.
# ---------------------------------------------------------------------------
set -euo pipefail

echo "[post-start] Starting secure container setup…"

# --- Validate OPENAI_ALLOWED_DOMAINS ---
if [[ -z "${OPENAI_ALLOWED_DOMAINS:-}" ]]; then
  echo "[post-start] WARNING: OPENAI_ALLOWED_DOMAINS is not set."
  echo "[post-start] Using built-in default whitelist (dev/pypi/npm/crates)."
  echo "[post-start] Set OPENAI_ALLOWED_DOMAINS to a space-separated list of"
  echo "[post-start] allowed domains to restrict outbound traffic further."
else
  echo "[post-start] OPENAI_ALLOWED_DOMAINS = $OPENAI_ALLOWED_DOMAINS"
fi

# --- Ensure bubblewrap is setuid (needed for inner sandbox) ---
if [[ -f /usr/bin/bwrap ]]; then
  if [[ ! -u /usr/bin/bwrap ]]; then
    echo "[post-start] Setting setuid bit on bubblewrap…"
    sudo chmod u+s /usr/bin/bwrap 2>/dev/null || true
  fi
  echo "[post-start] bubblewrap is $(ls -la /usr/bin/bwrap | awk '{print $1}')"
else
  echo "[post-start] WARNING: bubblewrap (bwrap) not found at /usr/bin/bwrap"
fi

# --- Install Codex CLI if not already present ---
if ! command -v sentinel &>/dev/null; then
  echo "[post-start] Installing Sentinel CLI from local build…"
  if [[ -f /workspaces/sentinel/target/release/sentinel ]]; then
    sudo cp /workspaces/sentinel/target/release/sentinel /usr/local/bin/sentinel
    echo "[post-start] Sentinel CLI installed from release build"
  elif [[ -f /workspaces/sentinel/target/debug/sentinel ]]; then
    sudo cp /workspaces/sentinel/target/debug/sentinel /usr/local/bin/sentinel
    echo "[post-start] Sentinel CLI installed from debug build"
  else
    echo "[post-start] WARNING: sentinel binary not found — build with 'cargo build --release' first"
  fi
else
  echo "[post-start] Sentinel CLI already installed: $(sentinel --version 2>/dev/null || echo 'version unknown')"
fi

# --- Apply firewall ---
echo "[post-start] Applying network firewall…"
bash .devcontainer/init-firewall.sh

# --- Verify firewall ---
echo "[post-start] Testing firewall…"
# Should succeed (loopback)
curl -s -o /dev/null -w "  loopback: %{http_code}\n" http://127.0.0.1:1 2>/dev/null || echo "  loopback: blocked (expected if nothing listening)"

echo "[post-start] Secure container setup complete."
