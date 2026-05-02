# ChimiaClaw — Speedrun summary

## What we are building

ChimiaClaw is a Rust-native runtime for autonomous scientific agents. Every agent action — chemistry computation, retrosynthesis, literature synthesis, procurement, governance vote, payment — is a signed, content-addressed artifact in a payload-bound DAG. ChimiaClaw is the substrate ChimiaDAO will run on.

## What is unique

- The artifact DAG is the canonical state, not a log next to a database.
- Economic settlement is itself an artifact lifecycle: quote, acceptance, escrow, settlement intent, result, acknowledgement, release, refund — each signed and content-addressed.
- Settlement is non-custodial by default. Live fund movement is opt-in, gated, and only ever invoked by an explicit adapter call.
- Identity is ENS-shaped from day one; storage is designed for 0G; long jobs are designed for KeeperHub; cross-node messaging is designed for AXL; live quoting is designed for Uniswap. None of these are load-bearing for core function.
- Audit-over-trust is enforced structurally. There is no "agent said so" surface; every claim is reproducible from the signed payload.

## What is real today

- A Rust workspace with the artifact substrate, file-backed store, polling runtime, ORD→ADT bridge, retrosynthesis route-quote engine, and a deterministic ENS-shaped science service market.
- A new `chimiaclaw-crucible` crate (`crates/chimiaclaw-crucible/`) that defines the `crucible.review.vote` signed-artifact schema for the SciCrucible discourse layer: closed `VoteKind` enum, layered `VoterIdentity` (ORCID / ENS / eth address), content-hash binding so a vote is invalidated by tampering, signed-artifact construction parented to the target. 8 unit tests passing.
- The SciCrucible_v1 dashboard (`SciCrucible_v1/`) now ships a `/dft` route surface: a typed loader at `SciCrucible_v1/lib/dft-artifacts.ts` reads the six signed `chem.dft.result` artifacts (and their `chem.dft.request` / `chem.molecule.adt` parents) at build time, hex-decodes the inline payloads, and renders them as a six-card gallery (`/dft`) plus per-artifact detail page (`/dft/[id]`) with all three orbital cubes (HOMO, LUMO, total density) inline. 18 signed JSONs (~140 KB) and 18 cube PNGs (~1.3 MB) live under `SciCrucible_v1/public/{artifacts,orbitals}/` and are embedded in the static bundle on Vercel.
- The static lab-swarm map (`demo/world-map.html` + `demo/world-model.json`) now visualises the distributed n×(AI+Scientist) network with real data: a new `LAB.CHIMIA.04` Olympus DFT Worker node, two new active trust edges (Olympus→Analysis-Dock for result return, Virtual-Retro-Swarm→Olympus for compute dispatch), and six `science_transactions` tagged `real-execution` whose `result_id` fields point to the actual signed `art_*` IDs. Selecting a lab now (a) draws animated dashed flow lines from an operator anchor to that lab, one per incoming transaction colored by service kind, (b) draws inbound dispatch and outbound result-return lines along the matching trust edges, (c) highlights the static trust edges that involve the selected lab, and (d) filters the science-transactions panel grouped by `service_kind` with per-group real-exec counts.
- A signed economic settlement state machine in `chimiaclaw-market`: quote acceptance, escrow authorization, settlement intent, result acknowledgement, simulated release, and a refund-after-rejection alternative path. All paths are validated by `ScienceEconomicSettlement::validate`.
- A canonical Molecular ADT substrate (`chimiaclaw-moladt`) with a curated SMILES library, a Cordero-2008 covalent-radii geometry guesser, a deterministic pure-Rust SVG renderer, an XYZ writer, an `ord_moladt` translator that walks an ORD reaction into signed `chem.molecule.adt` artifacts, and a uv-managed RDKit worker behind `CHIMIACLAW_SMILES_TO_MOLADT_COMMAND` for SMILES outside the curated library. The DFT branch of `chimiaclaw-market` now consumes this substrate end-to-end via a Skala-1.1 / def2-tzvp `chem.dft.request` artifact.
- An ASKCOS retrosynthesis adapter (`chimiaclaw-retrosynth-askcos`) plus a uv-managed `askcos-retro` worker behind `CHIMIACLAW_ASKCOS_ENDPOINT` + `CHIMIACLAW_ASKCOS_COMMAND` that signs a `chem.retrosynth.template_suggestions` artifact, refusing to invoke ASKCOS without explicit configuration and rejecting empty proposals.
- A CLI (`chimiaclaw-cli`) with reproducible deterministic demos: `demo-dag`, `demo-ord-adt`, `world-model`, `science-market-demo`, `moladt-dft-demo`, `ord-moladt-demo` (`--output-dir` writes per-substrate XYZ + SVG), and `moladt-render`, plus a real local polling loop (`node seed-*` / `node run` / `artifact inspect`).
- A dependency-free static HUD (`demo/world-map.html` over `demo/world-model.json`) that renders the lab swarm, science transactions, simulated escrow/release, refund-to agent, MSSP genealogy, and a World Avatar RDF projection.
- A pre-rendered MolADT gallery at `demo/molecules/` covering the entire curated library plus seven RDKit-tier renders (benzaldehyde, aspirin, salicylic acid, pyridine, methylamine, imidazole, acetone) so the geometry-tier story is visible at a glance.
- Feature-gated live sponsor surfaces: ENS read-only verification artifacts, 0G upload anchor artifacts through a private-key-safe wrapper boundary, and KeeperHub workflow schedule/status artifacts.
- ENS write-side publication path: a uv worker (`identity-ens`) drives `ens.set_text` idempotently behind `CHIMIACLAW_ENS_PUBLISH_COMMAND`, and `chimiaclaw-cli live ens-publish` chains publication → resolver → verifier into three signed artifacts (`identity.ens.publication`, `identity.ens.resolution`, `identity.ens.verification`).
- 0G stub mode (`ZEROG_STUB=1`) hashes the file with Blake2b-32 and emits a deterministic receipt with explicit `STUB MODE` audit notes; an end-to-end stub run produced signed `storage.zerog.upload` artifact `art_62a1177fa495209f` parented to a real ferrocene `MoleculeAdt` artifact, captured at `demo/zerog/anchor-stub.json`.
- KeeperHub workflow runbook at `demo/keeperhub/{workflow.json,README.md}` documents the manual-trigger workflow and the DFT request → KeeperHub schedule → 0G anchor chain that the existing Rust REST client already executes.
- **Six-molecule signed DFT gallery with orbital density cubes**: water, methanol, benzene, propylene glycol, caprylic acid (C8), capric acid (C10) all SCF-converged via PBE/def2-tzvp on `duck@olympus.local` (PySCF 2.13.0). Each `chem.dft.result` carries energy, HOMO/LUMO/gap, dipole, and three SHA-256-committed `.cube` files (HOMO orbital, LUMO orbital, total electron density) generated via `pyscf.tools.cubegen`. 18 cubes (~28 MB) under `demo/dft/cubes/`, viewable in VMD/PyMOL/Avogadro. Reproducible with `chimiaclaw-cli moladt-dft-demo --smiles ... --out-dir demo/dft/` followed by `live dft-execute --cube-out-dir demo/dft/cubes`.
- DFT crate `chimiaclaw-dft-skala` + uv worker `chimiaclaw-dft` ship the chem.dft.result schema, signing path, PySCF backend, and orbital-density cube generation. The Rust signer refuses unconverged or schema-mismatched results, and re-hashes every cube locally before signing so transport tampering can never reach the artifact graph.
- A Foundry contract scaffold for capability tokens, proposal anchoring, and reputation, with all current tests passing.
- A first public commit with `cargo fmt`, `cargo check --workspace`, `cargo test --workspace`, `cargo check --workspace --all-features`, and `forge test --root contracts` all green.

