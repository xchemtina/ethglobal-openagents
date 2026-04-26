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
```

## Recording outline

1. Run validation quickly.
2. Run `demo-dag` and point out parent IDs.
3. Run `demo-ord-adt` and point out the `chem.ord.reaction` → `chem.adt.reaction` lineage.
4. Show the contract tests passing.
5. Explain the next safety gate and governance anchor.
