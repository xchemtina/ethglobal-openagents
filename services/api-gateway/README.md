# ChimiaClaw API gateway (x402)

HTTP front door for agentic science payments. Sits in front of `chimiaclaw-cli` and sells **signed artifacts** for USDC via x402-shaped HTTP 402 challenges.

## Modes

| `X402_MODE` | Behavior |
|-------------|----------|
| `free` | No payment. Local UI / integration wiring. |
| `stub` | Returns HTTP 402; accept `PAYMENT-SIGNATURE: stub`. **No funds moved.** |
| `live` | Requires real `CHIMIA_X402_PAY_TO`. Facilitator verification upgrade path via `@x402/*`. |

Default is **`stub`** so you can demo the cashier loop without a funded wallet.

## Quick start

```bash
# 1. Build the science CLI
cargo build -p chimiaclaw-cli

# 2. Install gateway deps
cd services/api-gateway
npm install

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
