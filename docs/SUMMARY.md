# ChimiaClaw / OpenAgents — Summary

**Date:** 2026-07-12  
**Repo root:** monorepo at OpenAgents (Rust core + HTTP cashier + Next.js market + workers + evidence)

This is the current honest snapshot of what exists, what ships, and what is still operator-gated.

---

## One sentence

ChimiaClaw turns scientific work into **signed artifacts**; the public surface is an **agents-first DFT cashier** (x402 for machines, Stripe/Revolut for humans) backed by real Olympus DFT evidence and a deployable Vercel site.

---

## What is real today

### Substrate (Rust)

- Artifact DAG with payload digests and parent lineage  
- MolADT geometry path (SMILES → signed `chem.molecule.adt`)  
- DFT request/result signing (`chem.dft.request` / `chem.dft.result`)  
- x402 catalog + settlement method types (`crates/chimiaclaw-x402`)  
- CLI: `x402-*`, `moladt-api`, live DFT execute, world-model verify  

### Cashier (`services/api-gateway`)

| Endpoint | Status |
|----------|--------|
| `GET /health` | live |
| `GET /v1/catalog` | live SKUs + honesty notes |
| `GET /v1/dft/index` | free discovery of cached labels |
| `GET /v1/dft/cached?label=` | paid (stub signature or human rail) |
| `POST /v1/moladt` | paid MolADT |
| `GET /v1/payment-methods` | x402 + Stripe + Revolut |
| `POST /v1/checkout` | Stripe Payment Link URL or Revolut instructions |
| `GET /openapi.json` | agent contract |
| live DFT execute | 501 / operator-gated until Modal + flag |

**Stripe:** Payment Link  
`https://buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00`  
wired as both MolADT and cached-DFT links until products are split.

### Web (`web/`)

- Agents-first cashier (SMILES + cache labels + curl recipes)  
- Interactive 3D orbital gallery from cube-sampled fields  
- Catalog, payment rails UI, gateway badge → live API endpoints  
- Deployed on Vercel; custom domain `dft.chimiadao.io` added (Porkbun A record pending)  

### Evidence (`demo/`)

- Gallery: water, methanol, benzene, propylene glycol, caprylic, capric (PBE/def2-tzvp + cubes)  
- Ge→Sn batch: 10 XYZs; multiple Sn SCFs converged on Olympus  
- Inventory docs for what already ran overnight  
- World-map / world-model projection of lab swarm  

### Elastic compute

- Modal DFT worker package: `skills/scienceclaw-port/workers/dft_modal`  
- Guards for atoms, wall time, spend; batch map for H100 swarms  
- Not yet linked to a live Modal account for production jobs  

### Tooling

- `tools/ge_sn_batch` — Ge→Sn rewrite + collect  
- `tools/chimerax_mcp` — ChimeraX MCP (preferred viz over Avogadro)  

---

## Brand / DNS (confirmed)

| Name | Role |
|------|------|
| **chimiadao.io** | Brand; DNS at **Porkbun** |
| **dft.chimiadao.io** | Science market UI → Vercel (`A dft → 76.76.21.21`) |
| **api.chimiadao.io** | Cashier → named Cloudflare Tunnel → Olympus `:4021` (pending) |
| **dft.xyz** | Already owned by third party — not our free option |

---

## What we deliberately do not claim

- Live USDC settlement without facilitator + funded `pay_to`  
- Automatic Stripe → artifact fulfillment (no webhook yet; operator fulfills)  
- Draw-to-structure as a product path  
- Live unbounded DFT on every paid click  
- That quick Cloudflare tunnels are production hostnames  

---

## Layout (proper folders)

```text
OpenAgents/
  crates/                 # Rust workspace
  services/api-gateway/   # HTTP x402 cashier
  web/                    # Next.js market UI (landed design)
  skills/.../workers/     # PySCF, Modal, literature
  demo/                   # signed evidence + batches + map
  tools/                  # Ge→Sn, ChimeraX MCP
  docs/                   # SUMMARY, NEXT_STEPS, DECISIONS, THOUGHTS, deploy
  scripts/                # deploy helpers
```

Legacy / reference trees may exist (`dft-xyz-landing-page/`, `world-map-demo/`); **product path is `web/` + `services/api-gateway` + `crates/`**.

---

## Validation snapshots

- Gateway: Stripe `configured`, checkout returns Payment Link  
- Web build: Next.js 16 production build on Vercel  
- DFT cache: 6 labels loaded from `demo/dft`  
- Sn batch: collect script non-blocking; RESULTS.md tracks convergence  

---

## Immediate next (see NEXT_STEPS)

1. Porkbun: `A dft → 76.76.21.21`  
2. Named Cloudflare Tunnel + `api.chimiadao.io`  
3. Point Vercel `NEXT_PUBLIC_API_BASE` at stable API  
4. Stripe webhook → fulfillment token  
5. Modal account link for elastic live DFT  
