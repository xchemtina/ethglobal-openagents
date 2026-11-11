# Offline federation pilot

Exchange PoSyn evidence and human review between separate local installations.
The public CLI is `chimiaclaw-federation`; run `--help` for exact arguments.
No server, blockchain, token, live computation or shared secret is required.

Each participant generates a local Ed25519 key with `init`. It uses the operating
system random source and writes a new mode-0600 key file. The command prints only
the public identity. Keep human keys separate from AI keys; do not give an AI a
human key. Filesystem access to a human key grants its signing authority.

## Trust and scope

Recipients supply their own `openagents.federation.policy.v1` document. Confirm
human and AI public keys and the expected PoSyn root using a separate trusted
channel; an identity or root attached to the received package is not a trust pin.
Never replace local policy automatically with a sender-supplied policy.

The policy binds one current scope and handoff: opaque scope ID, positive revision,
algorithm-qualified SHA-256 scope digest, and three distinct human identities for
coordinator, producer and reviewer. The PoX adapter includes the exact inquiry and
accepted handoff IDs in the scope ID. Replacing a handoff invalidates old packages
even when the written scope text has not changed.

Policies explicitly name their observation and expiry times. Verification reports
`trust_state=local_snapshot` and hashes the exact policy-file bytes used. Revoked
keys and delegated authority are checked every time. Offline software cannot know
about revocations, replacement scopes or new policies it has not received. Update
policies and confirm current source applicability before relying on a package.
Local clocks and policy files are trusted administrator input, not a distributed
consensus or anti-rollback mechanism.

## Statements

- **Evidence:** the producer signs a portable PoSyn bundle. Verification rechecks
  the original manifest bytes, complete object inventory, hashes and conservative
  integrity claims against the recipient's independently pinned root.
- **Delegation:** the human producer may authorize one pinned AI key to produce
  evidence for this exact scope until a bounded expiry. The signed delegation is
  embedded in AI-produced evidence and can be revoked by its full content hash.
  It does not grant review, publication, payment or execution permission.
- **Review:** only the named human reviewer may sign `reviewed` or
  `corrections_required`. The statement binds the exact evidence content hash,
  artifact ID and scope. Verification requires that original evidence package
  and preserves the decision in its receipt. Both decisions retain
  `scientific_review=not_established` and `publication_authorized=false`.

Every statement is an existing OpenAgents signed artifact with a strictly typed
inline payload. Both the signature and payload digest are verified. Package and
policy JSON reject duplicate or unknown fields and excessive structure. Source
digest algorithms remain explicit: PoSyn uses BLAKE3; PoX scope and local policy
document identities use SHA-256. Hashes are never substituted for one another.

## Files and disclosure

Packages are **signed, not encrypted**. Mode-0600 output protects local filesystem
access but does not encrypt transmitted copies. Use a private encrypted transfer
channel or encrypt the file separately before exchange. Existing copies cannot
be recalled by revocation. `private` and `synthetic` are the only audience labels;
the synthetic label is the producer's declaration, not a content classifier.
There is no public export command, upload, automatic scientific admission or
PoX delivery permission in this pilot.

The CLI never overwrites existing outputs and rejects symlink input leaves. New
keys and package outputs currently require Unix filesystem permission semantics
(macOS/Linux). Use an owner-controlled directory: this pilot does not defend
against an attacker who controls your parent directories, trust policy, clock or
local process. Input/output paths and secret material are omitted from errors.

## Verification

```sh
cargo test --locked -p chimiaclaw-federation
cargo clippy --locked -p chimiaclaw-federation --no-deps --all-targets -- -D warnings
```

The tests cover three human identities and their distinct AI keys, valid exchange,
human review, correction outcomes, wrong-key impersonation, tampering after
resigning, separately pinned roots, scope/handoff changes, revocation, expiry,
delegation isolation, unknown/duplicate fields and private-file handling.

## Prospective terminal-attempt binding

The separate `openagents.terminal-attempt.package.v1` profile joins one **new**
terminal attempt to its exact retained observer event, raw receipt and original
PoSyn evidence package. It supports success, worker failure, timeout and
cancellation without converting a failed attempt into a successful result.
The existing exchange/review v1 profiles are unchanged. Use the new commands for
this profile; the old `verify` command intentionally does not accept it.

```sh
chimiaclaw-federation bind-attempt \
  --policy recipient-policy.json --expect-attempt held-expectation.json \
  --evidence original-evidence.json --receipt terminal-receipt.json \
  --event retained-observation.json --key local-producer-ai.key \
  --out new-signed-attempt.json

chimiaclaw-federation verify-attempt \
  --policy recipient-policy.json --expect-attempt held-expectation.json \
  --evidence original-evidence.json --receipt terminal-receipt.json \
  --event retained-observation.json --package new-signed-attempt.json
```

