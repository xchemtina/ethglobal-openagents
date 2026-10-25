# Legacy gateway review branch

This branch imports gateway **source and example configuration only** from the modified-main development line. Runtime data, fulfillment/job ledgers, `.env` files, secrets, and scientific campaign outputs were excluded. The original modified checkout was not changed. This gateway remains separate from the later cashier/discovery branch.

## Fulfillment guarantees and limits

- Run **one gateway process per fulfillment ledger**. Operations serialize per canonical store path, reload strict ledger state, append and fsync before returning. This is not cross-process/database locking; do not share the file between replicas.
- A Stripe Checkout Session ID grants one token. Event retries and `checkout.session.async_payment_succeeded` for the same session return that token without replenishing credit or extending expiry. `payment_intent.succeeded` is ignored to prevent a second grant for one checkout.
- Automatic grants require a valid webhook HMAC, paid payment-mode Checkout Session, known single-result SKU, exact configured USD amount, and bounded target metadata. Missing fields, `no_payment_required`, unsupported packs/credits, and conflicting replay metadata fail closed. Operators retain explicit pack/credit minting using a required stable payment reference and validated quantities.
- Checkout validates target presence, cache availability, SKU tier, and consistent label/SMILES before returning payment instructions. Premium Skala never uses the classical Payment Link. Payment Links require operator-managed matching metadata/price; configurations that do not match the webhook contract require manual reconciliation.
- Concurrent redemptions cannot spend the same token/credit twice in one process. Target and SKU mismatches do not spend entitlement; cache existence/tier validation precedes redemption.
- **Redemption is at-most-once dispatch, not exactly-once delivery.** MolADT entitlement is consumed durably before the worker starts; cached/pack entitlement is consumed before response logging/delivery. Worker failure, log failure, or lost response may leave the token spent without delivery. An operator must inspect the ledger and execution outcome and explicitly reconcile/reissue where justified. There is no automatic refund or retry credit.
- Corrupt/truncated, conflicting, or symlink ledgers fail closed. Historical credit rows without `issued_redemptions` also fail closed: they require a separately reviewed migration using original payment and redemption evidence. No old ledger was imported, reset, repaired, or automatically reissued here. Back up and audit historical ledgers before any migration.
- Bearer tokens grant access; keep ledgers private and back them up. Refund/revocation workflows and multi-process persistence remain future work.

## Other repairs

Live x402 headers fail closed until verification and settlement exist; separately issued fulfillment tokens remain usable. SMILES lookup is exact and case-sensitive. Rate buckets and CLI time/output/concurrency are bounded; raw forwarded headers cannot spoof rate identity. Express async errors return structured responses. The container compiles TypeScript and runs installed JavaScript as a non-root user.

```sh
npm ci
npm run typecheck
npm test
npm run build
```

Tests use temporary ledgers, synthetic cache envelopes, fake workers, and local HTTP listeners. They do not contact Stripe or run scientific calculations. Synthetic envelopes test delivery plumbing, not scientific validity or signature verification. The gateway still trusts its operator-managed cache.

## Modes

| `X402_MODE` | Behavior |
|-------------|----------|
| `free` | No payment. Local UI / integration wiring. |
| `stub` | Returns HTTP 402; accept `PAYMENT-SIGNATURE: stub`. **No funds moved.** |
| `live` | Rejects unverifiable x402 headers with 503. Separately issued fulfillment tokens authorize their bound SKU. |

Default is **`stub`** so you can demo the cashier loop without a funded wallet.

## Quick start

```bash
# 1. Build the science CLI
cargo build -p chimiaclaw-cli

# 2. Install gateway deps
cd services/api-gateway
npm ci

# 3. Configure
cp .env.example .env
# set CHIMIACLAW_CLI to absolute or relative path of target/debug/chimiaclaw-cli

# 4. Run
export $(grep -v '^#' .env | xargs)   # or use your preferred env loader
npm run dev
```

## Endpoints

| Method | Path | Payment | Notes |
|--------|------|---------|--------|
| GET | `/health` | free | Liveness |
| GET | `/v1/catalog` | free | Service catalog for site + agents |
| GET | `/.well-known/x402` | free | Compact discovery document |
| POST | `/v1/moladt` | **paid** | Body: `{ "smiles": "O", "no_worker": true }` |
| GET | `/v1/dft/index` | free | Labels/SMILES available in the DFT cache |
| GET | `/v1/dft/cached` | **paid** | Query: `?label=water` or `?smiles=O` or `?id=art_…` |
| POST | `/v1/literature` | — | 501 coming soon |
| POST | `/v1/dft/live` | — | 501 coming soon (operator-gated) |

### Stub paid DFT cache

```bash
# Free index of sellable signed results
curl -s http://127.0.0.1:4021/v1/dft/index | jq '.items[].label'

# Expect 402
curl -s -D- 'http://127.0.0.1:4021/v1/dft/cached?label=water'

# Expect 200 + signed chem.dft.result
curl -s 'http://127.0.0.1:4021/v1/dft/cached?label=water' \
  -H 'PAYMENT-SIGNATURE: stub' | jq '.result_artifact_id,.summary'
```

### Stub paid call

```bash
# Expect 402
curl -s -D- -X POST http://127.0.0.1:4021/v1/moladt \
  -H 'content-type: application/json' \
  -d '{"smiles":"O","no_worker":true}'

# Expect 200 + signed artifact
curl -s -X POST http://127.0.0.1:4021/v1/moladt \
  -H 'content-type: application/json' \
  -H 'PAYMENT-SIGNATURE: stub' \
  -d '{"smiles":"O","no_worker":true}' | jq '.result_artifact_id'
```

### Smoke (gateway already running)

```bash
npm run smoke
```

## Revenue log

Each successful paid MolADT call appends one JSONL line to `REVENUE_LOG_PATH` (default `./data/revenue.jsonl`):

```json
{"at_unix":...,"sku_id":"moladt.geometry","mode":"stub","amount_usdc_micros":10000,...}
```

## Live mainnet checklist

1. Set `CHIMIA_X402_PAY_TO` to the DAO Base treasury.
2. Set `X402_NETWORK=eip155:8453` (Base mainnet).
3. Point `X402_FACILITATOR_URL` at a production facilitator (not `x402.org/facilitator`).
4. Install and wire official `@x402/express` middleware (optional deps listed in `package.json`).
5. Keep expensive DFT gated; start with MolADT + cached DFT only.
6. For live SCF: deploy Modal worker (`docs/MODAL_DFT.md`), never free-run uncapped GPU jobs.

## Architecture

```
agent/site → api-gateway (402) → chimiaclaw-cli moladt-api → signed chem.molecule.adt
                ↓
         revenue.jsonl + (later) market.x402.* artifacts
```

Signed artifacts remain the source of truth. This gateway is the cashier and HTTP projection.
