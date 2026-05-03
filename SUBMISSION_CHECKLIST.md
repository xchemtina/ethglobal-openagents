# Submission checklist — ETHGlobal OpenAgents

Deadline: **Sunday, May 3, 2026 at 12:00 pm EDT** (~13 hours from now).

## Target prizes

1. **ENS — Best ENS Integration for AI Agents** ($2,500)
2. **0G — Best Agent Framework, Tooling & Core Extensions** ($7,500)
3. **KeeperHub — Builder Feedback Bounty** ($250)

## Before submission

- [ ] **ENS Sepolia roundtrip**: run `demo/ens-roundtrip.sh` with live credentials. Commit the three signed artifacts from `demo/ens-out/` to the repo.
- [ ] **Verify all demos still pass after ENS commit**:
  - `cargo run -p chimiaclaw-cli -- demo-dag`
  - `cargo run -p chimiaclaw-cli -- science-market-demo`
  - `cargo run -p chimiaclaw-cli -- moladt-dft-demo`
  - `cargo run -p chimiaclaw-cli -- world-model verify`
  - `cargo run -p chimiaclaw-cli -- crucible-demo`
- [ ] **Full test suite**: `cargo test --workspace`
- [ ] **All-features check**: `cargo check --workspace --all-features`
- [ ] **Repo audit**: `rg -i 'a16z|cris|/Users/' -g '!Cargo.lock' -g '!*.json' -g '!target' -g '!.git' -g '!*.lock'` — only operator-runbook examples.
- [ ] **Git log audit**: `git --no-pager log --oneline -20` — every commit authored as `ChimiaDAO <info@chimiadao.io>`.
- [ ] **Flip repo to public**: GitHub → Settings → Change visibility → Public.

## Submission form

- [ ] Project name: **ChimiaClaw**
- [ ] Short description (1-2 sentences): Rust-native signed artifact DAG for autonomous scientific agents with live ENS identity, a portable molecular ADT, real DFT computation with orbital densities, and a deterministic science service market.
- [ ] Public GitHub repo link
- [ ] Demo video link (2-4 min, no AI voiceover, ≥720p)
- [ ] Live demo link (Vercel SciCrucible dashboard + `world-map.html`)
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
- [ ] Explain which 0G features/SDKs are used (0G Storage URI hints, stub upload, signed anchor artifacts).

### KeeperHub
- [ ] `FEEDBACK.md` in repo root with honest builder feedback.

## Demo video script (3 min target)

1. (30s) What ChimiaClaw is — one slide, the core idea mermaid diagram.
2. (30s) Show `demo-dag` output — route → quote → receipt chain, each artifact signed and payload-bound.
3. (30s) Show `science-market-demo` — three service flows with full economic settlement lifecycle.
4. (30s) Show the six real DFT artifacts — orbital density cubes rendered in SciCrucible.
5. (30s) Show the live ENS Sepolia artifacts — publish → resolve → verify, real on-chain text records.
6. (30s) Show `world-map.html` — the lab-swarm with real nodes, trust edges, science transactions.
