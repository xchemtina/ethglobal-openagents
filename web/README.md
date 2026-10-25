# ChimiaDAO science cashier web

Next.js frontend for the OpenAgents gateway: SMILES to MolADT geometry, cached DFT
results, artifact browsing, and operator-fulfilled human checkout. Live DFT and
verified USDC settlement are unavailable. Challenge and escrow interfaces are
explicit previews.

## Quick start

From the repository root, build the Rust CLI and start the gateway using its
[README](../services/api-gateway/README.md). In a separate terminal:

```sh
cd web
cp .env.example .env.local
npm ci
npm run dev
```

The default API is `http://127.0.0.1:4021`; the site runs at
`http://localhost:3000`. Set `NEXT_PUBLIC_API_BASE` before building to select a
configured gateway. No public deployment is implied by these local commands.

## Verification

Use Node 22.6 or later and the committed npm lockfile. Vercel explicitly runs `npm ci`; the historical pnpm lockfile is not the supported install path:


```sh
npm test
npm run typecheck
npm run build
```

The production build checks TypeScript. Checkout-link tests reject executable,
insecure, relative, malformed, and credential-bearing destinations without making
network calls. There is no configured ESLint task.

## Gateway integration

See the [frontend contract](README.contract.md) for endpoints and payment modes.
The API badge checks reachability; it does not validate scientific results or
payment settlement. Gallery visuals are projections of archived evidence.

Challenge previews remain in browser memory and are discarded on close. They do
not need a wallet, submit work, collect a bond, or schedule computation.

The original design export in `../dft-xyz-landing-page/` is a historical reference;
`web/` is the maintained frontend.
