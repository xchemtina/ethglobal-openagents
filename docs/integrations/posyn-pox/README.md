# PoSyn / OpenAgents / PoX: local evidence exchange

This pilot connects PoSyn byte-integrity bundles, OpenAgents signed artifacts,
and PoX's accepted inquiry scopes. Three or more people can run independent local
installations and exchange signed evidence and review packages. Each human and AI
has a different key. There is no shared service, blockchain transaction, payment,
scientific execution, or automatic publication in this integration.

The runnable synthetic journey is [demo/federation/run.py](../../../demo/federation/run.py).
The real PoX adapter is [integrations/pox](../../../integrations/pox).
The delivery assessment has an [editable LaTeX source](REVIEW.tex), a
[PDF companion](REVIEW.pdf), and machine-readable [validation](VALIDATION.json).

## Responsibilities

| Component | Owns | Does not establish |
| --- | --- | --- |
| PoSyn | Original bundle manifest, object bytes, domain-separated BLAKE3 root and claim limits | Actual execution, reproducibility, physical synthesis or chemical identity |
| OpenAgents | Signature validation, independently pinned identities, bounded AI delegation, scope and exact evidence/review binding | Real-world identity, scientific correctness or global revocation freshness |
| PoX | Accepted local inquiry/handoff, human role mapping, current scope checks and a separate private exchange-reference log | A source download grant, customer acceptance, scientific admission or publication permission |

PoSyn's original files and claims remain intact inside the portable package. An
unsupported or rejected molecule stays unsupported or rejected. The bridge does
not translate native MolADT records into the smaller demonstration molecule model.
PoSyn BLAKE3 roots, OpenAgents artifact content hashes and PoX SHA-256 identities
are separate, explicitly labelled values.

## Try the three-person journey

From the repository root, with Rust and Python 3 available:

```sh
cargo build --locked -p chimiaclaw-federation
python3 demo/federation/run.py
```

The script creates three isolated directories, six random keys (three human and
three AI), and separate local trust policies. The producer delegates evidence
signing to their AI; the reviewer verifies the received package and signs a human
review; the coordinator checks both against their own policy. It also exercises
tampered signatures, untrusted/revoked keys, expired policy, changed scope/handoff,
wrong evidence references and attempted AI/human role substitution. All private
working state is temporary and removed when the script exits.

This is a same-host simulation of independent installations. It does not prove
network availability, independent-device installation or real-world identity.
The fixture is entirely synthetic and includes accepted, unsupported, rejected
and missing-identifier records. No private source pool is distributed.

## Set up real local identities

Each person runs `init` on their own machine. Never send another person your key
file and never commit it to Git. Generated keys and packages are owner-readable
only on supported Unix systems; outputs cannot overwrite existing files.

```sh
target/debug/chimiaclaw-federation init \
  --id your-human-id --kind human --key /private/local/path/human.key
target/debug/chimiaclaw-federation init \
  --id your-ai-id --kind ai --key /private/local/path/ai.key
```

The command prints only the public identity record. Exchange those public records
and check the complete public-key fingerprints through a separately trusted
channel. Names and keys embedded in received packages do not add themselves to
the recipient's trusted identities.

Each installation maintains its own policy file: current scope and handoff,
coordinator/producer/reviewer identities, trusted public keys, revoked keys and
delegations, a separately obtained expected PoSyn root, and observation/expiry
times. The three named human roles must be different. A sample policy is created
by the demo from fresh synthetic keys; it is not a production trust anchor.

For PoX work, export the exact accepted scope with the source adapter instead of
inventing its digest. Map existing PoX human accounts explicitly to federation
IDs; verify the associated public keys separately. A replacement handoff, scope
revision or deactivated participant must invalidate the older applicability.

The software distinguishes **human-designated keys** from AI-designated keys;
it cannot prove a human was present. A local AI with the same operating-system
permissions could read a human's plain key file. Keep human review keys in an
account or signing environment the AI cannot access. Hardware/user-presence
signing is not implemented. The demo tests policy roles, not an OS sandbox.

## Exchange and review

The CLI commands are `init`, `delegate`, `import-posyn`, `verify` and `review`.
Use `--key` for a local secret file, `--policy` for the recipient's local policy,
and `--out` for a new package file. `import-posyn` requires `--bundle` and an
independently held `--expect` root. An AI producer also supplies its human-signed
`--delegation`. Review verification requires the exact original `--evidence`.

Only the human producer can delegate the evidence-signing role, and only for the
current scope and a bounded lifetime. Delegation does not let an AI approve a
human-designated review or act as another participant. A `reviewed` decision
records a statement signed by that person's designated key;
`scientific_review` remains `not_established`. A `corrections_required` decision
is retained as such in verification and PoX receipts.

Packages are **signed, not encrypted**. Transfer private packages through an
appropriately private channel. The tool neither creates a public download nor
removes private metadata. A recipient who receives bytes can keep a copy;
revocation cannot erase a previously delivered file.

Offline verification reports `trust_state: local_snapshot`. It checks the policy
currently available on that installation, including its expiry and revocations.
It cannot discover a remote revocation that has not been received. Distribute
updated trust/scope information through your established trusted channel before
making a consequential review decision.

## Use the actual PoX inquiry state

The additive Python adapter requires an explicitly selected local PoX source
installation matching [source-pins.json](../../../integrations/pox/source-pins.json).
It imports only a verified temporary source copy, reads the existing inquiry DB
without running migrations, and authenticates using existing PoX staff logic.
Credentials are read from a private file and never stored in exchange receipts.

The adapter records verified evidence/review references in a separate SQLite
sidecar. It invokes the actual Rust verifier, checks the accepted handoff before
and after verification, and records the precise local snapshot used. Historical
references do not become download capabilities. Existing PoX delivery restrictions,
including the DFT exclusion, remain applicable.

The private source implementation and databases are not included in this public
repository. See the adapter's README for its pinned-runtime test and synthetic
end-to-end command. A missing or changed source installation is an explicit
compatibility error, not permission to bypass the PoX check.

## Boundaries for this pilot

- No new smart contract is required or deployed. The seven existing Solidity
  scaffolds are not part of this exchange path.
- Bundle profile limits are 128 artifact entries, 1 MiB per object, 8 MiB total
  artifact bytes, and 256 KiB manifest. Keep working directories locally owned
  and stable; do not treat a hostile shared filesystem as trusted storage.
- The PoSyn upstream check qualifies wire compatibility and byte integrity only.
  It does not run native molecular conversion, chemistry or laboratory work.
- Operator-owned local policies establish trust. Automatic identity discovery,
  multi-device key recovery, encrypted package formats and revocation distribution
  are future work.
- This delivery adds a CLI and local adapter. It does not add these workflows to
  the web storefront or expose a network API.
