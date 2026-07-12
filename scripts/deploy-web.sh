#!/usr/bin/env bash
# Deploy DFT.xyz web to Vercel from monorepo.
# Requires: vercel login (once) && network access
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/web"

if ! vercel whoami >/dev/null 2>&1; then
  echo "Not logged in to Vercel."
  echo "Run:  vercel login"
  echo "Then re-run:  ./scripts/deploy-web.sh"
  exit 1
fi

# Production API base — override when gateway is live
API_BASE="${NEXT_PUBLIC_API_BASE:-https://api.chimiadao.io}"
MODE="${NEXT_PUBLIC_X402_MODE:-stub}"

echo "Deploying web with:"
echo "  NEXT_PUBLIC_API_BASE=$API_BASE"
echo "  NEXT_PUBLIC_X402_MODE=$MODE"

# Link project in web/ if needed
if [[ ! -f .vercel/project.json ]]; then
  vercel link --yes --project dft-xyz || vercel link --yes
fi

vercel env add NEXT_PUBLIC_API_BASE production <<<"$API_BASE" 2>/dev/null || true
vercel env add NEXT_PUBLIC_X402_MODE production <<<"$MODE" 2>/dev/null || true

# Ship
vercel --prod --yes
echo "Done. Set REVOLUT/Stripe secrets only on the API host, not in the browser."
