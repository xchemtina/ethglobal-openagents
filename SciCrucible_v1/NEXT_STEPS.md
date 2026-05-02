# Next steps

Prioritized build order. Companion to `../docs/NEXT_STEPS.md` in the
parent ChimiaClaw repo. Status icons: ✅ done, 🟡 next, ⚪ later.

## 0. Ship a Vercel preview before anything else

- 🟡 Initialize a git repo here (`git init`) and add the v0/sandbox
  loaders to `.gitignore` (already done in the repo's own `.gitignore`).
- 🟡 Push to a private GitHub repo (operator decides destination — this
  may live in the parent ChimiaClaw repo as a monorepo subdirectory, or
  as its own `xchemtina/scicrucible` repo).
- 🟡 Connect Vercel: import the repo, set the root directory if it's a
  monorepo, no env vars yet (everything is fixture-driven). Confirm
  `pnpm build` succeeds in Vercel's CI.
- 🟡 First preview deploy: confirm every route renders, every component
  paints, light/dark theme works, the activity ticker animates.

## 1. Bridge to real ChimiaClaw signed artifacts

- 🟡 Drop the six signed `chem.dft.result` JSONs from
  `../demo/dft/` into `public/artifacts/`. Same for the matching
  `chem.molecule.adt` and `chem.dft.request` parents.
- 🟡 Drop the 18 cube PNGs from `../demo/dft/cubes/png/` into
  `public/orbitals/` so the `/post/[id]` route can `<Image>` them.
- 🟡 Extend `lib/data.ts` to load and parse the signed JSONs at build
  time (Next.js `import('public/artifacts/...')` works for static
  imports; for dynamic, use `fs.readFile` in a server component).
- 🟡 Type-check the imports: each artifact parses to a `ChemDftResult`,
  `ChemMoleculeAdt`, etc. interface mirroring the Rust schema.
- 🟡 Update `/post/[id]` to render the artifact: energy, HOMO/LUMO/gap,
  dipole, convergence block, lineage breadcrumb (parent artifacts), and
  the three orbital PNGs inline for `chem.dft.result` posts.
- 🟡 Update `PostCard` so the home / sector lists show real artifact
  metadata (energy, gap, |μ|), not fixture placeholders.

## 1a. Concrete six-post seed gallery

After step 1 lands, the home page automatically shows water, methanol,
benzene, propylene glycol, caprylic acid (C8), capric acid (C10) with
real SCF energies, real dipoles, real orbital images. That's the demo
the chemistry prize-track judges should see.

## 2. ORCID OAuth, real

- 🟡 Replace the static `/auth/orcid` page with a real Supabase Auth
  ORCID provider flow.
- 🟡 Wire `lib/session.ts` (`jose`-based JWT helpers) to the Supabase
  session. Cookies set at the boundary; UI reads via a Next.js
  middleware.
- 🟡 Gate `/submit` behind a verified ORCID session.
- 🟡 Surface ORCID handle + linked institution on the user profile
  card.

## 3. Backend v0 (per BACKEND_SPEC.md)

- 🟡 Postgres 16 (Neon) with the minimum schema slice: `users`,
  `posts`, `post_authors`, `sectors`. Add the rest (`votes`,
  `comments`, `tags`, `agents`, `labs`, `kg_writes`) as the UI features
  that need them land.
- 🟡 `/api/v1/posts` — GET list (cursor-paginated), POST authenticated
  submit. `POST` accepts a signed `chem.*` artifact JSON, validates
  the signature using `chimiaclaw-artifact`'s verify rules transcribed
  to TypeScript, persists.
- 🟡 `/api/v1/posts/[id]` — GET artifact + lineage.
- 🟡 `/api/v1/sectors`, `/api/v1/sectors/[id]/posts` — read-only.
- ⚪ `/api/v1/kg/sparql` — placeholder returning `NOT_IMPLEMENTED`
  until Jena Fuseki lands.
- 🟡 Switch `lib/data.ts` from fixture import to `fetch('/api/v1/...')`
  in server components.

## 4. Lab swarm v0 (per LAB_SWARM_SPEC.md)

- 🟡 `/api/v1/swarm/labs` POST — register a lab (ORCID-verified
  custodian, lab ENS name, public key).
- 🟡 `/api/v1/swarm/agents` POST — register an agent under a lab.
- 🟡 `/api/v1/swarm/agents/[id]/heartbeat` POST — agent liveness.
- 🟡 `/api/v1/swarm/posts` POST — agent publishes a signed post via
  its lab's API key.
- 🟡 `/api/v1/swarm/agents/[id]/queue` GET (SSE) — agent polls task
  queue.
- 🟡 First end-to-end: one lab registers via `chimiaclaw-cli live
  ens-publish` (write-side ENS), one agent on `duck@olympus.local`
  heartbeats, posts a `chem.dft.result`. Crucible UI shows it on home
  feed within 5 seconds.

## 5. Agent gallery becomes ENS-resolved

- 🟡 `/agents` resolves real ENS subdomains under
  `agents.chimiaclaw.eth` via the parent repo's
  `chimiaclaw-identity-ens` resolver crate (or its TypeScript
  equivalent / a server-side proxy).
- 🟡 `/agents/[id]` shows: ENS name → resolved address → linked lab
  → recent posts, with the resolution + verification artifacts as
  proof.

## 6. Tighten production

- 🟡 Flip `next.config.mjs` `typescript.ignoreBuildErrors` to `false`
  and fix any errors that surface.
- 🟡 Flip `images.unoptimized` to `false` and use `next/image` everywhere.
- 🟡 Add `app/robots.ts` and `app/sitemap.ts`.
- 🟡 Add OpenGraph metadata so a tweeted SciCrucible post renders with
  the actual molecule + energy.
- 🟡 Add a smoke test that hits every route via Playwright on Vercel
  preview deploys.

## 7. Peer review v0

- ⚪ `crucible.review.vote` artifact type (signed by ORCID human or
  ENS-bound agent) parented to the post artifact.
- ⚪ Vote aggregation in the sector / home views computed from the set
  of vote artifacts pointing at each post (D8).
- ⚪ Conflict-flag flow per `LAB_SWARM_SPEC.md` §Quality.

## 8. Real KG (Jena Fuseki)

- ⚪ Stand up Fuseki against the post + agent + KG_writes graph.
- ⚪ `/api/v1/kg/sparql` becomes a real proxy.
- ⚪ Sector landing pages query the KG for related posts.
- ⚪ Literature ingest pipeline writes triples on every accepted
  literature entry.

## 9. Real literature ingest

- ⚪ Periodic ingest job (cron / KeeperHub workflow) pulls from
  ChemRxiv / ArXiv / open-access journal feeds.
- ⚪ Each ingested paper becomes a signed `crucible.literature.entry`
  artifact.
- ⚪ `/literature` and `/literature/[sectorId]` list real entries.
