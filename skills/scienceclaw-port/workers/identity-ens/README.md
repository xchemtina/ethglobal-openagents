# chimiaclaw-ens-publisher
A uv-managed worker that publishes ChimiaClaw text records on a controller-owned
ENS name. The Rust side (`chimiaclaw-identity-ens`) does read-only resolution
and verification; this worker is its write-side companion.
## Contract
- Reads the ENS name from `--ens` and one or more text records from
  `--record key=value` (repeatable).
- Reads the RPC URL from `--rpc-url` or `ENS_WRITE_RPC_URL`; the private key
  always comes from `ENS_WRITE_PRIVATE_KEY` (never argv) so it never appears
  in process listings or shell history.
- Refuses to run if the configured account is not the registry owner of the
  name, and prints the existing owner so the operator can fix the situation
  manually.
- Skips records whose value already matches what the resolver returns, so
  re-running the publisher is cheap and idempotent.
- Emits a JSON document with one entry per intended record, each carrying
  `tx_hash`, `block_number`, `gas_used`, and a `status` of either
  `"published"`, `"unchanged"`, or `"failed"`.
## Wiring
```sh
export ENS_WRITE_RPC_URL="https://YOUR_SEPOLIA_RPC"
export ENS_WRITE_PRIVATE_KEY="0xYOUR_TESTNET_KEY_WITH_GAS"
uvx --from skills/scienceclaw-port/workers/identity-ens ens-publish-text-records \
  --ens dft.service.chimiadao.eth \
  --record chimiaclaw.profile.cid=zg://demo-profile-root \
  --record chimiaclaw.axl.peer_id=axl-dft-demo-peer \
  --record chimiaclaw.head_artifact.cid=zg://demo-head-root
```
The Rust adapter `chimiaclaw_identity_ens::EnsPublisher::from_env()` runs this
worker via `CHIMIACLAW_ENS_PUBLISH_COMMAND`, signs the response as a
`identity.ens.publication` artifact, and (with `--verify`) chains directly
into the existing `LiveEnsResolver` + `verify_resolution` path so the operator
gets three signed artifacts in a single CLI call:
1. `identity.ens.publication` (one entry per text record we tried to set);
2. `identity.ens.resolution` (the live read-back);
3. `identity.ens.verification` (`verified: true` once the publish lands).
## Status
Per repo policy this worker runs under uv/uvx, never under Docker. Sepolia and
Holesky testnets are the recommended targets; mainnet writes should require
explicit operator confirmation outside this script.
