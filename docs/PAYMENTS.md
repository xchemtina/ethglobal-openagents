# Payments — Stripe, Revolut, x402, and “not only Base”

## Can we use something other than Base?

**Yes.**

| Rail | Network / system | Who uses it |
|------|------------------|-------------|
| **x402 stub** | none (demo) | agents now |
| **x402 live** | any chain the facilitator supports (Base is common, **not required by our code**) | agents later |
| **Stripe** | card / Apple Pay / etc. (fiat) | humans |
| **Revolut** | your personal Revolut (fiat, manual) | humans |
| **Rippling** | HR/payroll platform — **not** a customer checkout rail | **not recommended** for end-user molecule payments |

Our gateway already models human rails as **fiat:stripe** / **fiat:revolut**. Base only appears when you set `X402_NETWORK=eip155:8453` for crypto agents.

Recommended for you right now:

1. **Agents:** keep `X402_MODE=stub` (or free).  
2. **Humans:** **Revolut payment link** (you said you’ll provide) and/or **Stripe test → live**.  
3. Skip Rippling for customer payments (wrong product).

---

## Stripe Payment Links (recommended for you)

You already opened the right page:

**[Payment links → Create](https://dashboard.stripe.com/acct_1PsRXKLWiL2XLIJa/payment-links/create)**

1. Toggle **Test mode** (top) until you’re ready for real charges.  
2. Create **two** products/links (or one each):  
   - MolADT geometry — e.g. **$0.50+** (Stripe card minimum is often ~$0.50; our catalog $0.01 is agent-stub pricing)  
   - Cached DFT result — e.g. **$0.50–$1.00**  
3. After save, copy the **Payment link** URL (`https://buy.stripe.com/...`).  
4. On Olympus gateway:

```bash
# Live link (acct_1PsRXKLWiL2XLIJa) — one link for both SKUs until you split products
export STRIPE_PAYMENT_LINK_MOLADT=https://buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00
export STRIPE_PAYMENT_LINK_DFT_CACHED=https://buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00
# restart gateway
```

Then:

```bash
curl -s -X POST https://YOUR_API/v1/checkout \
  -H 'content-type: application/json' \
  -d '{"sku_id":"dft.cached_result","method":"stripe"}'
# → { "url": "https://buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00" }
```

Direct human pay: [buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00](https://buy.stripe.com/5kQ28sahM1zR3mO3gB5Rm00)

## Stripe API keys (only if you want dynamic Checkout Sessions)

1. [Dashboard](https://dashboard.stripe.com) → **Developers → API keys**  
2. **Publishable key** → optional `STRIPE_PUBLISHABLE_KEY`  
3. **Secret key** → `STRIPE_SECRET_KEY` **only on Olympus** (never Vercel public env)

---

## Revolut payment link

When you have it:

```bash
export REVOLUT_PAYMENT_LINK=https://revolut.me/yourhandle
export REVOLUT_PAY_TO="your display name / tag"
```

`POST /v1/checkout` with `"method":"revolut"` already returns instructions + reference string.

---

## Domain: buy dft.xyz vs subdomain?

| Name | Status |
|------|--------|
| **dft.xyz** | **Already registered** (since 2014, expires 2027) — **not** free to register on Porkbun unless you buy from current owner |
| **dft.chimiadao.io** | Subdomain of **your** domain — free; **DNS is at Porkbun** |
| **api.chimiadao.io** | Same — best for the Olympus named Cloudflare Tunnel |

**Confirmed:** `chimiadao.io` nameservers are Porkbun (`*.ns.porkbun.com`). Keep them; add records only. Full recipe: [`docs/DNS_PORKBUN.md`](./DNS_PORKBUN.md).

Use:

- site: `dft.chimiadao.io` → Vercel project `web` (domain already added; needs Porkbun **A** `dft` → `76.76.21.21`)  
- api: `api.chimiadao.io` → named Cloudflare Tunnel to Olympus (`CNAME` `api` → `*.cfargotunnel.com`)
