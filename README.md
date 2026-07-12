# ChimiaClaw / OpenAgents

**Rust-native signed artifact DAG for autonomous scientific agents**  
ChimiaDAO substrate · agentic DFT cashier · EthGlobal OpenAgents lineage

Showcase (EthGlobal): https://ethglobal.com/showcase/sciclaw-x-dao-igcz8

Every scientific action, agent decision, payment, and governance event becomes an **immutable signed artifact** in a verifiable DAG.

---

## What this repo is

| Layer | Path | Role |
|-------|------|------|
| Artifact core | `crates/*` | Signed DAG, MolADT, market, DFT, x402 types, CLI |
| HTTP cashier | `services/api-gateway` | Catalog, HTTP 402, MolADT + cached DFT SKUs, Stripe/Revolut |
| Science market UI | `web/` | Agents-first Next.js site (DFT.xyz / ChimiaDAO) |
| Workers | `skills/scienceclaw-port/workers/*` | PySCF DFT, Modal H100 scaffold, literature |
| Evidence | `demo/` | Signed DFT gallery, Ge→Sn batch, world-model map |
| Tools | `tools/` | Ge→Sn batch, ChimeraX MCP |
| Operator docs | `docs/` | Deploy, payments, DNS, architecture, living notes |

Canonical living notes:

- [`docs/SUMMARY.md`](docs/SUMMARY.md) — what is real today  
- [`docs/NEXT_STEPS.md`](docs/NEXT_STEPS.md) — ordered build queue  
- [`docs/DECISIONS.md`](docs/DECISIONS.md) — stable architectural choices  
- [`docs/THOUGHTS.md`](docs/THOUGHTS.md) — working notes  

Deploy / money / DNS:

- [`docs/DEPLOY.md`](docs/DEPLOY.md) · [`docs/DEPLOY_NOW.md`](docs/DEPLOY_NOW.md)  
- [`docs/PAYMENTS.md`](docs/PAYMENTS.md) · [`docs/DNS_PORKBUN.md`](docs/DNS_PORKBUN.md)  
- [`docs/X402.md`](docs/X402.md) · [`docs/MODAL_DFT.md`](docs/MODAL_DFT.md)  

---

## Quick start

### Rust core

```bash
cargo build -p chimiaclaw-cli
cargo run -p chimiaclaw-cli -- x402-catalog
cargo run -p chimiaclaw-cli -- moladt-api --smiles c1ccccc1 --no-worker
```

### API gateway (cashier)

```bash
cd services/api-gateway
cp .env.example .env   # set STRIPE_PAYMENT_LINK_* , DFT_CACHE_DIR, etc.
npm install
npm start              # http://127.0.0.1:4021
```

```bash
curl -s http://127.0.0.1:4021/health
curl -s http://127.0.0.1:4021/v1/catalog | jq '.skus[] | {sku_id, price_display, status}'
curl -s 'http://127.0.0.1:4021/v1/dft/cached?label=water' \
  -H 'PAYMENT-SIGNATURE: stub'
```

### Web (agents-first science market)

```bash
cd web
cp .env.example .env.local   # NEXT_PUBLIC_API_BASE=http://127.0.0.1:4021
npm install
npm run dev                  # http://127.0.0.1:3000
```

Production: Vercel project `web` · custom host `dft.chimiadao.io` (Porkbun DNS).

---

## Product surface (honest)

| Who | How they pay | What they get |
|-----|--------------|---------------|
| **Agents** | x402 stub / live HTTP 402 | Signed `chem.molecule.adt` / `chem.dft.result` |
| **Humans** | Stripe Payment Link, Revolut | Same SKUs after operator fulfillment |
| **Operators** | Olympus / Modal | Live DFT (gated), cube gallery |

**Not a product:** draw→structure UI. Machines POST SMILES or cache labels.

Live SKUs (gateway):

1. `moladt.geometry` — SMILES → signed MolADT  
2. `dft.cached_result` — label → signed gallery DFT (water…capric acid)  
3. `dft.live_small` — catalog only / 501 until Modal + operator flag  

---

## Evidence already real

- Six-molecule PBE/def2-tzvp gallery on Olympus with HOMO/LUMO/density cubes (`demo/dft/`)  
- Interactive 3D orbital fields on the site (`web/public/orbitals/`)  
- Ge→Sn batch geometries + SCF results (`demo/ge-sn-batch/`)  
- Olympus inventory: `demo/olympus-dft-inventory.{json,md}`  
- World-model / lab-swarm projection: `demo/world-map.html`  

---

## Architecture (commerce path)

```text
Agents / humans
      │
      ▼
  web/  (Vercel · dft.chimiadao.io)
      │  NEXT_PUBLIC_API_BASE
      ▼
  services/api-gateway  (Olympus · api.chimiadao.io via Cloudflare Tunnel)
      │  x402 · Stripe Payment Links · Revolut
      ▼
  chimiaclaw-cli + PySCF / Modal workers
      │
      ▼
  signed chem.* artifacts  (trust boundary)
```

Workers never mint trust. The Rust CLI signs `chem.dft.request` / `chem.dft.result`.

---

## Repo hygiene

| Keep out of git | Why |
|-----------------|-----|
| `services/api-gateway/.env`, `web/.env*` | secrets / local API base |
| `**/node_modules`, `**/.next`, `/target` | build artifacts |
| Stripe secret keys | Olympus/operator only |

Payment **Links** (`buy.stripe.com/...`) are public URLs; secret keys are not.

---

## License

Apache-2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).  
Contact: info@chimiadao.io
