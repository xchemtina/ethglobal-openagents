# ChimiaClaw — a16z Speedrun application

ChimiaClaw is the runtime layer that turns autonomous scientific work into signed, auditable, and economically settleable artifacts. We are building it to be the substrate for ChimiaDAO: a decentralized scientific organization where compute, lab work, literature, and governance are all expressed as verifiable artifacts in a payload-bound DAG.

## One-line pitch

A Rust-native runtime where every scientific action — a retrosynthesis route, a DFT calculation, a literature synthesis, a procurement decision, a governance vote — is a signed, content-addressed artifact in a DAG, and where ENS-shaped scientific service agents transact with non-custodial, fully-auditable economic settlement.

## Why this matters now

Scientific agents are about to flood the internet. Most of what is being shipped is one of:

- a chat wrapper that calls a model and pretends the answer is truth,
- a notebook layer with no provenance, no signatures, no governance, and no economic primitives,
- a "lab automation" demo with no path to multi-party trust, no audit trail, and no exit from a single vendor's cloud.

None of those can be plugged into a DAO that owns physical or virtual labs. None of them can carry a chemical safety claim, a procurement receipt, a DFT output, or a compute payment across two organizations without a centralized broker.

ChimiaClaw is the missing substrate:

- **Every action is a signed artifact.** Every artifact commits to its payload via content hash, parents via lineage, and producer via signature.
- **Every economic event is also an artifact.** Quotes, acceptances, escrow authorizations, settlement intents, results, acknowledgements, releases, and refunds are all signed records — not opaque side effects of a backend.
- **Identity is portable.** Agents are addressed via ENS-shaped names like `dft.service.chimiaclaw.eth`. Profiles, offers, capabilities, and reputation are designed to live in ENS text records and on-chain reputation contracts.
- **Storage is decentralized.** Large payloads (DFT outputs, literature corpora, lab traces) are designed to live on 0G Storage with on-chain anchors.
- **Execution is reliable.** Long-running scientific jobs (DFT runs, literature pipelines, settlement releases) are designed to be scheduled through KeeperHub with signed completion artifacts.
- **Funds never move silently.** Settlement is non-custodial by default: the runtime emits a simulated release artifact today and only invokes a live payment adapter when an operator explicitly confirms.

## What is real today

Run any of these from a clean clone (no Docker, no Homebrew, no proprietary infra):

```sh
cargo run -p chimiaclaw-cli -- demo-dag
cargo run -p chimiaclaw-cli -- demo-ord-adt
cargo run -p chimiaclaw-cli -- world-model
cargo run -p chimiaclaw-cli -- science-market-demo
python3 -m http.server 8787 --directory demo
# open http://localhost:8787/world-map.html
```

Each command produces deterministic, signed, payload-bound artifacts. The static `world-map.html` is a dependency-free HUD over `demo/world-model.json`; it renders labs, trust edges, quests, science transactions (including payer, payee, simulated escrow, simulated release, refund-to agent), MSSP genealogy, and a World Avatar RDF projection.

`science-market-demo` emits three deterministic ENS-shaped service flows (retrosynthesis, DFT, literature). Each flow is ten signed artifacts:

1. provider profile
2. service offer
3. service request
4. service quote
5. quote acceptance
6. escrow authorization (simulated)
7. settlement intent
8. service result
9. result acknowledgement
10. settlement release (simulated)

A refund artifact is emitted instead of a release if the operator rejects the result, the provider fails, the quote expires, or the operator cancels before execution. The payer is `operator.chimiaclaw.eth`. The payees are `retro.service.chimiaclaw.eth`, `dft.service.chimiaclaw.eth`, and `literature.service.chimiaclaw.eth`. Amounts are deterministic USDC fixtures (3.10, 8.25, 1.50). No live funds move.

The local file-backed runtime (`chimiaclaw-node`) polls a `FileArtifactStore` and runs registered skills idempotently:

```sh
STORE=$(mktemp -d /tmp/chimiaclaw-store-XXXXXX)
cargo run -p chimiaclaw-cli -- node seed-ord --store-dir "$STORE"
cargo run -p chimiaclaw-cli -- node seed-route --store-dir "$STORE"
cargo run -p chimiaclaw-cli -- node run --store-dir "$STORE" --max-cycles 3 --interval-ms 1000
cargo run -p chimiaclaw-cli -- artifact inspect --store-dir "$STORE"
```

Repeated cycles are idempotent: parents that already have a child from a given skill are skipped.

A Foundry contract scaffold (`forge test --root contracts`) currently passes anchoring, capability-token, and reputation tests.

Feature-gated live sponsor surfaces now compile behind `--features live-sponsors`:

