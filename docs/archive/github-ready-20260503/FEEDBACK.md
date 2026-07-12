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

---

# Builder feedback — Uniswap Trade API integration

Honest feedback from integrating the Uniswap Trade API into ChimiaClaw during ETHGlobal OpenAgents.

## What we built

ChimiaClaw uses the Uniswap Trade API as the settlement quoting layer for science service transactions. An autonomous agent produces a signed `market.settlement.intent` artifact, calls `POST /v1/quote` through the Rust adapter (`crates/chimiaclaw-settle-uniswap`), and seals the full quote response into a signed `market.uniswap.quote` artifact. The agent never calls `/swap` without explicit operator confirmation — the quote artifact is an auditable record that downstream operators or adapters inspect before any token movement.

The retrosynthesis service flow in `chimiaclaw-market` now uses `UniswapPreparedTransfer` as its settlement method with a real Uniswap API route hint, while DFT and literature flows remain on the simulated artifact ledger for comparison.

## What worked well

- **The REST API is exactly right for agentic integration.** `POST /quote` with JSON in, JSON out is the ideal abstraction for a Rust agent. No SDK dependency, no Node.js runtime, no WebSocket state — just `reqwest` and `serde`. This is the correct design for backend/agent consumers.
- **The 3-step flow (check_approval → quote → swap) is clean and honest.** Each step is independently auditable, which maps perfectly to our signed artifact model: one artifact per step.
- **CLASSIC routing with `protocols: ["V2","V3","V4"]` avoids the UniswapX minimum trade size.** This is important for scientific micro-services where the settlement amount may be small (e.g. $3.10 USDC for a retrosynthesis route quote).
- **Free API keys with immediate provisioning.** We got a key from the developer platform and were making live quote requests within minutes.
- **The `x-universal-router-version` header is a good versioning pattern.** It makes the API surface evolution explicit without breaking existing integrations.
- **Quote responses include `gasFeeUSD` as a string.** This is the right abstraction — agents should not be estimating gas costs from wei and hardcoded ETH prices.

## UX / DX friction

- **No Rust SDK or example.** The docs and skills focus on TypeScript (viem/ethers) and browser frontends. We wrote our own `reqwest`-based client from the API reference, which was straightforward but added time. A minimal Rust example in the docs would help the growing Rust crypto ecosystem.
- **The `permitData` handling differs by routing type, and this is not obvious at first glance.** CLASSIC routes need `permitData` in the `/swap` body; UniswapX routes reject it. The skill doc covers this well, but the API reference page buries the distinction. A routing-type-specific response schema would prevent mistakes.
- **Quote response shape varies by routing type with no discriminated union at the API level.** CLASSIC returns `quote.output.amount`; UniswapX returns `quote.orderInfo.outputs[0].startAmount`. The Uniswap skill doc recommends a TypeScript discriminated union on `routing`, but this is something the API itself should enforce — returning a consistent `outputAmount` field across routing types would eliminate an entire class of bugs.
- **`slippageTolerance` vs `autoSlippage` mutual exclusivity is underspecified.** The error message when both are set is not helpful. A clear validation error would save debugging time.
- **No testnet quoting.** We could quote against mainnet pools (read-only), but there is no sandbox mode for testing the full quote → swap flow without real funds. A testnet-aware quote endpoint (even with synthetic prices) would let hackathon builders demo the full pipeline safely.

## Feature requests

1. **Consistent `outputAmount` field across routing types** — the single most impactful DX improvement. Agents should not need routing-type-specific output extraction logic.
2. **Testnet/sandbox quote endpoint** — let builders demo the full flow without mainnet funds.
3. **Rust SDK or example** — even a minimal `reqwest` example in the docs would help.
4. **Batch quoting** — for agents that need to quote multiple settlement legs simultaneously (e.g. multi-step retrosynthesis with per-step settlement), a batch `/quote` endpoint would reduce latency and rate-limit pressure.
5. **Quote validity metadata** — include an explicit `expiresAt` field in the quote response so agents know when to re-quote without guessing.

## Summary

The Uniswap Trade API's REST design is excellent for agentic finance. It's the right level of abstraction: agents call an HTTP endpoint, get a structured quote, and decide whether to execute. The 3-step flow maps directly to our signed artifact model. The main gaps are response shape inconsistency across routing types, missing Rust ecosystem support, and the lack of a testnet sandbox. For scientific agent settlement specifically, the API is already production-ready — we just need the ecosystem tooling to catch up.
