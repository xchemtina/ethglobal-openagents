#!/usr/bin/env bash
# Named Cloudflare Tunnel → api.chimiadao.io (stable hostname)
#
# Prerequisites (YOU once):
#   1. Cloudflare account (free)
#   2. Interactive login:  cloudflared tunnel login
#   3. Porkbun DNS for chimiadao.io (keep NS at Porkbun)
#
# Run on Olympus (preferred) after login:
#   ./scripts/named-tunnel-setup.sh
#
# Then add Porkbun CNAME if cloudflared route dns cannot write to Porkbun:
#   Host: api
#   Answer: <TUNNEL_ID>.cfargotunnel.com

set -euo pipefail
export PATH="${HOME}/bin:/opt/homebrew/bin:${PATH}"

TUNNEL_NAME="${TUNNEL_NAME:-chimiaclaw-api}"
HOSTNAME="${HOSTNAME:-api.chimiadao.io}"
LOCAL_URL="${LOCAL_URL:-http://127.0.0.1:4021}"
CONF_DIR="${HOME}/.cloudflared"
CONF_FILE="${CONF_DIR}/config.yml"

if ! command -v cloudflared >/dev/null; then
  echo "install cloudflared first" >&2
  exit 1
fi

if [[ ! -f "${CONF_DIR}/cert.pem" ]]; then
  echo "No Cloudflare cert. Run interactively on a machine with a browser:"
  echo "  cloudflared tunnel login"
  echo "Then copy ~/.cloudflared/cert.pem to Olympus and re-run this script."
  exit 2
fi

mkdir -p "$CONF_DIR"

# create tunnel if missing
if ! cloudflared tunnel list 2>/dev/null | grep -q "$TUNNEL_NAME"; then
  cloudflared tunnel create "$TUNNEL_NAME"
fi

TUNNEL_ID=$(cloudflared tunnel list | awk -v n="$TUNNEL_NAME" '$2==n {print $1; exit}')
if [[ -z "${TUNNEL_ID:-}" ]]; then
  echo "could not resolve tunnel id for $TUNNEL_NAME" >&2
  cloudflared tunnel list >&2 || true
  exit 1
fi

CRED="${CONF_DIR}/${TUNNEL_ID}.json"
if [[ ! -f "$CRED" ]]; then
  # credentials file usually created by tunnel create
  ls -la "$CONF_DIR" >&2
  echo "missing credentials $CRED" >&2
  exit 1
fi

cat >"$CONF_FILE" <<EOF
tunnel: ${TUNNEL_ID}
credentials-file: ${CRED}

ingress:
  - hostname: ${HOSTNAME}
    service: ${LOCAL_URL}
  - service: http_status:404
EOF

echo "Wrote $CONF_FILE"
echo "Tunnel id: $TUNNEL_ID"
echo "CNAME target: ${TUNNEL_ID}.cfargotunnel.com"

# Try Cloudflare DNS route (works only if domain is on Cloudflare DNS — we use Porkbun)
if cloudflared tunnel route dns "$TUNNEL_NAME" "$HOSTNAME" 2>&1; then
  echo "DNS route created via Cloudflare."
else
  echo "Could not auto-route DNS (expected if chimiadao.io stays on Porkbun)."
  echo "Add in Porkbun DNS:"
  echo "  Type: CNAME"
  echo "  Host: api"
  echo "  Answer: ${TUNNEL_ID}.cfargotunnel.com"
  echo "  TTL: 600"
fi

echo
echo "Run (foreground):"
echo "  cloudflared tunnel run $TUNNEL_NAME"
echo "Or background:"
echo "  nohup cloudflared tunnel run $TUNNEL_NAME > /tmp/chimiaclaw-named-tunnel.log 2>&1 &"
echo
echo "Then Vercel:"
echo "  NEXT_PUBLIC_API_BASE=https://${HOSTNAME}"
echo "Gateway:"
echo "  PUBLIC_BASE_URL=https://${HOSTNAME}"
echo "  CORS_ORIGIN=https://dft.chimiadao.io,https://www.chimiadao.io,https://web-five-rho-8v773a74lq.vercel.app"
