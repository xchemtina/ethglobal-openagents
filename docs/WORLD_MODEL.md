# Frontend world model
`demo/world-model.json` is the current frontend-facing projection of ChimiaClaw. It is deliberately simpler than SciCrucible: a static lab-swarm map over the signed artifact DAG, not a full discourse platform, ORCID backend, RDF store, or live swarm bus.
## Purpose
The model gives the frontend a stable surface for the “agentic kingdom” view:
- three ChimiaDAO-controlled physical lab nodes
- allied physical labs that may become trusted collaborators
- virtual agent labs that can propose or simulate work but cannot claim physical custody
- unknown labs held in quarantine until identity, signature, payload, and lineage checks pass
- quests, artifact cards, agents, and trust edges that map back to the current CLI/runtime
- science service transactions for DFT, retrosynthesis, and literature
- MSSP as optimization genealogy over artifacts
- World Avatar as an RDF projection over artifacts
## Root shape
The JSON root contains:
- `design_language`: SciCrucible-inspired HUD styling tokens and maturity warnings
- `current_truth`: boundaries the UI should preserve
- `abstraction_principles`: non-negotiable modeling constraints
- `layers`: map, quests, artifacts, agents, science market, trust gates, MSSP, and World Avatar
- `sectors`: compact scientific/operational domains with stable colors
- `trust_tiers`: custody-core, allied-verified, virtual-sandbox, and quarantine
- `labs`: physical, allied, virtual, and unknown lab nodes
- `trust_edges`: allowed information/material flows between labs
- `agents`: implemented and planned swarm roles
- `quests`: UI work orders linked to current or future backend flows
- `science_transactions`: ENS-shaped service transaction cards for provider profile, offer, request, quote, quote acceptance, escrow authorization, settlement intent, result, acknowledgement, release, and refund-policy chains
- `artifact_cards`: symbolic frontend cards for schema-tagged signed artifacts
- `mssp_projection`: Marchev/MSSP skill families and demo generations as artifact genealogy
- `world_avatar_projection`: derived RDF/PROV-O/OntoChimia view
- `backend_bindings`: exact commands and runtime concepts the fixture maps to
- `activity_ticker`: deterministic ticker rows for the first frontend pass
## Backend mapping
The world model is not canonical state. The signed artifact DAG remains canonical.
Current mappings:
- lab nodes are frontend abstractions over future node profiles, custody policies, and operator gates
- quests map to CLI seed/run/inspect flows
- artifact cards map to schema tags and parent-child relationships in `FileArtifactStore`
- turns map to JSONL `RunCycleReport` rows emitted by `chimiaclaw-cli node run`
- swarm agents map to `chimiaclaw-skill` implementations when `status` is `implemented-local-skill`
- science transactions map to `chimiaclaw-market` fixtures and the `science-market-demo` CLI command, including simulated non-custodial settlement lifecycle records
- MSSP generations map to future `opt.cybernetic.*`, `opt.mssp.*`, and `opt.switcher.*` artifacts
- World Avatar triples map to derived `chimiaclaw-semantic-rdf` projections
- trust and custody are visible UI gates today; enforcement is planned through capabilities, safety artifacts, and governance anchors
Implemented local skill mappings:
- `AGENT.ORD.ADT` maps `chem.ord.reaction` to `chem.adt.reaction` through `chem.ord.to_adt.v1`
- `AGENT.RETROQUOTE` maps `chem.retrosynth.route_proposal` to `chem.procurement.route_quote` through `chem.procurement.supplier_quote.v1`
- `AGENT.MARKET.RETRO`, `AGENT.MARKET.DFT`, and `AGENT.MARKET.LIT` map service requests to signed deterministic market quotes/results and simulated settlement releases through `chimiaclaw-market`
Planned mappings:
- `AGENT.SAFETY.GATE` should produce `chem.safety.assessment`
- `AGENT.ARTIFACT.AUDIT` should produce lineage/review artifacts for trust decisions
- `AGENT.GOV.ANCHOR` should connect reviewed artifact bundles to the governance scaffold
## Demo commands
Print the static frontend model:
```sh
cargo run -p chimiaclaw-cli -- world-model
```
Print the science market transaction fixture:
```sh
cargo run -p chimiaclaw-cli -- science-market-demo
```
Serve the static visual map:
```sh
python3 -m http.server 8787 --directory demo
```
Then open `http://localhost:8787/world-map.html`.
Run the current local artifact flows behind the model:
```sh
STORE=$(mktemp -d /tmp/chimiaclaw-store-XXXXXX)
cargo run -p chimiaclaw-cli -- node seed-ord --store-dir "$STORE"
cargo run -p chimiaclaw-cli -- node seed-route --store-dir "$STORE"
cargo run -p chimiaclaw-cli -- node run --store-dir "$STORE" --max-cycles 3 --interval-ms 0
cargo run -p chimiaclaw-cli -- artifact inspect --store-dir "$STORE"
```
## UI guidance
The frontend should render this as a command surface, not a gameboard that hides risk.
- show precise trust tier and custody labels on every lab
- separate virtual planning from physical execution
- show unknown labs as quarantined by default
- show artifact lineage and schema tags before scientific claims
- show science service market flows as fixture/planned-live until live sponsor integrations are wired
- show payer, payee, quote amount, simulated escrow/release state, and refund policy before any claim of payment
- show planned features as locked/gated rather than live
- show MSSP as artifact genealogy until optimizer execution exists
- show World Avatar as semantic projection until a live federated KG exists
- avoid exact physical coordinates unless an operator explicitly chooses to disclose them
