# Live sponsor integrations

This note documents the first live ENS, 0G Storage, and KeeperHub surfaces. They are intentionally feature-gated so the default ChimiaClaw demo remains deterministic, offline, and reproducible.

## Compile with live sponsors enabled

```sh
cargo run -p chimiaclaw-cli --features live-sponsors -- live ens-verify --help
```

The default workspace build does not make live sponsor calls:

```sh
cargo run -p chimiaclaw-cli -- science-market-demo
```

## Security boundary

- Do not put private keys or API keys in source files, command arguments, docs, or commits.
- ENS is read-only and uses `ENS_RPC_URL`.
- KeeperHub uses bearer auth from `KEEPERHUB_API_KEY`.
- 0G uses `ZEROG_PRIVATE_KEY`, but the Rust CLI does not pass that key as a process argument. Instead, it calls an operator-provided `ZEROG_UPLOAD_COMMAND` wrapper that reads secrets from env and returns JSON.
- No Uniswap transfer or live token movement is implemented in this slice.
- Settlement remains non-custodial and artifact-first.

## ENS verification

Crate: `crates/chimiaclaw-identity-ens`

What it does:

- Resolves an ENS name through read-only Ethereum JSON-RPC.
- Fetches the resolver, address, and text records:
  - `chimiaclaw.profile.cid`
  - `chimiaclaw.capabilities`
  - `chimiaclaw.settlement.endpoint`
  - `chimiaclaw.axl.peer_id`
  - `chimiaclaw.head_artifact.cid`
- Compares live records against operator-provided expectations.
- Emits signed `identity.ens.resolution` and `identity.ens.verification` artifacts.

Required env:

```sh
export ENS_RPC_URL="https://YOUR_ETHEREUM_RPC"
```

Smoke command:

```sh
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live ens-verify \
  --agent dft.service.chimiaclaw.eth \
  --ens dft.service.chimiaclaw.eth \
  --expect-text chimiaclaw.profile.cid=zg://expected-profile-root
```

If the ENS name has no resolver or the text record differs, the command still returns signed artifacts; the verification report has `verified: false` and lists mismatches.

## 0G Storage upload anchor

Crate: `crates/chimiaclaw-storage-0g`

What it does:

- Upload execution is delegated to `ZEROG_UPLOAD_COMMAND`.
- The wrapper receives non-secret upload metadata on stdin as JSON.
- The wrapper reads `ZEROG_PRIVATE_KEY` from the environment.
- The wrapper returns JSON with `root_hash` or `root_hashes`, and optional `tx_hash` or `tx_hashes`.
- ChimiaClaw emits a signed `storage.zerog.upload` anchor artifact whose parent is the original artifact. The original artifact is never mutated.

Required env:

```sh
export ZEROG_UPLOAD_COMMAND="/absolute/path/to/your/zerog-upload-wrapper"
export ZEROG_PRIVATE_KEY="0x..."
export ZEROG_RPC_URL="https://evmrpc-testnet.0g.ai"
export ZEROG_INDEXER_URL="https://indexer-storage-testnet-turbo.0g.ai"
```

Wrapper stdin shape:

```json
{
  "file_path": "/tmp/payload.json",
  "network": "0g-galileo-turbo",
  "rpc_url": "https://evmrpc-testnet.0g.ai",
  "indexer_url": "https://indexer-storage-testnet-turbo.0g.ai",
  "private_key_env": "ZEROG_PRIVATE_KEY",
  "expected_replica": 1,
  "finality_required": false
}
```

Wrapper stdout shape:

```json
{
  "root_hash": "0x...",
  "tx_hash": "0x..."
}
```

Smoke command:

```sh
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live zerog-anchor \
  --source-artifact-json /tmp/source-artifact.json \
  --payload-file /tmp/payload.json \
  --agent storage.zerog.operator.chimiaclaw.eth
```

The output is a signed anchor artifact with `schema_tags: ["storage.zerog.upload"]`, `output_cid: "zg://<root>"`, and parent lineage back to the source artifact.

## KeeperHub scheduling

Crate: `crates/chimiaclaw-exec-keeperhub`

What it does:

- Uses `KEEPERHUB_API_KEY` against `KEEPERHUB_BASE_URL`.
- Starts a workflow execution through `POST /api/workflows/{workflow_id}/execute`.
- Polls an execution through `GET /api/executions/{execution_id}`.
- Emits signed `exec.keeperhub.scheduled`, `exec.keeperhub.completed`, or `exec.keeperhub.failed` artifacts.

Required env:

```sh
export KEEPERHUB_API_KEY="kh_..."
export KEEPERHUB_BASE_URL="https://app.keeperhub.com"
```

Schedule smoke command:

```sh
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live keeperhub-schedule \
  --workflow-id wf_... \
  --input-json '{"artifact_id":"art_demo","mode":"testnet-safe"}' \
  --parent-artifact-id art_demo
```

Status smoke command:

```sh
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live keeperhub-status \
  --execution-id exec_... \
  --scheduled-artifact-id art_keeperhub_scheduled
```

The command records returned execution status and any available transaction hash or explorer URL in the signed status artifact.

## Current implementation status

- ENS: direct read-only JSON-RPC implementation is in Rust and compiles under `--features live-sponsors`.
- 0G: Rust artifact anchor path is implemented; live upload uses an operator-provided wrapper so secrets stay in env.
- KeeperHub: REST client and signed execution artifacts are implemented and compile under `--features live-sponsors`.
- Default deterministic demos are unchanged.

## Validation commands

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo check --workspace --all-features
forge test --root contracts
```
