# Zopf discovery dossier v1

Lifecycle: **testing / report-only**
Implementation commit: `f215747992409d6359a62d450ca6a4c3c6e7095a`
Zopf implementation commit: `3ad9ef221c129f2afb4bb5da42e4f7a2d71b8f12`
Olympus replay: `/Users/duck/ChimiaDAO/OpenAgents-wt-zopf-sciagents` clean at the exact Rust commit

`chimiaclaw-discovery` is the Rust/OpenAgents producer and signed-integrity verifier for one
bounded `science.discovery.dossier.v1`. A separate Zopf Babashka/JVM terminal validates the
exported dossier and emits `zopf.discovery-dossier-report.v1`. This bridge transports evidence for
human review. It provides no database, dispatch, execution, publication, mutation, or
human-decision authority.

## Data flow and ownership

```text
typed Rust dossier
  -> strict validation and canonical JSON
  -> inline BLAKE3 payload
  -> existing OpenAgents Ed25519 Artifact seal
  -> Rust signed-integrity and exact metadata/payload verification
  -> canonical dossier export
  -> independent Zopf structural/evidence validation
  -> normalized SHA-256 + counts + origin-unverified warning
  -> human review
```

OpenAgents owns typed construction, canonical export, payload binding, Artifact signed integrity,
and exact parent/topic/agent/fingerprint/timestamp/schema metadata binding. The Artifact carries its
own verification key, so v1 proves self-consistent signed integrity rather than trusted origin.
Parent artifact IDs are signature-bound references but are not recursively resolved.

Zopf owns an independent bounded review of the exported JSON. It does not parse or trust the
Artifact envelope, and it always reports `origin_authenticity=not-verified-by-zopf`. Agreement
between implementations establishes wire compatibility, not scientific truth.

## Contract and shared fixtures

The exact dossier top-level keys are:

```text
schema dossier_id title research_question summary
sources claims hypotheses proposed_tests conflicts open_questions
provenance governance
```

The closed schema requires evidence-bearing support links, claim-referenced falsifiable
hypotheses, at least one proposed test per hypothesis, explicit conflict records, bounded
provenance, and immutable pending-decision governance with every machine authority bit false.

The byte-identical fixture is
[`crates/chimiaclaw-discovery/fixtures/discovery-dossier.valid.json`](../crates/chimiaclaw-discovery/fixtures/discovery-dossier.valid.json):

- raw SHA-256: `836778ca0675892bfd74a90e4b2e06b7133f1081340402de8b5ec0142787bca6`;
- normalized dossier SHA-256:
  `c52bd4a48717fde7340f4a3545a85b4013f85c60d8a3dd1d5fef30e7cb57f067`.

The shared string, HTTPS-locator, and UTF-16-ordering corpus is
[`crates/chimiaclaw-discovery/fixtures/discovery-string-safety-parity.v1.json`](../crates/chimiaclaw-discovery/fixtures/discovery-string-safety-parity.v1.json),
raw SHA-256 `646ae7a82592a40d77dfd34cc327bce2602403206b1a33de4719314825b4786c`.
It freezes an explicit Unicode White_Space blank predicate and an ASCII-only
keyword, case-folding, and separator grammar for path/secret screening, so
Rust and JVM regex-engine defaults do not define the wire contract.

The deterministic signed test Artifact is
[`crates/chimiaclaw-discovery/fixtures/discovery-artifact.valid.json`](../crates/chimiaclaw-discovery/fixtures/discovery-artifact.valid.json):

- size: 5,285 bytes;
- raw SHA-256: `9fd126bbcdcd4e5e2cd2a04c3c52a2a64cfa90f7ff16f5957adfcc90c46c9310`;
- signing identity: public test seed `[23; 32]`, never a trusted origin.

The signed Artifact's inline payload must already be the canonical dossier bytes. Verification
rejects a signed but noncanonical payload; it does not silently reserialize that payload into
compliance. The terminal CLI exports those exact canonical payload bytes plus one framing newline.
The resulting dossier was accepted by Zopf with normalized SHA-256
`c52bd4a48717fde7340f4a3545a85b4013f85c60d8a3dd1d5fef30e7cb57f067`, every authority bit
false, and origin authenticity unverified. The checked Zopf report is
`reports/openagents-rust-sciagents-discovery-replay-20260813.json` in the Zopf repository, raw
SHA-256 `c615d6c58b3a4c278feab7ea067a2621912b2fd2e026d95c987782da6c52b638`.

