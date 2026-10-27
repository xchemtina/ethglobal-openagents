"""One local observer journal. Its records grant no external authority."""
from __future__ import annotations

from contextlib import contextmanager
import re
import secrets
import sqlite3
import time

from .safeio import (JournalError, MAX_DATABASE, canonical, check_file, digest,
                     identifier, private_directory, read_regular, safe_path,
                     strict_json, sync_directory, text_field, write_new)

APPLICATION_ID = 0x4F41574A
VERSION = 1
MAX_RECIPIENTS = 64
MAX_EVENTS = 4096
MAX_TIMESTAMP = 253402300799  # Last second of year 9999, not an authority clock.
GENESIS = "sha256:" + "0" * 64
ASSURANCES = {
    "record_kind": "unsigned_local_observation",
    "receipt_claims": "unverified",
    "owner_status": "last_reported_only",
    "source_identity": "operator_reported_not_authenticated",
    "gate_authority": "existing_board",
    "human_awareness": "unknown",
    "independent_review": "absent",
    "execution_authorized": False,
    "lease_authority": False,
    "scientific_admission": False,
    "publication_authorized": False,
}
SCHEMA = """
CREATE TABLE metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE recipients(id TEXT PRIMARY KEY, token_hash TEXT NOT NULL UNIQUE);
CREATE TABLE events(
 seq INTEGER PRIMARY KEY, event_id TEXT NOT NULL UNIQUE,
 payload BLOB NOT NULL, receipt BLOB NOT NULL,
 previous_hash TEXT NOT NULL, chain_hash TEXT NOT NULL UNIQUE,
 observed_at INTEGER NOT NULL);
CREATE TABLE outbox(
 event_id TEXT NOT NULL REFERENCES events(event_id),
 recipient TEXT NOT NULL REFERENCES recipients(id),
 received_at INTEGER,
 PRIMARY KEY(event_id,recipient));
CREATE TRIGGER events_no_update BEFORE UPDATE ON events
 BEGIN SELECT RAISE(ABORT,'immutable_event'); END;
CREATE TRIGGER events_no_delete BEFORE DELETE ON events
 BEGIN SELECT RAISE(ABORT,'immutable_event'); END;
"""


def _schema_objects(connection):
    return {(row[0], row[1], row[2]) for row in connection.execute(
        "SELECT type,name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'")}


def _expected_schema():
    # One source of truth: derive the expected objects from our static schema,
    # never from the journal being opened. No private data is copied here.
    connection = sqlite3.connect(":memory:")
    try:
        connection.executescript(SCHEMA)
        return _schema_objects(connection)
    finally:
        connection.close()


EXPECTED_SCHEMA = _expected_schema()


