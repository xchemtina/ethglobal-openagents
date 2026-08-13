# chimiaclaw-discovery

Lifecycle stage: **testing**.

This crate is the Rust/OpenAgents side of a report-only discovery boundary. It
validates one closed `science.discovery.dossier.v1`, binds its canonical JSON
bytes into an inline BLAKE3 payload, seals the envelope with the existing
OpenAgents Ed25519 artifact machinery, and verifies both signature and payload
before export. It grants no database, execution, dispatch, publication, or
human-decision authority.

The terminal verifier is deliberately narrow:

```sh
cargo run -p chimiaclaw-discovery -- \
  verify-export signed-artifact.json --json > discovery-dossier.json
```

It accepts one bounded regular JSON file, refuses a final symbolic link, and
emits only canonical dossier JSON after successful signed-integrity, payload,
and semantic validation. The embedded signing key is not a trusted identity
anchor, so v1 does not claim origin authenticity. Errors are path-redacted,
report-only envelopes. The
crate has no model, network, database, MCP, shell, or subprocess dependency.

The design is inspired by the role decomposition in MIT LAMM's
[SciAgentsDiscovery](https://github.com/lamm-mit/SciAgentsDiscovery), inspected
at commit `c5c30451b29cba813a11b5ce078909a214dad9f2` under its Apache-2.0
`LICENSE.txt`. This is a clean Rust reimplementation of a typed evidence
contract. No upstream Python source, prompts, model weights, pickle data,
GraphML, or runtime is embedded.

Concepts retained from the earlier ChimiaDAO SciClaw/SciCrucible design are
mapped without adding runtime authority: literature/planning produces sources
and claims; hypothesis roles produce falsifiable hypotheses and proposed
tests; contradiction roles produce inert conflict records; critic and
reconciliation roles produce open questions; and only a human may change the
pending decision. Numeric trust scores, automatic peer-review thresholds, and
persisted private reasoning traces are intentionally excluded.

The seven proposal facets used by the upstream SciAgents prototype are carried
as evidence rather than as an autonomous chat protocol: `hypothesis` maps to a
typed hypothesis; `outcome` maps to predictions and positive/falsifying
signals; `mechanisms`, `design_principles`, `comparison`, and `novelty` map to
supported claims with explicit uncertainty; and `unexpected_properties` maps
to testable claims or open questions. An independent critic records conflicts,
falsifiers, and unresolved questions. No facet is promoted merely because a
model emitted it.

The corresponding Zopf validator independently computes normalized SHA-256
over the same synthetic fixture. Neither a valid structure nor agreement
between validators establishes scientific truth. Agents draft; a human decides.

The copied cross-host string/locator/ordering corpus has raw SHA-256
`30bddc1f4adb05d06a92355b5dd6656ad7d6b6c382ca68290c39a1b6ce72ef4c`.
Both validators execute every case. The Rust dependency lock deliberately pins
the regex transitive versions available in the offline Olympus cache.

## Known limits

- Artifact canonical bytes use the existing OpenAgents serialization for its
  signed envelope; the dossier export separately uses lexical-key canonical
  JSON for cross-language review. The BLAKE3 artifact hash and Zopf SHA-256 are
  intentionally different proofs.
- The CLI's `O_NOFOLLOW` protects the final component on Unix. It does not
  establish descriptor-relative ancestry or a hostile concurrent path-race
  boundary. V1 is for a human-selected, quiescent offline artifact file.
- Parent artifact identifiers are signature-bound references. V1 does not
  recursively resolve their stores or claim their scientific content is true.
- No live model or SciAgents graph execution is part of this increment.
