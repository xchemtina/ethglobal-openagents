# Deploy DFT.xyz / ChimiaClaw cashier (pragmatic path)

Goal: something you can put on the public internet **without lying about money or science**.

## Architecture

```text
Browser / agents
    │
    ├─ web (Vercel)  ── NEXT_PUBLIC_API_BASE ──► api gateway (Fly/Railway/VPS)
    │                                              │
    │                                              ├─ stub x402 (agents)
    │                                              ├─ Stripe Checkout (humans, optional)
    │                                              └─ Revolut instructions (humans, optional)
    │
    └─ static orbital PNGs on Vercel (public/orbitals)
```

## 1. Frontend (Vercel) — ship this first

```bash
cd web
# set in Vercel project env:
# NEXT_PUBLIC_API_BASE=https://api.yourdomain.com
# NEXT_PUBLIC_X402_MODE=stub
# NEXT_PUBLIC_PAY_TO=0x... or revolut note for display
# NEXT_PUBLIC_WC_PROJECT_ID=... optional
vercel --prod
# or connect the monorepo root with Root Directory = web
```

- Builds offline with real orbital gallery assets under `public/orbitals/`.
- Without API, cashier shows errors but marketing + orbitals still work.

## 2. API gateway (Fly / Railway / small VPS)

```bash
cd services/api-gateway
# required
export X402_MODE=stub
export CHIMIACLAW_CLI=/path/to/chimiaclaw-cli   # or mount binary
export DFT_CACHE_DIR=/path/to/demo/dft
export PUBLIC_BASE_URL=https://api.yourdomain.com
export CORS_ORIGIN=https://dft.xyz,https://www.chimiadao.io

# human money (pick what you have)
export STRIPE_SECRET_KEY=sk_live_or_test_...
export STRIPE_PUBLISHABLE_KEY=pk_...
export REVOLUT_PAY_TO="your Revolut tag / IBAN label"
export REVOLUT_PAYMENT_LINK=https://revolut.me/...

npm install
npm start
```

Expose only HTTPS. Point `api.dft.xyz` or `api.chimiadao.io` DNS at the host.

## 3. Payment honesty matrix

| Rail | Who | Funds | Status env |
|------|-----|-------|------------|
| x402 stub | agents | none | default `X402_MODE=stub` |
| x402 live | agents | USDC | real pay_to + facilitator (later) |
| Stripe | humans | card | `STRIPE_SECRET_KEY` |
| Revolut | humans | manual | `REVOLUT_PAY_TO` / link |

**Never claim live USDC or Stripe charges without the matching env.**

## 4. What is safe to sell on day one

| SKU | Safe public? |
|-----|----------------|
| Catalog / health / openapi / dft index | yes free |
| `moladt.geometry` stub | yes demo |
| `dft.cached_result` stub | yes demo |
| Stripe checkout for micro SKUs | yes when key set |
| Revolut + operator fulfill | yes with manual process |
| Live SCF free-run | **no** |

## 5. Agent contract URLs

- `GET /health`
- `GET /v1/catalog`
- `GET /v1/payment-methods`
- `GET /openapi.json`
- `GET /.well-known/x402`
- `POST /v1/moladt` (402 / stub)
- `GET /v1/dft/cached?label=water` (402 / stub)
- `POST /v1/checkout` `{ "sku_id", "method": "stripe"|"revolut" }`

## 6. Remaining before “real business”

- Stripe webhook → automatic fulfillment token
- Revolut reconciliation (manual is OK at low volume)
- Hosted `chimiaclaw-cli` binary + health alerts
- Domain + TLS + backups of `revenue.jsonl`
- Optional: Modal for live DFT later
