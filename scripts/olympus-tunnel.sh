#!/usr/bin/env bash
# Start/restart Cloudflare *quick* tunnel on Olympus → local gateway :4021
# Usage (on Olympus or via ssh):
#   ./scripts/olympus-tunnel.sh
# Named tunnel (stable api.chimiadao.io) still needs: cloudflared tunnel login
# See docs/DNS_PORKBUN.md and scripts/named-tunnel-setup.sh

set -euo pipefail
export PATH="${HOME}/bin:/opt/homebrew/bin:${PATH}"

GW_DIR="${GW_DIR:-$HOME/ChimiaDAO/OpenAgents/services/api-gateway}"
LOG="${LOG:-/tmp/chimiaclaw-tunnel.log}"
URL_FILE="${URL_FILE:-$GW_DIR/.tunnel-url}"

if ! command -v cloudflared >/dev/null; then
  echo "cloudflared not found (expected ~/bin/cloudflared or brew)" >&2
  exit 1
fi

if ! curl -sf http://127.0.0.1:4021/health >/dev/null; then
  echo "gateway not healthy on :4021 — start api-gateway first" >&2
  exit 1
fi

for pid in $(pgrep -x cloudflared 2>/dev/null || true); do
  kill "$pid" 2>/dev/null || true
done
sleep 1

: >"$LOG"
nohup cloudflared tunnel --url http://127.0.0.1:4021 --no-autoupdate >"$LOG" 2>&1 &
echo "tunnel_pid=$!"

url=""
for _ in $(seq 1 25); do
  url=$(grep -oE 'https://[a-z0-9-]+\.trycloudflare\.com' "$LOG" 2>/dev/null | head -1 || true)
  if [[ -n "$url" ]]; then
    break
  fi
  sleep 1
done

if [[ -z "$url" ]]; then
  echo "failed to parse tunnel URL; see $LOG" >&2
  tail -30 "$LOG" >&2 || true
  exit 1
fi

echo "$url" | tee "$URL_FILE"
echo "health:"
curl -sS "$url/health" || true
echo
echo "Next: set Vercel NEXT_PUBLIC_API_BASE=$url and redeploy web"
