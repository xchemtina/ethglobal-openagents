# Private workstream observer

This is the first, observation-only stage of durable fleet workstreams. It retains
one exact JSON receipt, a local observation and all intended inbox deliveries in
one SQLite transaction. It survives process restarts and duplicate submissions.
It does not start jobs, acquire leases, open gates, send notifications or record
human acknowledgment. Existing boards and owners remain authoritative.

Use Python **3.10 or later on Unix** from the repository root; no additional
packages are required. Windows fails closed until its file/identity boundary is
implemented and independently qualified.

```sh
python3 -m unittest discover -s integrations/workstreams/tests -v
python3 -m integrations.workstreams.demo
```

The synthetic demo creates two separate inbox capabilities, observes a paused
workstream, retries the event, reopens its journal, records only AI inbox delivery,
and checks that the other inbox can catch up. Its temporary state is deleted.
Use `--receipt /private/existing-directory/new-receipt.json` to retain the public
synthetic result; the containing directory must be mode 0700 and the file new.

## Local use

Choose an existing private, locally owned directory. Paths must not traverse
symlinks or `..`; on macOS use a resolved `/private/...` temporary path where
necessary. Do not store this database on a network filesystem or synchronize the
live database file between machines. Keep the working directory stable and owned
by the operator.

```sh
python3 -m integrations.workstreams --journal /private/local/state/journal init
python3 -m integrations.workstreams --journal /private/local/state/journal register \
  --recipient owner-inbox --token-file /private/local/state/owner.token
python3 -m integrations.workstreams --journal /private/local/state/journal register \
  --recipient ai-inbox --token-file /private/local/state/ai.token

python3 -m integrations.workstreams --journal /private/local/state/journal observe \
  --receipt /private/local/state/completed-test.json \
  --expect-sha256 'sha256:REPLACE_WITH_INDEPENDENTLY_HELD_DIGEST' \
  --workstream software-test --source-host producer-host --source-session session-1 \
  --owner-reported owner --gate-reported paused \
  --next-step 'Inspect the completed test receipt' \
  --recipient owner-inbox --recipient ai-inbox

python3 -m integrations.workstreams --journal /private/local/state/journal inbox \
  --recipient ai-inbox --token-file /private/local/state/ai.token
python3 -m integrations.workstreams --journal /private/local/state/journal received \
  --recipient ai-inbox --token-file /private/local/state/ai.token \
  --event 'sha256:REPLACE_WITH_EXACT_EVENT_ID'
python3 -m integrations.workstreams --journal /private/local/state/journal audit
```

Obtain the expected receipt digest from the accepted local test or separately
trusted handoff. Merely computing a digest from an unknown sender's bytes does
not authenticate the sender. A digest match establishes byte identity only.
The source host/session, owner, gate and proposed next step are operator-reported
observations. Source identity is not cryptographically authenticated by this
module. Receipt claims remain **unverified**, including any embedded “pass,”
signature, scientific-review or publication claim. Use the existing federation
verifier separately when cryptographic evidence validation is required.

An observation is identified by its receipt digest, workstream, reported source,
owner/gate, next proposal and sorted recipient set. An exact retry returns the
same event and retains existing delivery receipts. Changing any of that metadata
creates a distinct observation, never a replacement. Observation timestamps are
the receiving machine's local clock; they are not a trusted global ordering.

`inbox` returns up to 100 pending observations and does not mark them received.
`received` records delivery of one exact assigned event to one recipient. Repeating
it is idempotent. There is no cumulative cursor that could skip unseen events.
The result always retains `human_awareness: unknown`. Neither an inbox with a
human's name nor its delivery token proves that a human read anything.

## Boundaries and recovery

- The journal directory is mode 0700; its database and generated tokens are 0600.
  Files must be regular, singly linked and owned by the calling user. Symlinks,
  parent traversal, shared permissions and unsupported platforms are rejected.
- Store token files in a separate private directory outside the journal. Only a
  SHA-256 hash of each random 256-bit token is retained in SQLite. Tokens bind the
  recipient and journal. They grant inbox delivery access only. No token appears
  in normal CLI output or diagnostic messages.
- This is **not an OS sandbox or remote authentication server**. The calling OS
  account is the trusted operator. A process with access to that account's files
  can read or alter the database and token files. Separate account/signing access
  is still needed before introducing human-only actions or untrusted AI clients.
- Input JSON is limited to 1 MiB, 32 nesting levels, 20,000 values/keys and bounded
  strings/numbers. Duplicate keys, non-finite numbers and invalid encodings fail.
  There are at most 64 registered recipients and 4,096 observations; the database
  is bounded to 128 MiB with SQLite's page limit. Full capacity is a refusal,
  never permission to prune evidence silently.
- SQLite DELETE journaling, full synchronization and immediate write transactions
  serialize append attempts. Exact receipt bytes, event and outbox rows commit
  together. Interrupted transactions recover through SQLite's rollback journal;
  do not remove a surviving `journal.sqlite3-journal` by hand.
- Registration writes its create-only token before committing its hash. A crash
  in that narrow interval can leave an **unregistered** private token file. It
  grants nothing. Re-register with a new file path after checking that recipient's
  registration; never overwrite the old file blindly. No token rotation/revocation
  interface is included in this first slice.
- The hash chain and `audit` detect local corruption and missing outbox rows; they
  are not signatures, external checkpoints or protection against a malicious
  same-account writer rewriting the database. All schema objects, version and
  application ID must match this implementation. Unknown schemas are not migrated.
- Snapshot/copy only while no writer is running, and retain the complete private
  journal directory. Automated live backup, restore qualification, fleet transport,
  recipient revocation, execution grants and automatic push remain later work.

Downstream AI clients must treat receipt content and next-step text as data, not
instructions that grant tools or permissions. The observer never reads referenced
files, imports receipt code, evaluates a command or changes an existing board.

The tests include concurrent multi-process retries, forced process death before
commit, recovery without partial events, offline recipient catch-up, capability
isolation, malformed input, file permission/path attacks and refusal of execution
or human-acknowledgment commands.
