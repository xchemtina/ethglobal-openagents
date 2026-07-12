# DFT.xyz web (ChimiaDAO science market)

Next.js landing dropped from Vercel/v0 into this monorepo.

**Customers:** agents and other machines first. Humans get the same cashier for demos and ops.

**Brand note:** ChimiaDAO / ChimiaClaw product face. `DFT.xyz` = HTTP 402 science marketplace (SMILES/label → signed artifact → stub or USDC). Ship at `dft.xyz` or `chimiadao.io/agents`.

**Not building:** draw-to-structure / image OCR as a product path. Optional human UI may exist later; it is not on the critical path.

## Quick start

```bash
# Terminal A — cashier
cd services/api-gateway
export X402_MODE=stub CHIMIACLAW_CLI=../../target/debug/chimiaclaw-cli
export DFT_CACHE_DIR=../../demo/dft
npm run dev

# Terminal B — site
cd web
cp .env.example .env.local   # NEXT_PUBLIC_API_BASE=http://127.0.0.1:4021
pnpm install   # or npm install
pnpm dev       # http://localhost:3000
```

## Gateway contract

See [`README.contract.md`](README.contract.md) and [`docs/X402.md`](../docs/X402.md).

| UI need | API |
|---------|-----|
| Health badge | `GET /health` |
| SKU grid | `GET /v1/catalog` |
| Cached DFT labels | `GET /v1/dft/index` |
| MolADT try (stub pay) | `POST /v1/moladt` + `PAYMENT-SIGNATURE: stub` |

Wired today:

- Header **API badge** (`GatewayBadge`)
- **Service catalog** section (`ServiceCatalog`)
- **Molecule ticker** prefers live DFT index; falls back to real Olympus gallery + Ge + Sn evidence
- **Cashier** (`#cashier`): MolADT SMILES + cached DFT label buys (`PAYMENT-SIGNATURE: stub`)
- **Agent curl panel** with live `NEXT_PUBLIC_API_BASE` endpoints

Deferred / non-goals: draw-to-structure, image OCR. Still demo: challenge fixtures, live facilitator wallet settle.

## Source drop

Original export also kept at `../dft-xyz-landing-page/` if you need to re-diff the Vercel package.
