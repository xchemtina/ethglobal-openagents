# Research workbench candidate

This candidate extends the hardened federation and observer with an explicit,
prospective binding between a terminal attempt receipt, its exact observer event,
and the exact signed PoSyn evidence package. The signed binding records the
reported terminal outcome, output identities and missing outputs. It does not
turn an unsigned historical event into signed history or prove that an execution
occurred.

The baseline is `b8d8bb56550e58e8b2557b3715e2879b8a27db46` on
`harden/workstreams-20261027`. The separate legacy development line remains
separate: its artifact-store file is already identical to this baseline, while
its gateway fulfillment changes target a different architecture. No automatic
legacy merge or default-branch merge is part of this candidate.

## Component responsibilities

| Component | Retained responsibility | New or preserved boundary |
|---|---|---|
| PoX, installed separately | Accepted inquiry scope, named participants, private delivery | Optional private viewer registry; authenticated tool discovery and administrator-only exact-file previews |
| PoSyn | Bounded manifest and object integrity against an independently held root | Current signed profile retains exactly one nonempty result-role payload |
| Federation | Trusted signer, scoped delegation, exact evidence-bound human review | New `bind-attempt` and `verify-attempt` commands |
| Observer | Durable local event, inbox/outbox and retry behavior | Original event remains unsigned and execution-unverified |

No smart contract or blockchain transaction is required. The private PoX
implementation, registry, credentials, scientific files and laptop sources are
not distributed in this repository. Its five core module pins remain unchanged
by the separate viewer addition; the existing read-only adapter is still the
integration boundary.

## Reproduce the generic protocol checks

```sh
cargo test --locked -p chimiaclaw-federation
cargo clippy --locked -p chimiaclaw-posyn -p chimiaclaw-federation --all-targets --no-deps -- -D warnings
cargo build --locked -p chimiaclaw-federation
python3 -B demo/federation/run.py
python3 -B demo/federation/attempt.py
python3 -B -m unittest discover -s integrations/workstreams/tests -v
python3 -B -m integrations.workstreams.demo
python3 -B -m unittest discover -s integrations/pox/tests -v
```

See the [attempt protocol and CLI](../crates/chimiaclaw-federation/README.md)
for recipient expectations and failure-record requirements. A zero-output
failure retains a typed failure record in PoSyn's required result role. That
record is not a produced scientific output and cannot be relabeled as success.
Synthetic demos use temporary identities and state; they do not demonstrate
separate human participation, actual scientific execution or notification
delivery.

To check a separately installed compatible PoX candidate, set `POX_SOURCE` to
that source directory and run the adapter tests and
`integrations/pox/demo.py --pox-source "$POX_SOURCE"`. See the
[adapter guide](../integrations/pox/README.md) for the binary argument and optional
private receipt. Do not alter the five source pins merely to bypass a refusal.

## Next acceptance gates

1. Qualify the generic candidate and the separate private PoX revision; retain
   their exact commit and test receipts.
2. Attach the prospective attempt emitter to one explicitly authorized bounded
   real task. Independently retain its expectation and compare the actual
   inputs/environment, rather than trusting the producer's hash assertions.
3. Have three actual participants use separately controlled keys and accounts
   for scope acceptance, review and acknowledgment of the next action.
4. Add further viewer adapters only after fixing source-specific security and
   scientific-meaning gaps. A catalog descriptor is not a qualified native
   application or a running service.

The new binding is a signed assertion with exact byte relationships. Execution,
scientific validity, human presence and external publication remain separate
claims requiring their own evidence and authority.
