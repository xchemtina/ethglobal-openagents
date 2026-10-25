# OpenAgents: source review and improvements

**Review date:** 2026-10-25. **Delivery:** two independent branches; no merge or deployment.

The project has a useful signed-artifact substrate, but its public-facing claims and trust boundaries were ahead of its implementation. This review prioritized reproducible signature, storage, molecular identity, payment, and authorization defects. It does not qualify scientific results, live settlement, governance, or production operation.

## Scope and source identity

| Line | Starting source | Delivery branch | Scope |
|---|---|---|---|
| Cashier/discovery | `5946ab1`, descending from GitHub `master` at `a7fa866` | `improve/reliability-20261023` | Rust, gateway, web, offline workers, CI and dated docs |
| Legacy main | Committed `4318565`; selected gateway source copied from the separately modified main checkout | `improve/legacy-main-20261025` | Shared Rust fixes, reviewed gateway source, committed SciCrucible preview, optional notebook patch |

The original checkouts were preserved. The legacy branch is deliberately **not a bulk snapshot of the modified main tree**: notebook records, scientific runs, private artifacts, local environments, caches, and the remaining uncommitted work were not imported. The later branch retains four existing discovery commits beyond GitHub `master`; these were reviewed separately from the new fixes. `SOURCE_MANIFEST.json` in each branch identifies the changed source files by SHA-256.

Review covered the primary Rust/HTTP/UI paths, offline worker tests, scaffold contracts, and the modified main notebook/fulfillment paths. Legacy alternate landing pages, all historical experimental outputs, deployed services, provider accounts, and scientific model validity were not exhaustively audited.

## Implemented improvements

| Boundary | Reproduced or inspected failure | Result |
|---|---|---|
| Signatures | An identity public key and trivial signature were accepted without a signing key | Strict Ed25519 verification; forged artifact and discovery process regressions |
| Artifact storage | IDs escaped the store, filenames could substitute another artifact, writes raced and followed symlinks | Canonical IDs, read-time identity checks, regular-file checks, atomic publication without replacement |
| DFT response | Worker IDs could disagree with the input or escape the cube output directory | Input/response identity binding before writes, digest-derived filenames, exclusive creation |
| Live x402 | Any arbitrary nonempty signature header could authorize work | Live mode rejects requests until verification/settlement exists; metadata reports unavailable |
| Cached chemistry | Case folding matched cyclohexane `C1CCCCC1` to benzene `c1ccccc1` | Typed exact SMILES matching; label normalization remains separate |
| Gateway resources | Spoofed forwarding headers bypassed limits; MolADT subprocesses were unbounded | Trusted Express IP semantics, bounded rate state, worker time/output/concurrency limits and process cleanup |
| HTTP and packaging | Async storage errors escaped Express 4; container needed a runtime `npx` download | Async error boundary; compiled JavaScript container with production dependencies |
| Checkout | Small catalog prices were silently raised for Checkout API | Unsupported under-minimum Checkout requests fail explicitly |
| User interface | Local state was presented as submitted challenges, compute, wallet settlement, ORCID or review queues | Explicit previews/unavailable actions, fixed fixture dates, intentional HTTPS checkout links |
| Build quality | TypeScript failures were suppressed and lint commands were not installed | Type checking enforced, hidden legacy errors repaired, reproducible build/test CI |

The legacy gateway additionally addresses webhook replay and concurrent redemption with serialized per-store mutations and checkout-session idempotency. Its exact test receipt and single-process limitations are recorded alongside the source. Old credit ledgers lacking the new grant metadata require explicit reconciliation. Redemption is at-most-once: a later worker/logging failure requires manual entitlement recovery; no exactly-once delivery is claimed.

## Verification

| Check | Cashier/discovery | Legacy main |
|---|---|---|
| Rust full workspace, all features | 139 passed | 89 passed |
| Gateway regression tests | 21 passed | 24 passed |
| Gateway TypeScript and compiled build | Passed | Passed |
| Frontend TypeScript and production build | Passed; 3 static pages | Passed; 43 generated routes |
| Frontend checkout URL tests | 2 passed | Not applicable |
| Offline Python workers, Python 3.12 | 70 literature + 10 Modal guard/stub tests passed | Not imported |
| Optional notebook authorization patch | Not applicable | 11 isolated checks passed |

Rust tests used the installed macOS 15.2 SDK because the host's default SDK could not be linked by its compiler. Initial cache/sandbox/build failures were resolved before the reported passing runs. CI targets Linux, Node 22, and Python 3.12; local success does not establish hosted CI success. Local Node was 25.9.0. Tests used local fixtures and fake subprocesses, without scientific computations or payment-provider transactions.

An independent review of the later branch found no blocking regression in the implemented changes, with reservations below. Credential-pattern checks on candidate changed files found no matching live-key/private-key patterns; this is a scoped check, not proof that the entire repository history contains no secrets.

## Notebook patch and unresolved issues

The modified, uncommitted notebook routes omit session authorization on three mutation surfaces. The legacy branch includes `notebook-auth.patch`, an isolated verifier, and a source/patch hash receipt. It adds session/role checks and actor binding, but **has not been applied to the original checkout** and does not import its notebook data.

The same isolated harness reproduced acceptance of a synthetic artifact with invalid signature, key, and hashes after JSON payload comparison. It was labelled ready for manual release; no payout occurred. This remains unresolved. Claim/review storage also needs transactional concurrency protection, and GET routes need an explicit access policy before integration.

Other remaining boundaries:

1. Cached DFT files are trusted operator inputs; the gateway does not independently verify their signatures. Strict Rust verification does not automatically cover that path.
2. Direct Rust DFT worker entry points still lack time/output bounds. The new process limits apply to the gateway's MolADT invocation.
3. Live x402 settlement and live DFT remain unavailable. Human Payment Links require fulfillment appropriate to the branch; a configured link does not prove payment or delivery.
4. Solidity contracts remain scaffolds: reputation scores can be set publicly, and governor authorization does not enforce quorum or voting. No contract deployment or financial authorization is implied.
5. Artifact publication requires filesystem hard-link support and an operator-owned directory. `path_for` is now fallible. Repeated cube output names fail instead of overwriting; use a fresh output directory per run.
6. Docker image execution, browser interactions, DNS, hosted uptime, and provider integration were not validated. No browser surface was connected for interactive inspection.

Next work should close notebook cryptographic authorization and transactional storage, verify cached artifacts at ingestion/serving, then qualify one bounded payment-to-artifact path with real provider test accounts. Keep the two development lines separate until their source and data ownership are reconciled.

## Reproduce

```sh
cargo test --workspace --all-features --locked
# In services/api-gateway on either branch:
npm ci --ignore-scripts
npm run typecheck
npm test
npm run build
# In web (later branch) or SciCrucible_v1 (legacy branch):
npm ci --ignore-scripts
npm run typecheck
npm run build
```

The later branch's CI installs the pinned Python 3.12 test dependencies in `.github/python-test-requirements.txt` and tests worker source through `PYTHONPATH`. Use the adjacent `BUILD.sh` with a LaTeX engine to rebuild this report; the final PDF pages were rendered and visually inspected. The diagram in the PDF is derived from repository component paths and branch ancestry, not measured scientific data.
