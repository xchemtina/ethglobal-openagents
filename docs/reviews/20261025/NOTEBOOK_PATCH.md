# Optional notebook authentication patch

`notebook-auth.patch` targets the separately modified, uncommitted SciCrucible
notebook routes. Those routes are absent from this branch's committed baseline.
The patch has **not** been applied to the original checkout, and no notebook
records, datasets, or pilot artifacts are included here.

It adds the existing shared session guard to next-task claims, correction
confirmation, and evidence reviews. Confirmation requires an operator/admin in
session-required mode; claim/reviewer identities bind to the session. It retains
the source's explicit development/session configuration policy.

`notebook-auth-check.json` records 11 isolated authorization checks and the exact
source/patch hashes. Re-run against a source copy with:

```sh
node docs/reviews/20261025/verify-notebook-auth.cjs /path/to/SciCrucible_v1
```

The verifier copies only four source files into a temporary directory, applies
the patch there, and uses in-memory fake storage and sessions. It does not read
notebook datasets or modify the original. Install this baseline's dependencies
first, or supply an alternate dependency root as the second argument.

The patch is incomplete as a production security fix. The same harness separately
reproduces acceptance of an artifact with deliberately invalid signature, key,
and hashes after payload comparison. It records `ready_for_manual_release`, but
makes no payment. Cryptographic verification, signer authorization, transactional
claim/review storage, public GET data policy, and end-to-end authentication remain
unresolved. Atomic rename alone does not prevent lost updates or competing claims.
