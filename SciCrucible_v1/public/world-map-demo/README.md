# Demo orchestration

This directory will hold local and multi-machine demo scripts.

Phase 0 planned scripts:

- `run-local-three-agents.sh`: three local profiles with mocked adapters.
- `run-axl-three-nodes.sh`: three AXL-backed nodes.
- `record-demo.sh`: deterministic run for the submission video.

## Commands that currently work

```sh
cargo run -p chimiaclaw-cli -- demo-dag
cargo run -p chimiaclaw-cli -- demo-ord-adt
cargo run -p chimiaclaw-cli -- world-model
cargo run -p chimiaclaw-cli -- world-model verify
cargo run -p chimiaclaw-cli -- crucible-demo
cargo run -p chimiaclaw-cli -- science-market-demo
python3 -m http.server 8787 --directory demo
STORE=$(mktemp -d /tmp/chimiaclaw-store-XXXXXX)
cargo run -p chimiaclaw-cli -- node seed-ord --store-dir "$STORE"
cargo run -p chimiaclaw-cli -- node seed-route --store-dir "$STORE"
cargo run -p chimiaclaw-cli -- node run --store-dir "$STORE" --max-cycles 3 --interval-ms 0
cargo run -p chimiaclaw-cli -- artifact inspect --store-dir "$STORE"
```

Feature-gated live sponsor command surfaces:

```sh
cargo run -p chimiaclaw-cli --features live-sponsors -- live ens-verify --agent dft.service.chimiaclaw.eth --ens dft.service.chimiaclaw.eth
cargo run -p chimiaclaw-cli --features live-sponsors -- live zerog-anchor --source-artifact-json /tmp/source-artifact.json --payload-file /tmp/payload.json
cargo run -p chimiaclaw-cli --features live-sponsors -- live keeperhub-schedule --workflow-id wf_... --input-json '{"artifact_id":"art_demo"}'
cargo run -p chimiaclaw-cli --features live-sponsors -- live keeperhub-status --execution-id exec_...
```

See `../docs/speedrun/INTEGRATIONS.md` before running these: ENS needs
`ENS_RPC_URL`, 0G needs an operator-provided `ZEROG_UPLOAD_COMMAND` wrapper plus
testnet env, and KeeperHub needs `KEEPERHUB_API_KEY`.
`world-model` prints `demo/world-model.json`, a deterministic frontend fixture
for the abstract lab-swarm map: four real ChimiaDAO nodes, allied candidate
labs, virtual agent labs, unknown/quarantined labs, trust edges, lab-to-lab
interactions, quests, artifact cards, and backend command bindings. It is a UI
projection over the artifact DAG, not a live backend server.

`world-map.html` is the first dependency-free visual demo of that abstraction.
Serve this directory and open `http://localhost:8787/world-map.html`. The page
fetches `world-model.json` and renders the lab map, trust edges, quests,
artifact cards, science service transactions, implemented/planned agents, active
data/concept sharing, MSSP genealogy projection, Crucible review votes, and
World Avatar RDF projection. It marks the four real ChimiaDAO nodes explicitly
and keeps candidate, virtual, and quarantined endpoints visibly distinct. It
does not contact a live backend.

`science-market-demo` prints the current hackathon transaction spine:
ENS-shaped provider profile → service offer → request → quote → quote
acceptance → simulated escrow authorization → operator-confirmation-required
settlement intent → result → result acknowledgement → simulated release. It
includes one flow each for retrosynthesis, DFT, and literature. The payer is
`operator.chimiaclaw.eth`; the payees are the ENS-shaped service agents. The
amounts are deterministic USDC-micro fixtures, and the refund policy returns the
full simulated escrow to the payer for quote expiry, rejected result, provider
failure, or operator cancellation before execution. The sponsor fields are
explicit attachment points only: no live ENS resolution, AXL traffic, 0G write,
live Uniswap quote, KeeperHub schedule, or fund movement occurs in fixture mode.

`node run` is the current overnight-safe local loop. It polls the file-backed
artifact store, invokes registered deterministic demo skills, emits JSONL
cycle reports, and skips parents that already have a child from the same skill.
Without `--max-cycles`, stop it with Ctrl+C.

## Recording outline

1. Run validation quickly.
2. Run `demo-dag` and point out parent IDs.
3. Run `demo-ord-adt` and point out the `chem.ord.reaction` → `chem.adt.reaction` lineage.
4. Run `science-market-demo` and point out the three signed service transaction chains, including acceptance, simulated escrow, acknowledgement, release, and refund-policy artifacts.
5. Run `world-model verify` and point out that the DFT results and Crucible vote targets resolve to signed artifacts.
6. Open `world-map.html`; select Sofia, Iberia, Analysis Dock, Olympus, Flow Ally, Retro Swarm, and Unknown to show every lab participates in the interaction graph.
7. Point out the real-node badges and the dual channel counters: green is data payload movement, purple is conceptual MSSP / World Avatar sharing.
8. Point out the MSSP panel as an artifact genealogy projection, not a live optimizer.
9. Point out the World Avatar panel as an RDF projection, not the canonical store.
10. Run `node seed-ord`, `node seed-route`, and `node run --max-cycles 3`; point out that cycle 1 creates children and later cycles skip existing children.
11. Show the contract tests passing.
12. Explain the next safety gate and live sponsor integrations.
