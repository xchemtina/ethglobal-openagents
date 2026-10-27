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
