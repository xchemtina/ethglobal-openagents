# Submission checklist — ETHGlobal OpenAgents

Deadline: **Sunday, May 3, 2026 at 12:00 pm EDT** (~13 hours from now).

## Target prizes

1. **ENS — Best ENS Integration for AI Agents** ($2,500)
2. **0G — Best Agent Framework, Tooling & Core Extensions** ($7,500)
3. **Uniswap — Best Uniswap API Integration** ($5,000)
4. **KeeperHub — Builder Feedback Bounty** ($250)

## Before submission

- [ ] **ENS Sepolia roundtrip / capability publication**: run `demo/ens-roundtrip.sh` or the ENS phase in `demo/overnight-full-pipeline.sh` with live credentials. Commit only non-secret signed artifacts intended for the submission.
- [ ] **Verify all demos still pass after ENS commit**:
  - `cargo run -p chimiaclaw-cli -- demo-dag`
  - `cargo run -p chimiaclaw-cli -- science-market-demo`
  - `cargo run -p chimiaclaw-cli -- moladt-dft-demo`
  - `python3 -m py_compile demo/live-dashboard-watch.py`
  - `python3 demo/live-dashboard-watch.py --once`
  - `cargo run -p chimiaclaw-cli -- world-model verify --world-model demo/world-model.json --artifact-dir demo`
  - `cargo run -p chimiaclaw-cli -- world-model verify --world-model demo/world-model.live.json --artifact-dir demo`
  - `cargo run -p chimiaclaw-cli -- crucible-demo`
- [ ] **Full test suite**: `cargo test --workspace`
- [ ] **All-features check**: `cargo check --workspace --all-features`
- [ ] **Repo audit**: `rg -i 'a16z|cris|/Users/' -g '!Cargo.lock' -g '!*.json' -g '!target' -g '!.git' -g '!*.lock'` — only operator-runbook examples.
- [ ] **Git log audit**: `git --no-pager log --oneline -20` — every commit authored as `ChimiaDAO <info@chimiadao.io>`.
- [ ] **Flip repo to public**: GitHub → Settings → Change visibility → Public.

## Submission form

- [ ] Project name: **ChimiaClaw**
- [ ] Short description (1-2 sentences): Rust-native signed artifact DAG for autonomous scientific agents with live ENS identity, portable molecular ADT, real DFT computation, quote-only Uniswap settlement artifacts, 0G anchoring, and an auto-refreshing local evidence dashboard.
- [ ] Public GitHub repo link
- [ ] Demo video link (2-4 min, no AI voiceover, ≥720p)
- [ ] Live demo link (Vercel SciCrucible dashboard + `world-map.html`; note that `world-model.live.json` is a local projection if used in the video)
- [ ] Contract deployment addresses (Foundry scaffold — note testnet if deployed)
- [ ] Team member names and contact info (Telegram & X)

## Per-prize requirements

### ENS
- [ ] Explain how ENS is the identity mechanism for agents (text-record profiles, discovery, verification).
- [ ] Demo is functional with no hard-coded values (live Sepolia artifacts prove this).
- [ ] Video or live demo link.

### 0G
- [ ] At least one working example agent built using the framework (the ORD→ADT skill in the polling runtime).
- [ ] Architecture diagram (use the mermaid diagram from `docs/ARCHITECTURE.md`).
- [ ] Explain which 0G features/SDKs are used (0G Storage upload wrapper, storage URI/root hash receipts, signed anchor artifacts).

### Uniswap
- [ ] `FEEDBACK.md` in repo root with Uniswap Trade API builder feedback (present alongside KeeperHub feedback).
- [ ] `chimiaclaw-settle-uniswap` crate calls `POST /v1/quote` and emits signed `market.uniswap.quote` artifact.
- [ ] Retrosynthesis / DFT demo flows use `UniswapPreparedTransfer` settlement shape where applicable.
- [ ] `live uniswap-quote` CLI subcommand (behind `--features live-sponsors`) signs quote-only `market.uniswap.quote` artifacts; no `/swap` or fund movement without operator confirmation.

### KeeperHub
- [ ] `FEEDBACK.md` in repo root with honest builder feedback.

## Demo video script (2:25–3:05 diagram-first target)

Shorter is fine. Keep the human voiceover minimal: one sentence per screen, mostly over dashboard footage.

1. (0:00–0:10) Personal opener: “I’m [name] from ChimiaDAO, a decentralized chemistry research project using decentralized collaboration and blockchain technology to make scientific work verifiable.”
2. (0:10–0:30) Architecture diagram: papers become signed literature artifacts, then signed route artifacts, then signed DFT result artifacts.
3. (0:30–1:00) Dashboard: show `world-map.html`, title, source pill, and topbar. Say the browser reads a local projection; signed artifacts remain source truth.
4. (1:00–1:25) Three lanes: Literature is next/operator-gated, RetroQuoter is implemented, DFT is real PySCF execution.
5. (1:25–2:05) Hero shot: switch through the six WebGPU HOMO/LUMO molecule tabs and mention signed cube commitments.
6. (2:05–2:30) Evidence cards: DFT, ENS, 0G, Uniswap. Keep caveats tight: scalar-only DFT has no cubes, ENS root is live, 0G proves anchors, Uniswap is quote-only.
7. (2:30–2:50) Verifier: show `world-model verify` resolving dashboard references back to signed artifacts.
8. (2:50–3:05) Monetization diagram: quote → acceptance → escrow intent → result → acknowledgement → release, with refunds on failure/cancel/expiry.
