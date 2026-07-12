# Morning checklist
This is the operator-only handoff for the ETHGlobal OpenAgents submission.
Everything in this list **requires your local credentials or your eyes** —
none of it could safely happen overnight without you.
## 0. Preflight (5 minutes)
Run the integrity check before anything else:
```
python3 -m py_compile demo/live-dashboard-watch.py
python3 demo/live-dashboard-watch.py --once
cargo run -p chimiaclaw-cli -- world-model verify --world-model demo/world-model.json --artifact-dir demo
cargo run -p chimiaclaw-cli -- world-model verify --world-model demo/world-model.live.json --artifact-dir demo
```
Expected: static and live world-model references verify against `demo/`.
The static model should include the six cube-backed orbital source artifacts,
the representative DFT artifact, and the five overnight germanium scalar DFT
artifacts. The live model should also contain `overnight_full_pipeline.counts`
and any referenced live DFT, Uniswap, 0G, or ENS artifacts. If anything fails,
**do not flip public** until it's fixed.
Smoke-test the discourse path end-to-end:
```
cargo run -p chimiaclaw-cli -- crucible-demo --out-dir /tmp/crucible-demo
```
Expected: two signed `crucible_review_vote.art_*.json` files written and
the JSON manifest printed. The CLI surface for one-off votes is
`crucible vote --target-id ... --target-content-hash ... --kind ...
--orcid ...`.
Open the dashboard locally:
```
python3 -m http.server 8787 --directory demo
# then http://localhost:8787/world-map.html
```
Expected: focused Literature → Retrosynthesis → DFT dashboard. If
`demo/world-model.live.json` exists, the source pill should show it; otherwise
it should show `world-model.json`. The WebGPU gallery should have exactly six
cube-backed molecule tabs, and scalar-only overnight DFT results should appear
as evidence cards rather than orbital tabs.
## 1. ENS Sepolia smoke (15-30 minutes)
**Need**: a Sepolia ENS name you own, a controller key with gas
(get gas from the `pk910.de` PoW faucet if you don't have any),
a Sepolia RPC URL.
```
export CHIMIACLAW_AGENT=<your.eth>
export CHIMIACLAW_ENS=<your.eth>
export ENS_WRITE_RPC_URL=<sepolia-rpc>
export ENS_WRITE_PRIVATE_KEY=<key-as-env-only-never-argv>
export ENS_RPC_URL=$ENS_WRITE_RPC_URL
demo/ens-roundtrip.sh
```
Expected: three signed artifacts in `demo/ens-out/`
(`identity.ens.publication`, `identity.ens.resolution`,
`identity.ens.verification`).
Commit those three artifacts to the repo, push.
## 2. 0G Galileo anchor (15 minutes)
**Need**: `0g-storage-client` binary on `$PATH` and a Galileo testnet key
with tokens.
```
export ZEROG_PRIVATE_KEY=<env-only>
demo/zerog-roundtrip.sh
```
Expected: a signed `storage.zerog.upload` artifact in
`demo/zerog-out/`. If the binary isn't installed yet, leave the existing
`ZEROG_STUB=1` artifact in place and skip — the substrate already
demonstrates the signing path.
## 3. Vercel deploy preview (10 minutes)
v0 has been iterating on the dashboard import overnight. Pull whatever
v0 produced, then push to GitHub. Vercel will auto-deploy a preview.
The static `/dft` route surface from `SciCrucible_v1/` should still
render the six real DFT artifacts, regardless of whatever v0 did to the
fixture pages.
## 4. Final repo audit (10 minutes)
```
git --no-pager log --oneline -20
```
Eyeball: every commit should be authored as
`ChimiaDAO <info@chimiadao.io>` and co-authored by `Oz`.
```
rg -i 'a16z|cris|/Users/' -g '!Cargo.lock' -g '!*.json' -g '!target' -g '!.git'
```
Expected: zero hits except in this file's path-style examples and
explicit operator-runbook env-var docs.
## 5. Flip private -> public
On GitHub: `xchemtina/ethglobal-openagents` -> Settings -> Change
visibility -> Public. Confirm by typing the repo name.
## 6. Submit
ETHGlobal submission form. Link the now-public repo, the Vercel preview,
and the deployed `world-map.html`.
## What's already landed (no action needed)
- Six real PBE/def2-tzvp DFT artifacts under `demo/dft/`
  (water, methanol, benzene, propylene glycol, caprylic acid C8,
  capric acid C10) with HOMO/LUMO/total-density cubes.
- `chimiaclaw-crucible` workspace crate with
  `crucible.review.vote` schema (8 unit tests passing).
- `/dft` Next.js route surface in `SciCrucible_v1/` rendering the six
  real artifacts and 18 cube PNGs.
- Focused `world-map.html` with a central six-molecule WebGPU HOMO/LUMO
  gallery, visible bond skeletons, scalar overnight DFT evidence, and
  optional live local projection from `demo/world-model.live.json`.
- `demo/live-dashboard-watch.py` for auto-refreshing the dashboard while
  `demo/overnight-full-pipeline.sh` emits DFT, Uniswap quote-only, 0G, and ENS
  artifacts.
- CLI commands: `crucible vote`, `crucible-demo`, `world-model verify`.
- LICENSE (Apache-2.0), NOTICE, CITATION.cff, info@chimiadao.io author
  attribution across the whole git history.
## Strict no-go zones
- Don't push directly to `master` while v0 is still iterating; pull v0's
  branch first.
- Don't run live ENS / 0G / KeeperHub calls without verifying that you
  are on a testnet, that the controller key is intentional, and that
  `--allow-mainnet` is **not** present.
- Don't move funds. Every settlement artifact in the repo is
  `SimulatedArtifactLedger`; the on-chain settlement adapter is opt-in
  only.
