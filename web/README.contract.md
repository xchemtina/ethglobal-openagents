# ChimiaDAO public website (frontend)

**Canonical brand site:** [https://www.chimiadao.io](https://www.chimiadao.io)  
Next.js on Vercel (`v0.app` stack). That is the human front door for the DAO — not a second OpenAgents-branded site.

## Recommended integration model

Do **not** launch a separate public product domain for ChimiaClaw/OpenAgents. Fold the agentic market into the existing ChimiaDAO brand:

| Surface | URL | Role |
|---------|-----|------|
| DAO home / narrative | `https://www.chimiadao.io` | Existing site (mission, story, trust) |
| Science market (humans) | `https://www.chimiadao.io/agents` (or `/market`) | Catalog, try MolADT, proof gallery, “for agents” |
| Cashier API (agents) | `https://api.chimiadao.io` (or Vercel rewrite to gateway) | x402 + signed artifacts |

**Why this shape**

- One brand, one SEO surface, one design system (cyan/purple/pink HUD already matches SciCrucible / world-map aesthetics).
- Marketing site stays light; heavy science/payment stays on the gateway.
- You can ship `/agents` in **stub / free** mode without a treasury address; only `api` needs `pay_to` for live USDC later.

## How to land the current Vercel site in this monorepo (optional)

If you want the site source next to OpenAgents:

1. Export / clone the chimiadao.io Vercel project.
2. Drop it into this `web/` folder **or** keep it in its own repo and only copy the integration contract below.
3. Add route(s) `/agents` (catalog + demo) wired to the gateway.
4. Point env vars at the API gateway (see `.env.example`).
5. Keep `www` on Vercel; host `services/api-gateway` on Railway / Fly / a small VPS, then CNAME `api.chimiadao.io`.

If the site stays in a separate repo, still use the same paths and env names so agents and humans share one discovery story.

## Integration contract (do not break)

### Free routes (site can call anytime)

| Site need | Gateway |
|-----------|---------|
| Service catalog / pricing table | `GET {NEXT_PUBLIC_API_BASE}/v1/catalog` |
| x402 discovery | `GET {NEXT_PUBLIC_API_BASE}/.well-known/x402` |
| Health | `GET {NEXT_PUBLIC_API_BASE}/health` |
| DFT cache index (free) | `GET {NEXT_PUBLIC_API_BASE}/v1/dft/index` |

### Paid routes (agentic cashier)

| Site need | Gateway |
|-----------|---------|
| MolADT geometry | `POST {NEXT_PUBLIC_API_BASE}/v1/moladt` body `{ "smiles": "c1ccccc1" }` |
| Cached DFT result | `GET {NEXT_PUBLIC_API_BASE}/v1/dft/cached?label=water` |

- Without payment → **HTTP 402** + `PAYMENT-REQUIRED` header / body `accepts[]`.
- Stub demo → retry with header `PAYMENT-SIGNATURE: stub`.
- Live → real x402 client / wallet flow.

### Design surfaces to include (minimum)

1. **Hero** — ChimiaClaw thesis: signed science → agentic market → DAO treasury.
2. **Service catalog** — render `/v1/catalog` SKUs with price + status.
3. **Try MolADT** — SMILES input; show 402 then paid result / artifact id.
4. **Proof gallery** — link or embed existing DFT / world-map assets from `demo/` or static copies.
5. **For agents** — curl examples + link to `/.well-known/x402`.
6. **DAO strip** — `pay_to` address, network, “no silent fund movement” honesty.

### Env vars

See `.env.example`:

- `NEXT_PUBLIC_API_BASE` — e.g. `http://127.0.0.1:4021` or production gateway URL
- `NEXT_PUBLIC_X402_MODE` — informational badge (`stub` / `live`)
- `NEXT_PUBLIC_PAY_TO` — display treasury address (should match gateway)

## Placeholder shell

Until the Vercel design lands, a minimal Next placeholder can be scaffolded with:

```bash
cd web
npx create-next-app@14 . --typescript --eslint --app --src-dir=false --import-alias "@/*"
# then wire fetch to NEXT_PUBLIC_API_BASE
```

Or simply open the gateway catalog in the browser while you transfer the design:

```text
http://127.0.0.1:4021/v1/catalog
```

## What stays out of the website

- Private keys, facilitator secrets, worker credentials
- Live DFT free-run (must stay gated)
- Inventing science state not backed by signed artifacts

The dashboard doctrine still holds: **the site is a projection; the artifact DAG is truth.**
