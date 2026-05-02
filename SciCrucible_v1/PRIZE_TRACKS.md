# Prize tracks

Per-sponsor positioning and distance-to-live. Combines the parent
ChimiaClaw repo's adapter surfaces (Rust side) with SciCrucible's UI
surfaces (this repo). Distance is measured in **work units to a
demoable signed artifact**: 0 = already there, 1 = an afternoon, 2 = a
day, 3 = multi-day.

## ENS — Ethereum Name Service

**Pitch.** Every agent and every lab has a verifiable ENS name; every
publication carries the agent's ENS handle in the signed artifact;
re-resolving the name verifies the agent still controls it. ChimiaClaw
already implements both the read side (resolver) and the write side
(publisher) as feature-gated `live ens-verify` / `live ens-publish` CLI
flows that produce three signed artifacts (publication → resolution →
verification).

**Distance to live demo.**

| Component | State | Notes |
| --------- | ----- | ----- |
| Read-side resolver crate (`chimiaclaw-identity-ens`) | ✅ done | Signs `identity.ens.resolution` + `identity.ens.verification`. |
| Write-side uv worker (`web3.py + ens.set_text`) | ✅ done | Refuses mainnet without `--allow-mainnet`, refuses non-owner accounts, never accepts the key on argv. |
| `live ens-publish` CLI subcommand | ✅ done | Three-artifact round-trip in one command. |
| **Live Sepolia smoke** (real testnet ENS name + funded controller key) | 🟡 distance 1 | Needs operator-supplied Sepolia ENS name + Sepolia ETH from a faucet. `demo/ens-roundtrip.sh` is the runbook. |
| SciCrucible `/agents/[id]` resolves real ENS | 🟡 distance 2 | After the Sepolia smoke, the dashboard reads the live resolver and renders the result with the verification badge. |

**Single biggest blocker.** Operator getting a Sepolia ENS name +
funded controller key. (See parent repo's previous discussion of
faucets.)

## 0G — 0G Storage

**Pitch.** Big scientific outputs (orbital density cubes, KG snapshots,
literature PDFs, MD trajectories) are content-addressed externally and
anchored on 0G Galileo. The signed artifact commits to the SHA-256;
the bytes live on 0G. ChimiaClaw already has a stub uploader that
hashes via Blake2b-32 and emits a deterministic stub receipt for
CI/demos, plus the full real-mode wrapper around `0g-storage-client`.

**Distance to live demo.**

| Component | State | Notes |
| --------- | ----- | ----- |
| `chimiaclaw-storage-0g` Rust crate | ✅ done | `live zerog-anchor` + signed `storage.zerog.upload` artifact. |
| Stub mode (`ZEROG_STUB=1`) | ✅ done | Blake2b-32 deterministic receipt with explicit `STUB MODE` audit notes. |
| End-to-end stub run | ✅ done | `art_62a1177fa495209f` parented to a real ferrocene MolADT artifact. |
| `0g-storage-client` binary installed locally | 🟡 distance 1 | Pre-built Go binary on 0G's GitHub releases. |
| Galileo turbo testnet 0G tokens | 🟡 distance 1 | Faucet on the 0G testnet site. |
| **Live anchor**: upload a real chem.dft.result + cube payload, sign on real Galileo | 🟡 distance 1 (after the two prereqs) | The Rust adapter is already wired; flipping `ZEROG_STUB` off is enough. |
| SciCrucible `/post/[id]` shows 0G CID + verifies SHA-256 | 🟡 distance 2 | After the live anchor, the dashboard surfaces the 0G storage URI + a "verify" link. |

**Most credible target payload.** The 6-molecule cube gallery (~28 MB
of `.cube` files, ~1.8 MB of PNGs). Anchoring those is the most
chemistry-credible storage demo we have.

## KeeperHub

**Pitch.** Long-running scientific jobs (DFT calculations, literature
ingest, periodic peer-review aggregation) are scheduled through
KeeperHub workflows. Every schedule and every status check is a signed
artifact; the workflow execution chains DFT request → KeeperHub
schedule → 0G anchor cleanly.

**Distance to live demo.**

| Component | State | Notes |
| --------- | ----- | ----- |
| `chimiaclaw-exec-keeperhub` Rust crate | ✅ done | REST client; `live keeperhub-schedule` + `live keeperhub-status` produce signed artifacts. |
| Reference workflow JSON | ✅ done | `demo/keeperhub/workflow.json` — manual-trigger workflow with `artifact_id`, `payload_hash`, `mode` inputs. |
| Operator runbook | ✅ done | `demo/keeperhub/README.md` — full DFT-request → KeeperHub-schedule → 0G-anchor chain. |
| KeeperHub account + workflow registered | 🟡 distance 2 | Operator needs to sign up at app.keeperhub.io, register the reference workflow, get an API key. |
| Live `keeperhub-schedule` smoke | 🟡 distance 2 | `KEEPERHUB_API_KEY` + workflow ID configured, then one `live keeperhub-schedule` call. |
| SciCrucible `/agents/[id]` shows scheduled jobs | 🟡 distance 3 | After the live smoke, the agent detail page lists pending KeeperHub workflows. |

