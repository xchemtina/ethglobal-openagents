## LIVE (this session — 2026-07-12 resume)

| Surface | URL |
|---------|-----|
| **Web (Vercel)** | https://web-five-rho-8v773a74lq.vercel.app |
| **API (Cloudflare quick tunnel → this Mac :4021)** | https://biggest-surf-majority-passport.trycloudflare.com |
| Gateway (local) | `127.0.0.1:4021` — Stripe Payment Link **configured** |
| Gateway (Olympus) | `duck@olympus.local:4021` — Stripe Payment Link **configured** |
| Stripe pay | https://buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00 |
| `cloudflared` | installed locally (Homebrew) + Olympus `~/bin/cloudflared` |

**Note:** Quick tunnel hostnames change when `cloudflared` restarts. Re-set Vercel `NEXT_PUBLIC_API_BASE` after each new hostname. For stable `api.chimiadao.io`, create a **named** Cloudflare Tunnel + Porkbun CNAME.

**DNS:** `chimiadao.io` is on **Porkbun** nameservers (do not move NS to Vercel). See [`docs/DNS_PORKBUN.md`](./DNS_PORKBUN.md).

| Host | Target | Status |
|------|--------|--------|
| `dft.chimiadao.io` | Vercel `web` — Porkbun **A** `dft` → `76.76.21.21` | domain on Vercel; SSL cert requested; **confirm A record in Porkbun** |
| `api.chimiadao.io` | Cloudflare Tunnel → Olympus `:4021` | cloudflared binary ready; named tunnel still pending login |
| `www` / apex | existing Vercel brand site | leave alone |

**Vercel env now:** `NEXT_PUBLIC_API_BASE=https://biggest-surf-majority-passport.trycloudflare.com` (prod + preview).

---

## B. API gateway (after web URL exists)

### Option B1 — any VPS / Mac mini / Olympus (fastest for you)

```bash
cd services/api-gateway
export X402_MODE=stub
export PUBLIC_BASE_URL=https://api.YOURDOMAIN.com
export CORS_ORIGIN=https://YOUR-VERCEL-URL.vercel.app,https://www.chimiadao.io
export DFT_CACHE_DIR=/absolute/path/to/OpenAgents/demo/dft
export CHIMIACLAW_CLI=/absolute/path/to/OpenAgents/target/debug/chimiaclaw-cli
export REVOLUT_PAY_TO="your real Revolut tag or handle"
export REVOLUT_PAYMENT_LINK="https://revolut.me/you"
# optional
# export STRIPE_SECRET_KEY=sk_test_...
# export STRIPE_PUBLISHABLE_KEY=pk_test_...

npm install
npm start
```

Put Caddy/nginx TLS in front, or Cloudflare Tunnel:

```bash
# example cloudflare tunnel to local 4021
cloudflared tunnel --url http://127.0.0.1:4021
```

Paste the tunnel URL into Vercel env `NEXT_PUBLIC_API_BASE`, redeploy web.

### Option B2 — Docker / Fly.io

```bash
cd services/api-gateway
# install flyctl, then:
fly launch --config fly.toml
fly secrets set X402_MODE=stub REVOLUT_PAY_TO=... PUBLIC_BASE_URL=https://chimiaclaw-api-gateway.fly.dev
fly deploy
```

Mount DFT cache volume or bake a subset of `demo/dft` into the image later.

---

## C. Wire money (your rails)

| Rail | Where | What to set |
|------|--------|-------------|
| **Revolut** (you) | gateway env | `REVOLUT_PAY_TO`, `REVOLUT_PAYMENT_LINK` |
| **Stripe** | gateway env | `STRIPE_PAYMENT_LINK_MOLADT` / `STRIPE_PAYMENT_LINK_DFT_CACHED` (= [buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00](https://buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00) for both until split) |
| **x402 agents** | gateway | keep `X402_MODE=stub` until live USDC |

Then:

```bash
curl -s https://API/v1/payment-methods | jq .
curl -s -X POST https://API/v1/checkout \
  -H 'content-type: application/json' \
  -d '{"sku_id":"dft.cached_result","method":"revolut"}'
```

Update Vercel:

```text
NEXT_PUBLIC_API_BASE=https://API
NEXT_PUBLIC_X402_MODE=stub
```

Redeploy web.

---

## D. Done when

- [ ] `https://*.vercel.app` loads orbitals  
- [ ] `GET https://API/health` → ok  
- [ ] `GET https://API/v1/payment-methods` shows revolut **configured**  
- [ ] Stub moladt/dft work from browser cashier  
- [ ] No claim of live USDC without facilitator  

---

## Right now on this machine

```bash
# 1) interactive
vercel login

# 2) deploy site
cd web && vercel --prod
```

Then reply with the Vercel URL + your preferred API host (tunnel vs VPS vs Fly) and we finish wiring.
