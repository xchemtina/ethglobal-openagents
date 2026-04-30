# chimiaclaw-zerog-uploader
A uv-managed wrapper that satisfies the `ZEROG_UPLOAD_COMMAND` contract used by
`crates/chimiaclaw-storage-0g`. The Rust crate signs a `storage.zerog.upload`
anchor artifact whose payload binds the upload's root hash + tx hash; this
wrapper is the privacy-respecting boundary that actually moves the bytes.
## Contract
- Reads a JSON metadata document on stdin with the shape produced by
  `chimiaclaw_storage_0g::ZeroGUploadRequest`:
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
- Reads the private key from the env var named in `private_key_env` (default
  `ZEROG_PRIVATE_KEY`); never accepts a key on argv or stdin.
- Shells out to `${ZEROG_BINARY:-0g-storage-client}` with appropriate flags
  to upload the file to the indexer, captures the binary's stdout, and parses
  out the merkle root hash and the EVM tx hash.
- Emits a JSON document on stdout matching `ZeroGUploadReceipt`:
  ```json
  {
    "network": "0g-galileo-turbo",
    "indexer_url": "https://indexer-storage-testnet-turbo.0g.ai",
    "root_hashes": ["0x..."],
    "tx_hashes": ["0x..."],
    "uploaded_at_unix": 1730000000,
    "audit_notes": ["..."]
  }
  ```
- On failure, exits non-zero with a clear stderr message.
## Wiring
```sh
export ZEROG_UPLOAD_COMMAND="uv run --project skills/scienceclaw-port/workers/storage-0g zerog-upload"
export ZEROG_PRIVATE_KEY="0xYOUR_TESTNET_KEY"
export ZEROG_BINARY="${HOME}/bin/0g-storage-client"  # Optional override
```
The 0G binary is operator-installed (the official Go CLI from `0glabs/0g-storage-client`).
Per repo policy we never wrap that install in Docker or Homebrew; download the
prebuilt release or `cargo install` an alternative client.
## Stub mode
For CI and local-dev runs without a real 0G key, the wrapper accepts
`ZEROG_STUB=1` which short-circuits the upload, hashes the file content
locally, and emits a deterministic stub receipt. Stub receipts carry an
explicit `audit_notes` entry flagging that the upload was simulated, so the
signed artifact never silently impersonates a real on-chain event.
