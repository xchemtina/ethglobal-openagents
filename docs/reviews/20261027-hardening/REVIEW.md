# Tested evidence exchange and a hardened workstream observer

27 October 2026. This branch extends the existing offline integration with a
private local observer, dependency fixes and stronger CI controls. It was tested
locally and on a second macOS machine using synthetic inputs. It is not a deployed
fleet scheduler, notification service or human-signing interface.

**What changed.** The new `integrations/workstreams` module atomically retains
exact JSON receipt bytes, an immutable observation and intended deliveries in
SQLite. Repeated submissions deduplicate; each recipient has a separate delivery
capability and receipt. A process restart restores pending items. A forced death
before commit leaves no partial event or delivery. Concurrent processes submitting
the same observation produce one stored event.

The observer requires a separately supplied SHA-256 receipt digest. It records
source, owner, gate and next step as operator-reported data. Those records remain
unsigned observations, with source identity and receipt claims unverified. The
existing federation verifier remains the separate cryptographic boundary.
An inbox receipt always leaves human awareness unknown. Existing boards retain
ownership and gate authority. No command starts work, opens a gate, acquires a
lease, sends a message or records a human acknowledgment.

The storage boundary rejects shared permissions, wrong ownership, symlinks,
hardlinks, path traversal, unsupported platforms and unknown database schemas.
JSON size/depth/node/string bounds reject malformed, duplicate and nonfinite
values. Recipient capabilities bind both journal and recipient; only token hashes
are stored in the database. Independent review found a corruption gap where a
malformed delivery timestamp could hide a pending item, and a malformed stored
token hash could raise a type error. Both now fail closed before inbox filtering,
authentication or audit; regressions cover the fixes.

**Functional validation.** These are local results bound to the file hashes in
`VALIDATION.json`; hosted checks are available on the branch's Actions page.

| Suite | Passed tests |
| --- | ---: |
| Rust workspace, all features and locked dependencies | 167 |
| Gateway | 22 |
| Web URL boundary | 2 |
| Literature worker | 70 |
| Offline DFT guard/stub worker | 10 |
| Actual pinned PoX adapter | 10 |
| Workstream observer | 40 |
| **Total** | **321** |

Gateway and web typechecks and production builds passed. Scoped PoSyn/federation
Clippy passed with warnings denied. Synthetic journeys passed another 27
federation checks, 14 real PoX/Rust checks and 12 observer checks. The worker tests
do not represent scientific calculations or external provider qualification.
The separately installed PoX source is private; public CI explicitly skips its
five source-dependent tests when that source is unavailable.

A second physical macOS machine ran the 40 observer tests, 12 observer journey
checks and 27 federation journey checks. In addition, a synthetic package signed
on the first machine was transferred with a pinned file manifest, verified on the
second machine and recorded in its observer. Only public synthetic policy and
evidence moved; signing secrets stayed in temporary state on the source machine.
Both machines' temporary canary state was removed. This establishes a bounded
two-machine test, not independent human participation or production deployment.
The intended Windows machine answered SSH but rejected unattended authentication;
its runtime and jobs remain unverified. The current private-file boundary is Unix
only, so Windows support still requires an adapter and qualification.

**Dependency hardening.** Point-in-time advisory scans were checked against exact
lockfiles. Counts below are vulnerable package entries, not unique exploits.

| Surface | Before | After |
| --- | ---: | ---: |
| Gateway npm production audit | 10 | 0 |
| Web npm production audit | 8 | 0 |
| Rust applicable package/version findings | 5 | 0 after explicit upstream adjudication |
| Python pinned test requirements | 0 | 0 |

Both full npm audits also report zero known vulnerabilities. The gateway remains
on Express 4, now 4.22.3, with Zod 4.6.7 and patched parser/proxy dependencies.
Next is 16.4.3 with patched Sharp/PostCSS/transitives. Unused optional x402 SDKs
and unused web validation dependencies were removed; live payments remain
unavailable and fail closed. A compressed-request regression verifies malformed
trailing bytes return a generic HTTP 400 and the server remains responsive.
See the relevant [proxy-addr](https://github.com/advisories/GHSA-jqcg-44mw-7w3h),
[Zod](https://github.com/advisories/GHSA-mrhx-c7xc-m3w5),
[Next.js](https://github.com/advisories/GHSA-2464-fhr4-w9xw) and
[body-parser](https://github.com/advisories/GHSA-27r3-jxmh-p828) advisories.

The Rust lock now pins quinn-proto 0.11.18, Rustls 0.23.46, rustls-webpki 0.103.16,
webpki-roots 1.0.10 and Tokio 1.53.3. A fresh OSV scan covered all 175 resolved
registry package versions. Its sole raw hit was `block-buffer 0.10.4`: the
aggregate record says `<0.12.1`, while the
[maintainer advisory](https://github.com/RustCrypto/utils/security/advisories/GHSA-qwgh-2vcv-g2f7)
explicitly limits affected versions to `>=0.11.0,<0.12.1`. The exclusion applies
only to locked 0.10.4; it is not a package-wide suppression. The raw hit and
adjudication are retained in the validation record. All 14 pinned Python test
requirements were scanned with pip-audit, with zero known vulnerabilities or skips.
These results do not prove absence of undiscovered vulnerabilities.

**CI and operational limits.** Actions are pinned to verified upstream commit
SHAs; checkout credentials are not persisted; job timeouts are bounded and token
permissions remain read-only. CI now includes observer tests on Linux and macOS
and production npm audit gates. These controls follow
[GitHub's secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use).
Local Node was 25.9 and Rust 1.89; hosted CI is configured to check Node 22 and Rust
stable separately. Its results must be checked against the exact branch commit.
The first hosted run failed during checkout because an unused historical submodule
had no `.gitmodules` definition. Its orphaned entry was replaced by an optional
upstream pointer retaining the exact revision, preserving credential cleanup.
The public branch contains methods and synthetic
fixtures; private source databases, host credentials and canary tokens are excluded.

The local OS account remains trusted. Any AI with the same filesystem privileges
can access that account's keys or journal, so human-only signing needs a separate
account or presence-enforcing signing boundary. Hash chains detect corruption;
they do not defeat a malicious same-account writer. The observer is not a remote
authentication service. Token rotation, live backup/restore, distributed claims,
execution grants, private push delivery and Windows adaptation remain later work.
No contracts, production services, scientific jobs or external messages were added.

Run the observer with `python3 -m integrations.workstreams.demo`; the detailed
[runbook](../../../integrations/workstreams/README.md), [PDF](REVIEW.pdf),
[editable LaTeX](REVIEW.tex) and [validation](VALIDATION.json) accompany this review.