The original evidence signer signs this additional statement. A delegated AI uses
its existing bounded `produce_evidence` authority, which grants evidence signing
only. Verification rechecks current trust, scope, root, revocations and delegation
expiry. It validates the original evidence package again, including every PoSyn
object. The new statement signs the full original evidence content hash and
artifact ID, exact SHA-256 digests of the retained event/receipt and the recipient
expectation, and the typed terminal receipt. No file named within a record is
opened and no command is executed.

The receiver independently holds an `openagents.terminal-attempt.expectation.v1`
record containing:

- `job_id`, `attempt_id` and the SHA-256 identities of the authorization record,
  input manifest and environment record;
- exact `event_sha256` and `receipt_sha256` byte commitments;
- the expected result names and a finite `not_before_unix` / `expires_at_unix`
  window, no later than the local policy expiry.

Obtain these pins from the agreed work-order and accepted handoff channel. A
sender-supplied adjacent expectation cannot authenticate itself. Hashing an
authorization reference does not validate the authority or content of that
work order: the returned `authorization_reference_verified` and
`execution_authorized` remain **false**. Input/environment identities bind what
the signer reports, not proof that a particular runtime was actually used.

The raw `openagents.terminal-attempt.receipt.v1` record carries the exact scope,
job/attempt and manifest/authorization identities, start/end times, outcome,
optional typed failure code, output identities and missing-output names. Successful
receipts require at least one result and no missing outputs. Failed receipts retain
typed reasons (`worker_failed`, `missing_output`, `invalid_output`, `task_timeout`
or `cancelled`) and any partial results. Present and missing names must partition
the expected inventory without duplicates. Present result names, BLAKE3 digests
and sizes must exactly equal **all result-role artifacts** in the verified PoSyn
manifest, with one explicit alternative for zero produced outputs: PoSyn v1 still
requires a result role, which may retain a typed
`openagents.terminal-attempt.failure-record.v1` report. In that case the receipt
identifies it separately in `retained_failure_record`, its schema/job/attempt/
outcome/failure/missing inventory must match, and it cannot appear among expected
or produced outputs. For normal/partial-result receipts this field is `null`.
The underlying PoSyn v1 contract currently permits exactly one result-role
artifact. This profile therefore supports one produced result payload (which may
itself be an archive or manifest) or one retained failure report; it does not yet
support several independent result-role artifacts. Additional expected names may
be recorded as missing in a failed or timed-out attempt.
The required scientific output remains missing; a failure report is not counted
as that output. The reserved failure-record schema is also refused among ordinary
outputs, even after changing the held expectation. Inventing an empty successful
result is refused.

This profile accepts the existing `openagents.workstream.observation.v1` event:
`workstream_id` must equal `job_id`, `source_session` must equal `attempt_id`, and
`receipt_sha256` must match the exact receipt. The current observer's unchanged
unsigned/unverified assurance fields are required. Export the canonical observation
payload, whose SHA-256 is its event ID, rather than a mutable dashboard projection.
Other event schemas require explicit future adapters. This typed linkage does not
authenticate the original observer's operator-reported source identity or grant
authority to `next_step_proposed` text.

The verifier is read-only: repeated verification preserves the package content
hash. It refuses the same package under a different expected attempt. A receiving
workflow can retain one verification receipt and submit those same bytes to the
observer repeatedly for its existing transactional deduplication. Reissuing a
verification at a different timestamp creates different receipt bytes; this CLI
does not maintain an anti-replay database or an execution scheduler.

Timestamps are sender/local-clock claims, not trusted time attestation. The terminal
receipt must fit the held window and precede the signed evidence; the attempt
signature must follow that evidence. This is a prospective integration capability,
not a way to backdate or repair missing historical provenance. New reviews or
bindings of old evidence must state their actual new creation time and limits.

Verification retains `execution: not_verified`, `scientific_review: not_established`,
`human_awareness: unknown` and `publication_authorized: false`. These signed
packages are **not encrypted**. Human-only acknowledgment, a next-action grant,
hardware signing, transport and genuine three-person participation remain separate
work.

Run the complete synthetic attempt/observer journey:

```sh
cargo build --locked -p chimiaclaw-federation
PYTHONDONTWRITEBYTECODE=1 python3 -B demo/federation/attempt.py
```

The demo uses fresh temporary keys and actual observer journals in two local
directories. It binds a success, a timeout with partial outputs, and a zero-output
failure with a separately retained failure report. It verifies them
as the receiver, reopens/retries observer delivery, and rejects signature changes,
attempt substitution and output overwrite. It performs no scientific calculation,
network transfer, human acknowledgment or authorized execution. Its optional
`--receipt NEW_FILE` records only synthetic check results and public hashes.

Inputs are bounded to 1 MiB for the expectation, receipt and event; the signed
attempt container is bounded to 4 MiB. Existing PoSyn limits still apply. The
portable result-name profile uses bounded identifiers, not arbitrary filesystem
paths. Rust types in `src/attempt.rs` are the exact field/schema contract.