**Decision so far** (see parent repo's discussion). Stay runbook-only
through Saturday submission. Sign-up + integration is post-MVP unless
there's slack on Friday.

## ORCID

**Pitch.** Real human authorship via the academic-standard ORCID
identifier. Crucible posts authored by humans get an ORCID badge that
links back to the actual publication record. Combined with ENS for
agents, this gives a credible human-vs-agent provenance distinction
that no other hackathon submission has.

**Distance to live demo.**

| Component | State | Notes |
| --------- | ----- | ----- |
| `/auth/orcid` page (UI) | ✅ done | Static landing page in the dashboard. |
| `lib/session.ts` (jose JWT helpers) | ✅ done | Session signing primitives. |
| `OrcidGate` component | ✅ done | Wraps protected pages. |
| Backend spec (Supabase Auth + ORCID provider) | ✅ done | `docs/BACKEND_SPEC.md` §OAuth. |
| Live ORCID OAuth handshake | 🟡 distance 2 | Supabase project + ORCID app credentials, then wire the provider. |
| ORCID-gated submit end-to-end | 🟡 distance 2 | After OAuth lands, `/submit` is protected and `posts.author_orcid` is populated. |

## Uniswap

**Pitch.** Science service quotes (DFT, retrosynthesis, literature)
priced in real USDC via a Uniswap quote API; release of escrow on
result acknowledgement. ChimiaClaw already has the artifact-native
settlement state machine (quote → acceptance → escrow → release →
refund) under `chimiaclaw-market`; what's missing is the live USDC
quote and the live payment adapter.

**Distance to live demo.**

| Component | State | Notes |
| --------- | ----- | ----- |
| Settlement state machine (signed) | ✅ done | `ScienceEconomicSettlement::validate` covers quote/escrow/release/refund. |
| Three signed transaction flows (DFT, retro, literature) | ✅ done | `science-market-demo`. |
| Live Uniswap USDC quote | 🟡 distance 2 | API integration. |
| Operator-confirmed release on real USDC | 🟡 distance 3 | Requires a wallet adapter + signed release artifact + actual on-chain tx. |
| SciCrucible shows quoted prices live | 🟡 distance 3 | After the live quote, sector pages show service price ranges. |

## AXL — Cross-node messaging

**Pitch.** Lab-to-lab traffic (a request submitted on one lab's
SciCrucible posts to another lab's swarm) transits real AXL nodes,
not direct HTTP. Authority chain is preserved across node boundaries.

**Distance to live demo.** Currently shape-only (3+). No code yet.
This is the lowest-priority sponsor track for the submission window.

## Vercel (analytics + hosting)

**Pitch.** SciCrucible deploys cleanly on Vercel and uses
`@vercel/analytics` for first-party metrics. The mission-control UI is
purpose-built for Vercel's edge runtime.

**Distance to live demo.**

| Component | State | Notes |
| --------- | ----- | ----- |
| `@vercel/analytics` in package.json | ✅ done | Wired in root layout. |
| Vercel-friendly Next config | ✅ done | `images.unoptimized = true`, App Router, no edge-incompatible deps. |
| First Vercel preview deploy | 🟡 distance 1 | Push to GitHub + connect Vercel. |
| Production-ready config (typed strict, image opt) | 🟡 distance 2 | Tighten `next.config.mjs`. |

---

## Total prize-track distance

If we assume the chemistry-prize-track (real signed DFT result with
orbital cubes, lineage breadcrumb, and a clean URL on Vercel) is the
top priority, the critical path is:

1. **0** afternoons: chemistry artifacts already signed, cubes already
   rendered, gallery README already written. Done.
2. **1** afternoon: push SciCrucible to private GitHub + Vercel; bridge
   `/post/[id]` to the six existing signed `chem.dft.result` JSONs.
3. **1** afternoon: live ENS Sepolia smoke (read+write+verify three
   signed artifacts).
4. **1** afternoon: live 0G anchor on Galileo for one cube payload.

Total: ~3 afternoons of focused work to a public-facing
SciCrucible URL with real chemistry, real ENS verification, and a
real 0G anchor visible on the post detail page.

That is the prize-track demo we can credibly ship.
