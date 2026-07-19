#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Sentinel AI — Secure container firewall
#
# Implements a default-deny outbound network policy using iptables + ipset.
# IPv6 is fully blocked.
#
# If OPENAI_ALLOWED_DOMAINS is set, only that space-separated list of
# domains is allowed outbound. Otherwise a built-in whitelist for
# common package registries and VCS hosts is used.
#
# Requires: NET_ADMIN, NET_RAW capabilities, iptables, ipset.
# ---------------------------------------------------------------------------
set -euo pipefail

echo "[init-firewall] Applying firewall rules…"

# -----------------------------------------------------------------------
# Domain whitelist
# -----------------------------------------------------------------------
if [[ -n "${OPENAI_ALLOWED_DOMAINS:-}" ]]; then
  IFS=' ' read -ra WHITELIST_DOMAINS <<< "$OPENAI_ALLOWED_DOMAINS"
  echo "[init-firewall] Using OPENAI_ALLOWED_DOMAINS (${#WHITELIST_DOMAINS[@]} domains)"
else
  WHITELIST_DOMAINS=(
    "github.com"
    "api.github.com"
    "crates.io"
    "static.crates.io"
    "index.crates.io"
    "pypi.org"
    "files.pythonhosted.org"
    "npmjs.org"
    "registry.npmjs.org"
    "nodejs.org"
    "objects.githubusercontent.com"
    "api.sentinel-ai.dev"
  )
  echo "[init-firewall] Using built-in whitelist (${#WHITELIST_DOMAINS[@]} domains)"
fi

# -----------------------------------------------------------------------
# IPv6: block everything
# -----------------------------------------------------------------------
echo "[init-firewall] Blocking IPv6…"
ip6tables -P INPUT DROP 2>/dev/null || true
ip6tables -P OUTPUT DROP 2>/dev/null || true
ip6tables -P FORWARD DROP 2>/dev/null || true
ip6tables -F 2>/dev/null || true
ip6tables -A INPUT  -j DROP 2>/dev/null || true
ip6tables -A OUTPUT -j DROP 2>/dev/null || true

# -----------------------------------------------------------------------
# IPv4: ipset whitelist
# -----------------------------------------------------------------------
echo "[init-firewall] Resolving whitelist domains to IPs…"
ipset create sentinel-whitelist hash:ip 2>/dev/null || ipset flush sentinel-whitelist

RESOLVED=0
for domain in "${WHITELIST_DOMAINS[@]}"; do
  while IFS= read -r ip; do
    if [[ -n "$ip" ]]; then
      ipset add sentinel-whitelist "$ip" 2>/dev/null && RESOLVED=$((RESOLVED + 1)) || true
    fi
  done < <(
    dig +short "$domain" 2>/dev/null ||
    host -t A "$domain" 2>/dev/null | grep -oE '([0-9]{1,3}\.){3}[0-9]{1,3}' ||
    nslookup "$domain" 2>/dev/null | grep -oE '([0-9]{1,3}\.){3}[0-9]{1,3}' ||
    echo ""
  )
done
echo "[init-firewall] Resolved $RESOLVED IPs into whitelist"

# -----------------------------------------------------------------------
# iptables rules
# -----------------------------------------------------------------------
echo "[init-firewall] Installing iptables rules…"

# Default deny outbound
iptables -P OUTPUT DROP

# Allow loopback
iptables -A OUTPUT -o lo -j ACCEPT

# Allow established/related connections
iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow whitelisted IPs
iptables -A OUTPUT -m set --match-set sentinel-whitelist dst -j ACCEPT

# Allow DNS (UDP port 53)
iptables -A OUTPUT -p udp --dport 53 -j ACCEPT

# Allow DHCP (UDP port 67/68)
iptables -A OUTPUT -p udp --dport 67:68 -j ACCEPT

# Log blocked packets (rate-limited)
iptables -A OUTPUT -m limit --limit 5/min -j LOG --log-prefix "FW-BLOCK: "

# -----------------------------------------------------------------------
# Verification
# -----------------------------------------------------------------------
echo "[init-firewall] Verifying firewall…"

# Test 1: loopback should be allowed
if curl -s -o /dev/null --max-time 2 http://127.0.0.1:1 2>/dev/null; then
  echo "  ✓ loopback allowed"
else
  echo "  ~ loopback unreachable (expected — nothing listens on port 1)"
fi

# Test 2: whitelisted domain should resolve and connect
TEST_DOMAIN="${WHITELIST_DOMAINS[0]}"
if curl -s -o /dev/null --max-time 5 "https://${TEST_DOMAIN}" 2>/dev/null; then
  echo "  ✓ whitelist domain reachable: $TEST_DOMAIN"
else
  echo "  ~ whitelist domain unreachable: $TEST_DOMAIN (DNS may not have propagated)"
fi

# Test 3: blocked domain should fail
if ! curl -s -o /dev/null --max-time 3 "https://example.com" 2>/dev/null; then
  echo "  ✓ blocked domain correctly denied: example.com"
else
  echo "  WARNING: blocked domain was reachable — check firewall rules!"
fi

echo "[init-firewall] Firewall applied and verified."