class Journal:
    """The calling OS account is the trusted operator; tokens restrict inbox access.

    This class is not a network authorization boundary. A same-account process can
    edit its files, so hash chains detect corruption rather than prove provenance.
    """

    def __init__(self, root):
        self.root = private_directory(root)
        self.database = self.root / "journal.sqlite3"
        with self._connect() as connection:
            self._validate_schema(connection)

    @classmethod
    def create(cls, root):
        root = safe_path(root)
        try:
            root.mkdir(mode=0o700)
        except OSError:
            raise JournalError("journal_exists_or_unavailable") from None
        private_directory(root)
        database = root / "journal.sqlite3"
        write_new(database, b"")
        connection = sqlite3.connect(database, isolation_level=None)
        try:
            connection.execute("PRAGMA journal_mode=DELETE")
            connection.execute("PRAGMA synchronous=FULL")
            connection.executescript("BEGIN IMMEDIATE;" + SCHEMA)
            connection.execute("INSERT INTO metadata VALUES('journal_id', ?)",
                               (secrets.token_hex(32),))
            connection.execute(f"PRAGMA application_id={APPLICATION_ID}")
            connection.execute(f"PRAGMA user_version={VERSION}")
            connection.commit()
        except sqlite3.Error:
            connection.rollback()
            raise JournalError("journal_initialization_failed") from None
        finally:
            connection.close()
        sync_directory(root)
        return cls(root)

    def _file_checks(self):
        private_directory(self.root)
        # A hot rollback journal may survive a killed writer and must be retained.
        for path in self.root.iterdir():
            if path.name not in ("journal.sqlite3", "journal.sqlite3-journal"):
                raise JournalError("unexpected_journal_file")
            safe_path(path)
            try:
                check_file(path.lstat(), MAX_DATABASE, True)
            except FileNotFoundError:
                if path.name == "journal.sqlite3":
                    raise JournalError("database_unavailable") from None
        try:
            check_file(self.database.lstat(), MAX_DATABASE, True)
        except OSError:
            raise JournalError("database_unavailable") from None

    @contextmanager
    def _connect(self):
        self._file_checks()
        connection = None
        try:
            connection = sqlite3.connect(self.database.as_uri() + "?mode=rw",
                                         uri=True, timeout=10, isolation_level=None)
            connection.row_factory = sqlite3.Row
            connection.execute("PRAGMA foreign_keys=ON")
            connection.execute("PRAGMA trusted_schema=OFF")
            connection.execute("PRAGMA synchronous=FULL")
            connection.execute("PRAGMA temp_store=MEMORY")
            connection.execute("PRAGMA max_page_count=32768")
            yield connection
        except sqlite3.Error:
            if connection is not None:
                connection.rollback()
            raise JournalError("journal_database_error") from None
        finally:
            if connection is not None:
                connection.close()

    @staticmethod
    def _validate_schema(connection):
        if (connection.execute("PRAGMA application_id").fetchone()[0] != APPLICATION_ID
                or connection.execute("PRAGMA user_version").fetchone()[0] != VERSION
                or connection.execute("PRAGMA journal_mode").fetchone()[0] != "delete"):
            raise JournalError("unsupported_journal_schema")
        if _schema_objects(connection) != EXPECTED_SCHEMA:
            raise JournalError("unsupported_journal_schema")
        rows = connection.execute("SELECT key,value FROM metadata").fetchall()
        if (len(rows) != 1 or rows[0][0] != "journal_id"
                or not isinstance(rows[0][1], str)
                or not re.fullmatch(r"[a-f0-9]{64}", rows[0][1])):
            raise JournalError("invalid_journal_metadata")

    @staticmethod
    def _validate_state(connection):
        # SQLite's column affinities allow TEXT/BLOB in INTEGER/TEXT columns.
        # Validate before filtering pending rows or comparing secret hashes, so
        # corrupt values cannot hide delivery intent or escape as a TypeError.
        for row in connection.execute("SELECT id,token_hash FROM recipients"):
            identifier(row[0])
            if (not isinstance(row[1], str)
                    or not re.fullmatch(r"sha256:[a-f0-9]{64}", row[1])):
                raise JournalError("recipient_integrity_failed")
        invalid_event = connection.execute(
            "SELECT 1 FROM events WHERE typeof(observed_at)!='integer' "
            "OR observed_at<0 OR observed_at>? LIMIT 1", (MAX_TIMESTAMP,)).fetchone()
        invalid_delivery = connection.execute(
            "SELECT 1 FROM outbox WHERE received_at IS NOT NULL AND "
            "(typeof(received_at)!='integer' OR received_at<0 OR received_at>?) LIMIT 1",
            (MAX_TIMESTAMP,)).fetchone()
        if invalid_event is not None or invalid_delivery is not None:
            raise JournalError("timestamp_integrity_failed")

    @staticmethod
    def _now():
        now = int(time.time())
        if not 0 <= now <= MAX_TIMESTAMP:
            raise JournalError("invalid_local_clock")
        return now

    @contextmanager
    def _transaction(self):
        with self._connect() as connection:
            self._validate_schema(connection)
            connection.execute("BEGIN IMMEDIATE")
            try:
                self._validate_state(connection)
                yield connection
                connection.commit()
            except BaseException:
                connection.rollback()
                raise

    def register(self, recipient: str, token_file):
        recipient = identifier(recipient)
        token_file = safe_path(token_file)
        if token_file == self.root or self.root in token_file.parents:
            raise JournalError("token_must_be_outside_journal")
        with self._transaction() as connection:
            if connection.execute("SELECT 1 FROM recipients WHERE id=?", (recipient,)).fetchone():
                raise JournalError("recipient_already_registered")
            if connection.execute("SELECT COUNT(*) FROM recipients").fetchone()[0] >= MAX_RECIPIENTS:
                raise JournalError("recipient_limit_reached")
            journal_id = connection.execute("SELECT value FROM metadata").fetchone()[0]
            secret = secrets.token_hex(32)
            token = {"schema": "openagents.workstream.inbox-token.v1", "journal_id": journal_id,
                     "recipient": recipient, "token": secret}
            # Write first: a crash can leave an unusable file, never an unbacked grant.
            write_new(token_file, canonical(token))
            connection.execute("INSERT INTO recipients VALUES(?,?)", (recipient, digest(secret.encode())))
        return {"recipient": recipient, "permission": "inbox_delivery_only"}

    def observe(self, receipt_file, expected_sha256: str, *, workstream_id: str,
                source_host: str, source_session: str, owner_last_reported: str,
                gate_reported: str, next_step_proposed: str, recipients):
        raw = read_regular(receipt_file)
        strict_json(raw)
        if expected_sha256 != digest(raw):
            raise JournalError("receipt_hash_mismatch")
        if gate_reported not in ("open", "paused", "unknown"):
            raise JournalError("invalid_reported_gate")
        if not isinstance(recipients, (list, tuple)) or not 1 <= len(recipients) <= MAX_RECIPIENTS:
            raise JournalError("invalid_recipients")
        recipients = sorted(identifier(item) for item in recipients)
        if len(set(recipients)) != len(recipients):
            raise JournalError("duplicate_recipient")
        payload = {
            "schema": "openagents.workstream.observation.v1",
            "workstream_id": identifier(workstream_id), "receipt_sha256": expected_sha256,
            "source_host": identifier(source_host), "source_session": identifier(source_session),
            "owner_last_reported": identifier(owner_last_reported), "gate_reported": gate_reported,
            "next_step_proposed": text_field(next_step_proposed), "recipients": recipients,
            "assurances": ASSURANCES,
        }
        encoded = canonical(payload)
        event_id = digest(encoded)
        with self._transaction() as connection:
            for recipient in recipients:
                if not connection.execute("SELECT 1 FROM recipients WHERE id=?", (recipient,)).fetchone():
                    raise JournalError("recipient_not_registered")
            existing = connection.execute("SELECT * FROM events WHERE event_id=?", (event_id,)).fetchone()
            if existing is not None:
                if existing["payload"] != encoded or existing["receipt"] != raw:
                    raise JournalError("event_integrity_mismatch")
                return {"event_id": event_id, "new": False, "assurances": ASSURANCES.copy()}
            row = connection.execute("SELECT seq,chain_hash FROM events ORDER BY seq DESC LIMIT 1").fetchone()
            seq, previous = (row[0] + 1, row[1]) if row else (1, GENESIS)
            if seq > MAX_EVENTS:
                raise JournalError("event_limit_reached")
            observed_at = self._now()
            chain_hash = digest(canonical([seq, event_id, previous, observed_at]))
            connection.execute("INSERT INTO events VALUES(?,?,?,?,?,?,?)",
                               (seq, event_id, encoded, raw, previous, chain_hash, observed_at))
            connection.executemany("INSERT INTO outbox(event_id,recipient) VALUES(?,?)",
                                   [(event_id, recipient) for recipient in recipients])
        return {"event_id": event_id, "new": True, "assurances": ASSURANCES.copy()}

    @staticmethod
    def _authenticate(connection, recipient, token_file):
        identifier(recipient)
        token = strict_json(read_regular(token_file, 4096, private=True))
        if (set(token) != {"schema", "journal_id", "recipient", "token"}
                or token.get("schema") != "openagents.workstream.inbox-token.v1"
                or token.get("recipient") != recipient
                or not isinstance(token.get("token"), str)
                or len(token["token"]) != 64):
            raise JournalError("inbox_authentication_failed")
        journal_id = connection.execute("SELECT value FROM metadata").fetchone()[0]
        row = connection.execute("SELECT token_hash FROM recipients WHERE id=?", (recipient,)).fetchone()
        if (token.get("journal_id") != journal_id or row is None
                or not secrets.compare_digest(row[0], digest(token["token"].encode()))):
            raise JournalError("inbox_authentication_failed")

    def inbox(self, recipient: str, token_file, *, limit=100):
        if type(limit) is not int or not 1 <= limit <= 100:
            raise JournalError("invalid_inbox_limit")
        with self._transaction() as connection:
            self._authenticate(connection, recipient, token_file)
            rows = connection.execute(
                "SELECT e.event_id,e.payload,e.observed_at FROM events e JOIN outbox o "
                "ON e.event_id=o.event_id WHERE o.recipient=? AND o.received_at IS NULL "
                "ORDER BY e.seq LIMIT ?", (recipient, limit)).fetchall()
            return [{"event_id": row[0], "observation": strict_json(row[1]),
                     "observed_at_unix": row[2]} for row in rows]

    def received(self, recipient: str, token_file, event_id: str):
        if not isinstance(event_id, str) or len(event_id) != 71 or not event_id.startswith("sha256:"):
            raise JournalError("invalid_event_id")
        with self._transaction() as connection:
            self._authenticate(connection, recipient, token_file)
            row = connection.execute("SELECT received_at FROM outbox WHERE event_id=? AND recipient=?",
                                     (event_id, recipient)).fetchone()
            if row is None:
                raise JournalError("event_not_in_recipient_inbox")
            if row[0] is None:
                received_at = self._now()
                connection.execute("UPDATE outbox SET received_at=? WHERE event_id=? AND recipient=?",
                                   (received_at, event_id, recipient))
            else:
                received_at = row[0]
        return {"event_id": event_id, "recipient": recipient, "received_at_unix": received_at,
                "receipt_kind": "inbox_delivery", "human_awareness": "unknown",
                "execution_authorized": False}

    def audit(self):
        with self._transaction() as connection:
            if connection.execute("PRAGMA integrity_check").fetchone()[0] != "ok":
                raise JournalError("database_integrity_failed")
            if connection.execute("PRAGMA foreign_key_check").fetchone() is not None:
                raise JournalError("database_integrity_failed")
            previous, count, expected_deliveries = GENESIS, 0, 0
            for row in connection.execute("SELECT * FROM events ORDER BY seq"):
                count += 1
                raw = row["receipt"]
                strict_json(raw)
                payload = strict_json(row["payload"])
                if (row["seq"] != count or row["previous_hash"] != previous
                        or row["event_id"] != digest(row["payload"])
                        or canonical(payload) != row["payload"]
                        or payload.get("receipt_sha256") != digest(raw)
                        or payload.get("assurances") != ASSURANCES
                        or row["chain_hash"] != digest(canonical([
                            count, row["event_id"], previous, row["observed_at"]]))):
                    raise JournalError("event_integrity_failed")
                actual = [r[0] for r in connection.execute(
                    "SELECT recipient FROM outbox WHERE event_id=? ORDER BY recipient", (row["event_id"],))]
                if payload.get("recipients") != actual:
                    raise JournalError("outbox_integrity_failed")
                expected_deliveries += len(actual)
                previous = row["chain_hash"]
            deliveries, received = connection.execute(
                "SELECT COUNT(*),COUNT(received_at) FROM outbox").fetchone()
            if deliveries != expected_deliveries:
                raise JournalError("outbox_integrity_failed")
            return {"schema": "openagents.workstream.audit.v1", "status": "pass",
                    "events": count, "deliveries": deliveries, "received": received,
                    "pending": deliveries - received, "head_hash": previous,
                    "integrity_scope": "local_corruption_detection_not_provenance",
                    "assurances": ASSURANCES.copy()}