## What is honest about the gaps

- ENS publication, 0G upload, and KeeperHub all have feature-gated adapter surfaces and operator runbooks, but live smoke runs still require operator-provided RPC/API credentials, a funded controller, an installed `0g-storage-client` binary, and a registered KeeperHub workflow.
- AXL and Uniswap are currently shape-only attachment points, not live integrations.
- Governance execution semantics (quorum, vote weighting, treasury authority) are scaffolded, not enforced.
- The direct `chimiaclaw-node` daemon binary, capability checks before skill execution, and richer runtime metrics are still on the build path.

## What we will ship next, in order

1. Real Skala/PySCF/GPU4PySCF DFT execution: a `CHIMIACLAW_DFT_COMMAND` wrapper consumes the signed `chem.dft.request` artifact (with its MolADT parent) on `duck@olympus.local` and emits a `chem.dft.result` artifact with energies/orbitals/timings.
2. Wire `chimiaclaw-retrosynth-askcos` into `apps/retroquoter` so the deterministic route quote becomes a child of a real ASKCOS template-suggestions artifact, plus a curated in-stock filter and disk cache for ASKCOS responses.
3. Live ENS smoke: with an operator-supplied testnet ENS name and funded controller key, run `demo/ens-roundtrip.sh` to publish + resolve + verify three signed artifacts on real Sepolia state.
4. Live 0G smoke: install `0g-storage-client`, drop `ZEROG_STUB`, and replay `demo/zerog-roundtrip.sh` to anchor a real payload on Galileo turbo.
5. Live KeeperHub job: register `demo/keeperhub/workflow.json`, schedule it via `live keeperhub-schedule`, and emit signed schedule/status artifacts.
6. Live Uniswap quote: real USDC quote populates the science quote price; release still requires explicit operator confirmation.
7. Live AXL cross-node: one service request and one signed result transit two real AXL nodes.
8. Governance execution: proposal artifact → vote bundle → on-chain anchored execution receipt.

Each of these lands as a sponsor-credible artifact extension with passing tests and a documented smoke command.

## Why now

Scientific agents are about to scale. The market currently lacks an audit-first, economically-settleable, decentralized runtime. ChimiaClaw is that runtime.

## Why us

We are building this in the open, in Rust, with no Docker, no Homebrew, and no cloud lock-in, against a long-running ChimiaDAO research direction in chemistry, agent autonomy, governance, and decentralized scientific infrastructure. Every claim in this document is reproducible from the repository.

## Ask

Speedrun runway and network to harden the live sponsor integrations (ENS, 0G, KeeperHub, Uniswap, AXL), expand the science skill set, bootstrap the first ChimiaDAO governance flows on top of the artifact DAG, and ship a reference swarm that other scientific teams can fork.