- ENS read-only JSON-RPC resolution and signed `identity.ens.resolution` / `identity.ens.verification` artifacts.
- 0G Storage upload anchor artifacts via an operator-provided `ZEROG_UPLOAD_COMMAND` wrapper that keeps private keys in environment variables.
- KeeperHub workflow execution/status REST client and signed `exec.keeperhub.scheduled` / `exec.keeperhub.completed` / `exec.keeperhub.failed` artifacts.

See `docs/speedrun/INTEGRATIONS.md` for environment variables and smoke commands.

## What is honest about the gaps

We are explicit about what is not yet live:

- ENS, 0G, and KeeperHub now have feature-gated adapter surfaces, but live smoke runs still require operator-provided RPC/API credentials and testnet setup.
- 0G upload is intentionally routed through an operator wrapper so private keys never appear in process arguments; the next hardening step is a first successful Galileo upload and a committed wrapper template.
- KeeperHub scheduling has a REST client and signed artifacts, but a first project-specific workflow still needs to be created in KeeperHub and run with `KEEPERHUB_API_KEY`.
- AXL transport is currently shape-only; peer IDs exist but no cross-node AXL traffic has been sent.
- Uniswap settlement is currently shape-only; route hints and a settlement intent exist but no live Uniswap quote is fetched and no funds move.
- Governance execution semantics (quorum, vote weighting, treasury authority) are scaffolded but not enforced.

This is not vaporware framing. The artifact substrate, the ORD→ADT bridge, the deterministic procurement quote engine, the file-backed runtime, the polling loop, the science market lifecycle, and the static HUD are all real, tested, and reproducible. The integrations above are next on the build path.

## Why we are differentiated

- **Audit-over-trust by default.** Every claim a ChimiaClaw agent makes is a signed artifact. There is no "agent told me so" surface. Operators can re-derive results from payloads.
- **Economic primitives are first-class.** Most agent frameworks model "tools" but not money. ChimiaClaw treats acceptance, escrow, acknowledgement, release, and refund as the same kind of object as a chemistry result. That makes scientific commerce auditable.
- **Non-custodial by construction.** Live fund movement is opt-in and gated by explicit operator confirmation. The default state is "signed intent only".
- **Rust-native, dependency-light.** No Python runtime in the hot path, no Docker, no cloud lock-in. Local-first; cloud and on-chain are projections.
- **DAO-aligned from day one.** The artifact DAG is the canonical state. Governance, reputation, and capability tokens are anchored on-chain over the same DAG.
- **Designed for sponsor stacks (ENS, AXL, 0G, Uniswap, KeeperHub) without depending on any of them for core function.** Every adapter is an attachment point, not a load-bearing dependency.

## Where we are going (sponsor-track roadmap)

Near-term build plan, in order of submission credibility:

1. **ENS live smoke.** Register or resolve real `*.chimiaclaw.eth` text records (profile CID, capabilities, settlement endpoint), then publish the signed verification artifact in the demo.
2. **0G Storage write.** Persist large request/result payloads and the service catalog root through 0G; record the returned root in a signed `storage.zerog.upload` anchor artifact.
3. **KeeperHub scheduling.** Create one DFT or literature workflow in KeeperHub, schedule it through the feature-gated client, and publish signed scheduled/completed artifacts tying back to the result.
4. **Uniswap quote.** Replace the deterministic quote price with a live Uniswap-derived USDC quote, still emitting acceptance/escrow/release as signed artifacts and still requiring explicit operator confirmation before any token transfer.
5. **AXL cross-node.** Send one service request and one signed result across two real AXL nodes, demonstrating that the artifact DAG composes across machines.
6. **Governance execution.** Wire the contract scaffold to the governance crate so a proposal artifact, a vote bundle, and an execution receipt form a complete on-chain anchor.

Each of these lands as a sponsor-credible artifact addition with passing tests and a smoke command in the README.

## Why fund this through Speedrun

ChimiaClaw is the runtime kernel for a decentralized scientific organization. We are not asking for permission to build it; we are already building it. We are asking for the runway and network to:

- harden the live sponsor integrations (ENS, 0G, KeeperHub, Uniswap, AXL),
- expand the science skill set (chemistry safety, DFT, retrosynthesis, literature),
- bootstrap the first ChimiaDAO governance flows on top of the artifact DAG,
- and ship a reference swarm that other scientific teams can fork.

The artifact substrate is the leverage point. Once every scientific action is a signed artifact, every other primitive — payment, governance, custody, reputation, anchoring — composes on top of it.

## Current operational state

- Repository: `/Users/crischimiadao/OpenAgents` (Rust workspace + Foundry scaffold + dependency-free static HUD).
- Validation: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo check --workspace --all-features`, `forge test --root contracts` all pass.
- First public commit: `feat: add signed artifact DAG runtime, lab-swarm world model, and science settlement lifecycle`.

## Contact

ChimiaDAO. Operator: `operator.chimiaclaw.eth` (ENS-shaped identity; live verification requires `ENS_RPC_URL`). All design decisions are tracked in `docs/`. All claims in this document are reproducible from the repository.
