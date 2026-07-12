# Future improvements

Post-hackathon roadmap distilled from an architectural review of the ChimiaClaw substrate.

## Phase 0 — Honesty cleanup
- Cull the 14 empty scaffold crates (`chimiaclaw-agent`, `chimiaclaw-mutator`, `chimiaclaw-optimization`, `chimiaclaw-governance`, `chimiaclaw-transport-axl`, `chimiaclaw-inft`, `chimiaclaw-onchain-pox`, `chimiaclaw-settle-uniswap`, `chimiaclaw-semantic-rdf`, `chimiaclaw-skill-subprocess`, `chimiaclaw-skill-wasm`, `chimiaclaw-reputation`, `apps/dft-daemon`, `apps/marchev-mssp`). Move referenced type definitions into a single `chimiaclaw-future` crate or inline them.
- Migrate CLI from manual `match argv.as_slice()` to `clap` derive macros.
- Fix Solidity tests: replace raw `require` with Forge-native `assertEq` / `vm.expectRevert`; add edge-case coverage (unauthorized release, double-mint, zero-value escrow).

## Phase 1 — Make the node a service
- Add `tokio` async runtime; replace `std::thread::sleep` polling with `tokio::time::interval`.
- Add a JSON-RPC or gRPC listener (`submit_artifact`, `get_artifact`, `list_artifacts`, `get_children`).
- Implement two-node artifact replication over the RPC layer — the minimum viable swarm.
- Wire the daemon binary (`chimiaclaw-node/src/main.rs` currently prints a placeholder).

## Phase 2 — Replace fixtures with real dispatch
- Market request routing: a node publishes a `ServiceOffer`; another submits a `ServiceRequest`; the runtime matches by schema tag and dispatches. Replace `demo_science_market()` hardcoded flows with runtime-generated artifacts.
- Live worker execution loop: when a DFT request artifact lands and a registered skill matches, the node invokes the PySCF worker and signs the result automatically.
- Demote the curated molecule library to test-only fallback; make the RDKit worker the default SMILES path.

## Phase 3 — Real governance
- Implement quorum and reputation-weighted vote counting in `ChimiaGovernor.sol`. Add configurable quorum threshold and execution timelock.
- Wire on-chain anchoring from the Rust side: a CLI command and a skill that anchor high-value artifact roots to `ProposalRegistry`.
- Replace dev signer seeds with proper key generation and storage (keyfile with file-permission enforcement, or macOS Keychain integration).

## Phase 4 — Real integrations
- Wire `LiveEnsResolver` into node startup so provider profiles resolve from the chain.
- Install `0g-storage-client` and persist large payloads (DFT cube files) to 0G Storage.
- Wire KeeperHub REST client to schedule PySCF jobs from DFT request artifacts.
- Replace `SimulatedArtifactLedger` with real Uniswap API quotes, producing `PreparedLiveTransfer` artifacts requiring operator approval.

## What not to do (yet)
- No AXL/cross-chain transport until two nodes reliably share artifacts over localhost.
- No MSSP/Marchev optimization until the reactor can score and dispatch real needs.
- No World Avatar RDF projection until there is a real artifact flow to project.
- No frontend backend API until Phase 1 RPC is running.
