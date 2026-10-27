# PoX local federation adapter

This optional Python adapter connects the existing PoX inquiry workflow to the
OpenAgents signed-package CLI. It requires a separately installed compatible PoX
source tree; no PoX runtime, private records, credentials or upstream source code
are bundled here. The portable Rust exchange works without PoX installed.

The adapter:

1. Checks the five source files against `source-pins.json`, then loads verified
   temporary copies instead of ambient modules or bytecode caches.
2. Opens an **existing** SQLite v4 inquiry database read-only. It uses the real
   PoX authentication and accepted-handoff methods; it never calls the migrating
   `InquiryStore` constructor on that database.
3. Exports only an authenticated scope identifier, revision, SHA-256 and explicit
   mappings for three distinct humans. It exports no inquiry text or contacts.
4. Invokes the actual `chimiaclaw-federation verify` binary on private staged
   package/policy bytes, then rechecks the local accepted scope and membership.
5. Appends the full verification result to a separate owner-only SQLite sidecar.
   Signed reviews retain `reviewed` versus `corrections_required`; neither means
   scientific admission.

The sidecar records **historical local observations**. It creates no PoX evidence
link, download grant, delivery decision, publication approval or blockchain
transaction. In particular, it does not bypass PoX's DFT delivery exclusion.
Permission and remote revocation changes cannot be discovered from an old signed
package. Run verification against an explicitly refreshed local policy before
reuse. Even an exact retry rechecks the package, current PoX scope and membership.
Changes arriving after the recorded snapshot can invalidate its applicability.

## Local setup

Use the pinned `data-inquiry-20261007` source directory, the existing local inquiry
database, and a **named coordinator's** current credential in an owner-only file.
Administrator, holder and participant credentials cannot export coordinator
scope through this adapter. Never pass credential literals as command arguments.

Create an explicit local account mapping, using actual PoX staff IDs and the
federation human IDs whose public keys were confirmed separately:

```json
{
  "schema": "pox.federation-members.v1",
  "source_instance": "my-private-pox",
  "members": [
    {"pox_staff_id": "staff_111111111111111111111111", "federation_id": "alice"},
    {"pox_staff_id": "staff_222222222222222222222222", "federation_id": "bob"},
    {"pox_staff_id": "staff_333333333333333333333333", "federation_id": "carol"}
  ]
}
```

The IDs above are placeholders. AI identities are delegated in the federation
policy; they are not substitute PoX human accounts. A new PoX handoff produces a
different `scope_id`, even if its scope text/hash happens to remain unchanged.

From the repository root, set local paths and run:

```sh
python3 -B integrations/pox/bridge.py \
  --pox-source "$POX_SOURCE" --database "$POX_DATABASE" \
  --members "$POX_MEMBERS" --credential-file "$POX_COORDINATOR_CREDENTIAL_FILE" \
  --inquiry "$POX_INQUIRY_ID" \
  scope --out ./private-scope.json --receipt-out ./private-scope-receipt.json
```

Put the scope JSON into `policy.current_scope` and configure the trusted human/AI
keys, expected PoSyn root, policy lifetime and locally known revocations. Treat
the exported scope as a local input, not an independently authenticated remote
authority. The root federation guide describes key exchange and package signing.

Record a received, cryptographically verified package:

```sh
python3 -B integrations/pox/bridge.py \
  --pox-source "$POX_SOURCE" --database "$POX_DATABASE" \
  --members "$POX_MEMBERS" --credential-file "$POX_COORDINATOR_CREDENTIAL_FILE" \
  --inquiry "$POX_INQUIRY_ID" \
  record --federation-bin ./target/debug/chimiaclaw-federation \
  --package ./private-evidence.json --policy ./private-policy.json \
  --sidecar ./private-federation-references.sqlite3
```

For a review, use `--package ./private-review.json` and additionally pass
`--evidence ./private-evidence.json`. The exact evidence is verified again.
The sidecar stores digests, scope identities and verification receipts, not the
bundle's scientific payload or credentials. Its records are private too.

Outputs use exclusive creation and mode `0600`; existing output files are not
overwritten. Packages are signed **but not encrypted**. Transfer private packages
through an access-controlled encrypted channel. This adapter performs no transfer.
Source compatibility failures are explicit: qualify a new version and update its
pins deliberately rather than bypassing the check. The adapter is a local CLI,
not a daemon or concurrent source-module loader. JSON policies are limited to
1 MiB and package inputs to 48 MiB, matching the Rust exchange limit.

## Reproduce checks

Pure tests run anywhere with Python 3.10+; real PoX tests require its installed
source. Without `POX_SOURCE`, those five tests explicitly report skips.

```sh
python3 -B -m unittest discover -s integrations/pox/tests -v
POX_SOURCE=/path/to/PoX/implementation/data-inquiry-20261007 \
  python3 -B -m unittest discover -s integrations/pox/tests -v
```

The integration demo uses the actual pinned PoX `InquiryStore`, three synthetic
human accounts, six freshly generated Rust identities, and temporary databases.
It never reads or changes an existing inquiry database:

```sh
cargo build --locked -p chimiaclaw-federation
python3 -B integrations/pox/demo.py \
  --pox-source "$POX_SOURCE" \
  --federation-bin ./target/debug/chimiaclaw-federation
```

The demo checks real signed evidence/review import, retry idempotency, revoked AI
and human keys, AI/wrong-human review rejection, changed account mapping,
replacement handoffs and stale scope. No network, payment or scientific execution
occurs. `--receipt-out NEW_FILE` optionally writes a non-secret synthetic receipt.