## Concept provenance

The role decomposition is inspired by MIT LAMM's
[SciAgentsDiscovery](https://github.com/lamm-mit/SciAgentsDiscovery), inspected at commit
[`c5c30451b29cba813a11b5ce078909a214dad9f2`](https://github.com/lamm-mit/SciAgentsDiscovery/tree/c5c30451b29cba813a11b5ce078909a214dad9f2)
under its Apache-2.0 `LICENSE.txt`. This crate is a clean Rust evidence-contract implementation. It
contains no upstream Python, prompts, GraphML, pickle, embeddings, weights, or runtime.

The earlier local SciClaw/SciCrucible work contributed concepts only: explicit artifact parents,
declared consumed/produced schemas, separate literature/hypothesis/contradiction/critic roles,
preserved uncertainty, and a human overseer. Its current mixed tree was audited read-only; no code
was copied. Numeric trust voting, automatic peer-review thresholds, persisted private reasoning,
and unknown-safety-as-approval are deliberately excluded.

## Terminal use

Verify an already-created, human-selected, quiescent offline Artifact:

```sh
cargo run -p chimiaclaw-discovery --offline -- \
  verify-export signed-artifact.json --json > discovery-dossier.json
```

The command accepts one bounded regular `.json` file, refuses a final symlink and FIFO on Unix,
and emits the canonical dossier only after Artifact/payload/semantic validation. Errors are stable,
path-redacted, report-only JSON. The crate performs no network, model, database, MCP, shell,
subprocess, storage, dispatch, or publication operation.

Command arguments are read as operating-system strings. Any non-UTF-8 argument fails closed as
command misuse with exit 64 before path conversion; stderr contains neither the hostile path nor a
panic. This is a process-level regression gate, not merely a direct function test.

## Verification snapshot

At `f215747992409d6359a62d450ca6a4c3c6e7095a` (after canonical-payload/parity commit
`e56aa396e47a00d060e7bce0ff98193d941d1c40`):

- `chimiaclaw-discovery`: 23 library + 6 CLI unit + 1 CLI process-integration tests, PASS;
- `chimiaclaw-artifact`: 9 tests, PASS offline;
- package formatting: PASS;
- strict package/all-targets, no-dependencies Clippy: PASS locked/offline;
- full OpenAgents workspace: PASS locked/offline;
- full OpenAgents all-features workspace check: PASS locked/offline;
- isolated Olympus checkout at the exact commit: focused tests, package formatting, strict package
  no-dependencies Clippy, digest replay, full locked/offline workspace tests, and the locked/offline
  all-features check PASS; checkout clean.

The matching Zopf commit `3ad9ef221c129f2afb4bb5da42e4f7a2d71b8f12` passes Babashka 46 tests /
385 assertions, dependency-free JVM 40 / 355, full JVM/JDBC 112 / 886, and normal Python 374 run /
11 skipped / zero failures. System Python is 372 run / 15 skipped with the sole known transcript
failure.

The final Git bundle `openagents-zopf-sciagents-f215747.bundle` is 12,260,912 bytes with SHA-256
`16b29d8d5d2664cfa7d319f0c74a76c50ccdd8ca8f6e833872edb7ec1239e54a` and passes Git's complete
bundle verification. Olympus was fast-forwarded from the baseline commit and left clean. Its
focused/package gates, full locked/offline workspace tests, and the locked/offline all-features
check pass at the exact tip after a cache-only warm from the locally verified Cargo registry; no
network fetch was attempted. Workspace-wide formatting and Clippy beyond the discovery package remain
pre-existing unrelated debt.

Independent final adversarial review returned **PASS on 2026-08-13** for exact Rust commit
`f215747992409d6359a62d450ca6a4c3c6e7095a` and exact Zopf commit
`3ad9ef221c129f2afb4bb5da42e4f7a2d71b8f12`. It found no unresolved correctness or security
blocker inside the bounded v1 boundary. This is not a scientific-validation, trusted-origin,
deployment-readiness, or pilot-approval claim.

## Parked

Trusted signer identity, recursive parent resolution, live SciAgents/model execution, source
fetching, citation-entailment scoring, database or knowledge-graph ingestion, DuckDB/LanceDB
projection, MCP/Buzz, autonomous scheduling, dispatch, publication, and deployment all require
separate contracts and explicit human decisions.
