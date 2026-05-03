# KeeperHub round-trip
This directory holds the reference workflow definition (`workflow.json`) and a
small runbook for exercising the existing `live keeperhub-schedule` /
`live keeperhub-status` CLI surfaces against a real KeeperHub account.
The Rust adapter (`crates/chimiaclaw-exec-keeperhub`) already implements the
REST client; this runbook documents the operator steps so the smoke is
reproducible.
## Prerequisites
1. A KeeperHub account at https://app.keeperhub.com.
2. A workflow registered in the UI matching `workflow.json` (or any
   manual-trigger workflow that accepts `artifact_id` as a string input).
3. An API key issued for that account.
## Environment
```sh
export KEEPERHUB_API_KEY="kh_..."
export KEEPERHUB_BASE_URL="https://app.keeperhub.com"
```
## Schedule
```sh
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live keeperhub-schedule \
  --workflow-id wf_REPLACE_WITH_REAL_ID \
  --input-json '{"artifact_id":"art_c6fb4314b4dc7ac7","mode":"testnet-safe"}' \
  --parent-artifact-id art_c6fb4314b4dc7ac7 \
  > demo/keeperhub/scheduled.json
```
The output is a signed `exec.keeperhub.scheduled` artifact whose payload
records the workflow id, execution id, raw KeeperHub response, and the agent
that scheduled it. Save it next to `workflow.json` for the demo trail.
## Poll
```sh
EXEC_ID=$(python3 -c "import json; print(json.load(open('demo/keeperhub/scheduled.json'))['scheduled']['execution_id'])")
SCHED_ART=$(python3 -c "import json; print(json.load(open('demo/keeperhub/scheduled.json'))['artifact']['id'])")
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live keeperhub-status \
  --execution-id "${EXEC_ID}" \
  --scheduled-artifact-id "${SCHED_ART}" \
  > demo/keeperhub/status.json
```
The status artifact is tagged `exec.keeperhub.scheduled`,
`exec.keeperhub.completed`, or `exec.keeperhub.failed` depending on the
returned state, and parents the schedule artifact so the run is auditable
end-to-end.
## DFT angle
For the prize-track demo the recommended pattern is:
1. Build a `chem.dft.request` artifact via `chimiaclaw-cli moladt-dft-demo`.
2. Schedule a KeeperHub workflow with `artifact_id` set to that DFT request id.
3. Anchor the resulting status artifact through 0G via
   `live zerog-anchor` so the chain of custody (DFT request -> scheduled -> on-chain
   receipt -> 0G anchor) is fully signed.
The `workflow.json` here is intentionally cheap: it logs the input and emits a
zero-value transaction. Once a real Skala/PySCF DFT worker is wired up on
`duck@olympus.local` we can swap the workflow body without changing any
Rust/CLI code.
