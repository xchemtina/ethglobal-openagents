# DNS — chimiadao.io on Porkbun

**Authority confirmed:** nameservers are Porkbun:

- `salvador.ns.porkbun.com`
- `curitiba.ns.porkbun.com`
- `fortaleza.ns.porkbun.com`
- `maceio.ns.porkbun.com`

**Do not** move nameservers to Vercel. Keep DNS at Porkbun and add records only.

Porkbun panel: **Domain Management → chimiadao.io → DNS**

---

## Records to add (copy into Porkbun)

### 1. Science market site (do this now)

Already added on Vercel project `web` as custom domain `dft.chimiadao.io`.

| Type | Host | Answer / Answer | TTL | Notes |
|------|------|-----------------|-----|--------|
| **A** | `dft` | `76.76.21.21` | 600 | Vercel anycast (recommended) |

Optional alias (either A above **or** this CNAME, not both):

| Type | Host | Answer | TTL |
|------|------|--------|-----|
| **CNAME** | `dft` | `cname.vercel-dns.com` | 600 |

After DNS propagates:

- https://dft.chimiadao.io → Vercel `web` deploy  
- Apex `chimiadao.io` / `www` stay as they are (already Vercel)

Verify:

```bash
dig +short A dft.chimiadao.io
# expect 76.76.21.21 (or Vercel edge IPs)
curl -sI https://dft.chimiadao.io | head -5
```

### 2. Agent API (after named Cloudflare Tunnel)

| Type | Host | Answer | TTL | Notes |
|------|------|--------|-----|--------|
| **CNAME** | `api` | `<TUNNEL_ID>.cfargotunnel.com` | 600 | From Cloudflare Zero Trust → Tunnels |

Until the named tunnel exists, API stays on an ephemeral quick-tunnel hostname and Vercel `NEXT_PUBLIC_API_BASE` must be updated when it changes.

Target:

- https://api.chimiadao.io/health  
- https://api.chimiadao.io/v1/catalog  

Gateway env once live:

```bash
export PUBLIC_BASE_URL=https://api.chimiadao.io
export CORS_ORIGIN=https://dft.chimiadao.io,https://www.chimiadao.io,https://web-five-rho-8v773a74lq.vercel.app
```

Vercel env:

```text
NEXT_PUBLIC_API_BASE=https://api.chimiadao.io
```

### 3. Optional: agents path on main site

If you prefer path over subdomain later:

- keep `www.chimiadao.io` as brand  
- either reverse-proxy `/agents` to the `web` project or merge routes into the main site repo  

Subdomain `dft.chimiadao.io` is the faster path and is already wired on Vercel.

---

## What already exists (leave alone)

| Name | Role |
|------|------|
| Apex / `www` | Main ChimiaDAO site (Vercel) |
| `chimiadao.xyz` | Alternate brand domain (Vercel) |

---

## Named tunnel checklist (api.chimiadao.io)

On a machine with Cloudflare login (you or Olympus once `cloudflared` is installed):

```bash
# one-time
cloudflared tunnel login
cloudflared tunnel create chimiaclaw-api
cloudflared tunnel route dns chimiaclaw-api api.chimiadao.io
# config.yml: hostname api.chimiadao.io → http://127.0.0.1:4021
cloudflared tunnel run chimiaclaw-api
```

If Cloudflare “route dns” cannot write to Porkbun automatically, paste the CNAME Cloudflare shows into Porkbun manually (Host `api` → `xxxx.cfargotunnel.com`).

Olympus currently has the gateway on `:4021` but **no** `cloudflared` binary yet — install there for production API.

---

## After `dft` record is live

1. Porkbun: add **A** `dft` → `76.76.21.21`  
2. Wait a few minutes  
3. Vercel domain should flip to **Valid**  
4. Set production CORS on gateway to include `https://dft.chimiadao.io`  
5. Continue Stripe Payment Links + named tunnel for `api`
