# Frontend and gateway contract

`web/` is the Next.js science cashier. It reads the gateway selected by
`NEXT_PUBLIC_API_BASE`; source code and a successful build do not establish that a
public deployment is online.

## Current routes

| UI surface | Gateway request |
|---|---|
| Reachability badge | `GET /health` |
| Service catalog | `GET /v1/catalog` |
| Payment configuration | `GET /v1/payment-methods` |
| Cached result index | `GET /v1/dft/index` |
| Molecular geometry | `POST /v1/moladt` with `{ "smiles": "O", "no_worker": true }` |
| Existing DFT result | `GET /v1/dft/cached?label=water` |
| Human checkout | `POST /v1/checkout` with `sku_id` and `method` |
| Agent contract | `GET /openapi.json`, `GET /.well-known/x402` |

## Payment and execution boundaries

- In `stub` mode, artifact requests without `PAYMENT-SIGNATURE: stub` receive
  HTTP 402. This header exercises a demo flow and does not prove payment.
- In `free` mode, the gateway allows requests without payment.
- In `live` mode, artifact requests fail closed with HTTP 503 until verified
  payment and settlement are implemented. Setting an environment variable or
  connecting a wallet cannot enable settlement.
- Live DFT execution remains unavailable (HTTP 501). A cached result request
  retrieves existing evidence; it does not schedule a new calculation.
- Stripe/Revolut configuration is reported by the gateway. Human payments require
  operator fulfillment. The frontend offers an explicit HTTPS link after a
  successful checkout response and does not claim a payment has completed.
- Payment links are public configuration; private keys and provider secrets stay
  outside the frontend.

## Demonstration content

Challenge cards are fixed sample fixtures. The challenge dialog operates only in
local React state: no submission, payment, worker dispatch, or notification occurs.
Settlement diagrams illustrate proposed adapters. Archived gallery projections
must remain distinct from new computations and cryptographic verification.

## Local configuration and checks

See `.env.example` and [`README.md`](README.md). Use Node 22.6 or later.

```sh
npm ci
npm test
npm run typecheck
npm run build
```

Production builds enforce TypeScript checks. Fonts use the existing CSS system
font stack, so builds do not fetch Google Fonts. `npm test` checks navigation URL
safety; it does not contact a payment provider.
