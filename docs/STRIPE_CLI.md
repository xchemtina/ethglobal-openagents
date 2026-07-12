# Stripe CLI + Projects — ChimiaClaw wiring

## Status (this machine)

| Piece | Status |
|-------|--------|
| Stripe CLI | **Installed** via Homebrew `stripe/stripe-cli/stripe` → `1.43.7` (`/opt/homebrew/bin/stripe`) |
| Agent skill | **Installed** `~/.agents/skills/stripe-projects` |
| Projects plugin | **Installing** — binary is ~131 MB from JFrog; network is slow. Manual URL below if CLI hangs. |
| Auth | Run **`stripe login`** once (browser) against account `acct_1PsRXKLWiL2XLIJa` |
| Payment Link (already live) | `https://buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00` |

## Why this matters for ChimiaClaw

| Use | Command / path |
|-----|----------------|
| Human checkout (already) | Gateway `STRIPE_PAYMENT_LINK_*` → Payment Links |
| Webhook fulfillment (next) | `stripe listen --forward-to localhost:4021/v1/webhooks/stripe` |
| Provision side services (DB, cache, host) | `stripe projects …` (Projects catalog) — **not** a substitute for our cashier |
| Avoid hand-copying secrets | `stripe projects env --pull` → map into gateway env (gitignored) |

**Projects ≠ hosting the x402 cashier.** Prefer Olympus + Cloudflare Tunnel for API. Projects is optional infra (Postgres, Redis, etc.) if we need it later.

## Setup

### 1. CLI (done on this Mac)

```bash
# already: brew install stripe/stripe-cli/stripe
stripe --version   # expect >= 1.40.0
```

### 2. Projects plugin

```bash
stripe plugin install projects
# if that hangs downloading ~131MB, use the known binary:
# https://stripe.jfrog.io/artifactory/stripe-cli-plugins-local/projects/0.23.0/darwin/arm64/stripe-cli-projects
# Then place per `stripe plugin install` layout under ~/.config/stripe/ (or re-run install when network is better).
```

### 3. Login (you — browser)

```bash
stripe login
# optional: stripe config --list
```

### 4. Init Projects in this repo (after login)

```bash
cd /path/to/OpenAgents
stripe projects init --preflight --json
stripe projects init --accept-tos --yes
# installs local skill at .claude/skills/stripe-projects-cli
stripe projects catalog --json | head
```

Do **not** commit `.projects/` or env dumps (see root `.gitignore`).

### 5. Dev webhooks → gateway (fulfillment path)

```bash
# terminal A: gateway with Stripe Payment Links configured
cd services/api-gateway && npm start

# terminal B: forward Stripe events (needs stripe login)
./scripts/stripe-webhook-dev.sh
# prints webhook signing secret whsec_… → set STRIPE_WEBHOOK_SECRET on gateway when handler lands
```

Endpoint target (to implement): `POST /v1/webhooks/stripe` → verify signature → fulfill SKU / write revenue event.

## Relation to existing env

```bash
# Already used (Payment Links — no secret key required for charge URL):
STRIPE_PAYMENT_LINK_MOLADT=https://buy.stripe.com/...
STRIPE_PAYMENT_LINK_DFT_CACHED=https://buy.stripe.com/...

# Optional later (dynamic Checkout Sessions + webhooks):
STRIPE_SECRET_KEY=sk_...          # Olympus only
STRIPE_WEBHOOK_SECRET=whsec_...   # from stripe listen or Dashboard
```

## Honest limits

- Homebrew was required for the official macOS CLI path (forgiven).  
- Projects plugin download is large; CLI itself is enough for `login` / `listen` / `trigger` today.  
- Live Payment Link charges already work without Projects.  
