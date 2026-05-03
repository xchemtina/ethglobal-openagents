# Builder feedback — KeeperHub integration

Honest feedback from integrating KeeperHub into ChimiaClaw during ETHGlobal OpenAgents.

## What we built

ChimiaClaw uses KeeperHub as the execution scheduling layer for DFT (density functional theory) computations and settlement workflows. An agent submits a signed `chem.dft.request` artifact; KeeperHub schedules the PySCF worker execution; the result is signed and anchored. We implemented a Rust REST client against the KeeperHub API (`crates/chimiaclaw-exec-keeperhub`), a reference workflow (`demo/keeperhub/workflow.json`), and an operator runbook (`demo/keeperhub/README.md`).

## What worked well

- **The REST API is clean.** `POST /api/workflows/{id}/execute` and `GET /api/executions/{id}` are exactly the right abstraction for our use case. No overengineering.
- **The MCP server concept is strong.** Agent-native execution scheduling via MCP is the right direction for the ecosystem.
- **Bearer auth is simple and sufficient** for the hackathon context.

## UX / DX friction

- **No sandbox or testnet mode.** We couldn't safely test workflow scheduling without a live API key tied to real infrastructure. A sandbox environment with mock execution (returns canned results after a delay) would have let us develop and demo the integration end-to-end without needing production credentials during a hackathon.
- **Workflow registration is manual.** We had to describe the workflow JSON shape and then tell operators to register it through the UI. A CLI command like `keeperhub workflow create --from-json workflow.json` would have saved significant time and made the integration scriptable.
- **No webhook / callback on completion.** We poll `GET /api/executions/{id}` in a loop. A webhook callback URL in the execution request would eliminate polling and fit better with event-driven agent architectures.

## Documentation gaps

- **No Rust SDK or example.** The docs focus on TypeScript/Python. We wrote our own `reqwest`-based client from the API docs, which was fine but added time. A minimal Rust example in the docs would help Rust-native projects.
- **Workflow input schema is underspecified.** The docs describe `input_json` as a freeform object but don't document what fields the built-in workflow steps can consume. We had to guess at the shape and iterate.
- **Error responses are inconsistent.** Some endpoints return `{"error": "message"}`, others return `{"detail": "message"}`. Standardizing on one shape would simplify client error handling.

## Feature requests

1. **Sandbox/testnet mode** — the single most impactful addition for hackathon builders.
2. **`keeperhub workflow create` CLI command** — make workflow registration scriptable.
3. **Webhook callbacks on execution completion** — eliminate polling.
4. **Execution cost estimation** — before scheduling, tell the agent what it will cost. This is essential for the economic settlement model we're building.

## Summary

KeeperHub's core abstraction (reliable onchain execution with retry logic and audit trails) is exactly what scientific agent workloads need. The gap is developer ergonomics: a sandbox, a CLI for workflow management, and better Rust/multi-language support would make the integration significantly faster. We'd use KeeperHub in production if these gaps were closed.
