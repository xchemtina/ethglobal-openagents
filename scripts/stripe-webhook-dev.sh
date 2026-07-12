#!/usr/bin/env bash
# Forward Stripe events to local api-gateway for fulfillment development.
# Requires: stripe CLI logged in (`stripe login`)
#
# Usage:
#   ./scripts/stripe-webhook-dev.sh
#   GATEWAY_WEBHOOK_URL=http://127.0.0.1:4021/v1/webhooks/stripe ./scripts/stripe-webhook-dev.sh

set -euo pipefail
export PATH="/opt/homebrew/bin:${PATH}"

URL="${GATEWAY_WEBHOOK_URL:-http://127.0.0.1:4021/v1/webhooks/stripe}"

if ! command -v stripe >/dev/null; then
  echo "stripe CLI not found. Install: brew install stripe/stripe-cli/stripe" >&2
  exit 1
fi

echo "Forwarding Stripe webhooks → $URL"
echo "Copy the whsec_… secret into gateway STRIPE_WEBHOOK_SECRET when the handler exists."
echo "Events of interest: checkout.session.completed, payment_intent.succeeded, checkout.session.async_payment_succeeded"
echo

exec stripe listen \
  --forward-to "$URL" \
  --events checkout.session.completed,payment_intent.succeeded,checkout.session.async_payment_succeeded
