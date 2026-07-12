# Arkhai vs Fly.io — what we use where

**Nothing was lost in the push/rebase.** Product path is still:

`crates/` · `services/api-gateway/` · `web/` · `demo/` · `docs/` · `tools/`  
HEAD: `origin/master` (see [`SUMMARY.md`](./SUMMARY.md)).

---

## Short answer

| Platform | What it is | Use for ChimiaClaw? |
|----------|------------|---------------------|
| **[Fly.io](https://fly.io)** | App hosting (containers, always-on HTTP) | Optional host for the **API gateway** only |
| **[Arkhai](https://www.arkhai.io)** | Agent-driven **compute market** (discover / negotiate / escrow / provision GPUs) | **Yes** — elastic **DFT workers**, not the cashier process |

**We do not replace Fly with Arkhai.** They solve different problems.  
**We prefer Arkhai (or Modal) for H100-scale jobs over Fly for science compute.**

---

## What Arkhai actually is

From [arkhai.io](https://www.arkhai.io) and [Simple Compute Market](https://github.com/arkhai-io/simple-compute-market):

- Open infrastructure for **machine-readable markets**: discovery, negotiation, escrow (Alkahest), settlement  
- **SCM** roles: buyer CLI, seller storefront, listings registry  
- Concrete domain today: **buy GPU/VM compute** (`market listing list --gpu-model H200`, `market buy …`)  
- Not a “deploy my Express app” PaaS  

Pilot / join: [arkhai.io/contact?form=joinThePilot](https://www.arkhai.io/contact?form=joinThePilot)

---

## Target architecture (honest)

```text
Agents / humans
      │
      ▼
  web/  (Vercel · dft.chimiadao.io)
      │
      ▼
  api-gateway  (Olympus + Cloudflare Tunnel · api.chimiadao.io)
      │  x402 · Stripe · Revolut
      │
      ├─ cached DFT  → demo/dft on disk (already live)
      ├─ live DFT    → CHIMIACLAW_DFT_COMMAND
      │                   ├─ Olympus local PySCF (proven)
      │                   ├─ Modal H100 worker (scaffolded)
      │                   └─ Arkhai SCM buyer → leased GPU VM + same worker (next)
      └─ trust always seals in Rust (chimiaclaw-cli)
```

Fly only appears if we ever want the **gateway itself** off Olympus. Prefer tunnel + Olympus for now.

---

## How we use Arkhai (planned integration)

### Role: **buyer of compute** for `dft.live_small` / batch Sn jobs

1. Join SCM pilot; install buyer CLI from [simple-compute-market](https://github.com/arkhai-io/simple-compute-market).  
2. Negotiate a GPU listing (e.g. H100/H200) with escrow.  
3. On provisioned host: install `chimiaclaw-dft` / `dft_modal` worker + CLI.  
4. Point gateway env:

```bash
export CHIMIACLAW_DFT_COMMAND="ssh arkhai-lease 'chimiaclaw-dft-modal --mode local'"
# or a small wrapper that runs on the leased VM and streams worker JSON back
```

5. Rust still signs `chem.dft.result`; Arkhai only supplies **capacity**.

### Role we are **not** taking first

- Running the **x402 cashier** as an Arkhai “listing” (wrong shape until we define a science-SKU schema).  
- Hosting Next.js on Arkhai (stays on Vercel).

### Optional later: **seller** of DFT service

List Olympus/Modal capacity on SCM with a Chimia-specific resource schema (`chem.dft.request` in, artifact hash out). That is a product decision after buyer path works.

---

## Fly.io status

- `services/api-gateway/fly.toml` remains a **optional** fallback.  
- Not required for current public demo (tunnel → local/Olympus `:4021`).  
- Docs should say **Olympus + Tunnel first**, Fly only if Olympus is offline and we need a cheap always-on Node host.

---

## Comparison for our SKUs

| Need | Best fit |
|------|----------|
| Public HTTP cashier | Olympus + Cloudflare Tunnel (now); Fly optional |
| Static site + orbitals | Vercel |
| Human card pay | Stripe Payment Link |
| Agent pay | x402 |
| Elastic DFT (H100) | **Arkhai SCM** and/or **Modal** |
| Proven small SCF | Olympus PySCF gallery |

---

## Immediate actions

1. Keep gateway on Olympus + tunnel (no Fly deploy required).  
2. Join Arkhai pilot if we want market-based GPUs.  
3. Keep Modal scaffold (`skills/.../dft_modal`) as parallel elastic path.  
4. Do **not** rewrite the gateway for Arkhai hosting APIs — there are none in the Fly sense.

See also: [`MODAL_DFT.md`](./MODAL_DFT.md), [`DEPLOY.md`](./DEPLOY.md), [`NEXT_STEPS.md`](./NEXT_STEPS.md).
