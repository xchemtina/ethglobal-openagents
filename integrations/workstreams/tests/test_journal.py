from __future__ import annotations

import contextlib
import io
import json
import os
from pathlib import Path
import select
import sqlite3
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest

from integrations.workstreams import Journal, JournalError
from integrations.workstreams.__main__ import main
from integrations.workstreams.safeio import MAX_RECEIPT, check_file, digest, read_regular, strict_json, write_new

REPO = Path(__file__).resolve().parents[3]


@unittest.skipUnless(os.name == "posix", "Private local observer currently requires Unix")
class JournalTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name).resolve()
        self.base.chmod(0o700)
        self.root = self.base / "journal"
        self.journal = Journal.create(self.root)
        self.human_token = self.base / "human.token"
        self.ai_token = self.base / "ai.token"
        self.journal.register("human", self.human_token)
        self.journal.register("ai", self.ai_token)
        self.receipt = self.base / "receipt.json"
        write_new(self.receipt, b'{"status":"pass","scientific_admission":true}\n')
        self.arguments = dict(workstream_id="work-1", source_host="local",
                              source_session="session-1", owner_last_reported="owner",
                              gate_reported="paused", next_step_proposed="Inspect result",
                              recipients=["human", "ai"])

    def observe(self, **changes):
        arguments = dict(self.arguments, **changes)
        return self.journal.observe(self.receipt, digest(self.receipt.read_bytes()), **arguments)

    def test_exact_bytes_retained_and_claims_never_promoted(self):
        event = self.observe()
        with sqlite3.connect(self.journal.database) as con:
            self.assertEqual(con.execute("SELECT receipt FROM events").fetchone()[0], self.receipt.read_bytes())
        record = self.journal.inbox("human", self.human_token)[0]
        self.assertEqual(record["event_id"], event["event_id"])
        self.assertEqual(record["observation"]["gate_reported"], "paused")
        self.assertEqual(record["observation"]["assurances"]["receipt_claims"], "unverified")
        self.assertFalse(record["observation"]["assurances"]["scientific_admission"])
        self.assertFalse(record["observation"]["assurances"]["execution_authorized"])

    def test_retry_deduplicates_and_preserves_delivery(self):
        event = self.observe()
        delivery = self.journal.received("ai", self.ai_token, event["event_id"])
        repeated = self.observe(recipients=["ai", "human"])
        self.assertFalse(repeated["new"])
        self.assertEqual(repeated["event_id"], event["event_id"])
        self.assertEqual(self.journal.received("ai", self.ai_token, event["event_id"]), delivery)
        self.assertEqual(self.journal.audit()["events"], 1)
        self.assertEqual(self.journal.audit()["received"], 1)

    def test_restart_and_offline_catchup(self):
        event = self.observe()
        self.journal.received("ai", self.ai_token, event["event_id"])
        restarted = Journal(self.root)
        self.assertEqual(restarted.inbox("ai", self.ai_token), [])
        self.assertEqual(len(restarted.inbox("human", self.human_token)), 1)
        self.assertEqual(restarted.audit()["pending"], 1)

    def test_tokens_cannot_cross_recipient_boundary(self):
        event = self.observe()
        with self.assertRaises(JournalError):
            self.journal.inbox("human", self.ai_token)
        with self.assertRaises(JournalError):
            self.journal.received("human", self.ai_token, event["event_id"])
        self.assertEqual(self.journal.audit()["received"], 0)

    def test_renamed_recipient_in_token_is_not_authority(self):
        token = json.loads(self.ai_token.read_text())
        token["recipient"] = "human"
        replacement = self.base / "replacement.token"
        write_new(replacement, json.dumps(token).encode())
        with self.assertRaises(JournalError):
            self.journal.inbox("human", replacement)

    def test_tokens_cannot_cross_journals(self):
        other = Journal.create(self.base / "other-journal")
        other.register("ai", self.base / "other.token")
        with self.assertRaises(JournalError):
            other.inbox("ai", self.ai_token)

    def test_delivery_cannot_skip_unassigned_events(self):
        event = self.observe(recipients=["ai"])
        with self.assertRaises(JournalError):
            self.journal.received("human", self.human_token, event["event_id"])
        with self.assertRaises(JournalError):
            self.journal.received("ai", self.ai_token, "sha256:" + "0" * 64)
        self.assertEqual(self.journal.audit()["received"], 0)

    def test_only_token_hash_stored_in_database(self):
        token = json.loads(self.ai_token.read_text())["token"].encode()
        self.assertNotIn(token, self.journal.database.read_bytes())

    def test_registration_never_overwrites_and_token_is_not_printed(self):
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            code = main(["--journal", str(self.root), "register", "--recipient", "third",
                         "--token-file", str(self.base / "third.token")])
        self.assertEqual(code, 0)
        token = json.loads((self.base / "third.token").read_text())["token"]
        self.assertNotIn(token, stdout.getvalue())
        original = self.ai_token.read_bytes()
        with self.assertRaises(JournalError):
            self.journal.register("fourth", self.ai_token)
        self.assertEqual(self.ai_token.read_bytes(), original)

    def test_private_file_modes(self):
        self.assertEqual(self.root.stat().st_mode & 0o777, 0o700)
        self.assertEqual(self.journal.database.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.ai_token.stat().st_mode & 0o777, 0o600)
        self.ai_token.chmod(0o644)
        with self.assertRaises(JournalError):
            self.journal.inbox("ai", self.ai_token)

    def test_shared_directory_rejected(self):
        self.root.chmod(0o755)
        with self.assertRaises(JournalError):
            Journal(self.root)

    def test_shared_database_rejected(self):
        self.journal.database.chmod(0o644)
        with self.assertRaises(JournalError):
            Journal(self.root)

    def test_symlink_receipt_rejected(self):
        linked = self.base / "linked.json"
        linked.symlink_to(self.receipt)
        with self.assertRaises(JournalError):
            self.journal.observe(linked, digest(self.receipt.read_bytes()), **self.arguments)

    def test_symlink_directory_rejected(self):
        linked = self.base / "linked"
        linked.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(JournalError):
            Journal(linked)

    def test_symlink_ancestor_rejected(self):
        linked = self.base / "linked-parent"
        linked.symlink_to(self.base, target_is_directory=True)
        with self.assertRaises(JournalError):
            Journal(linked / "journal")

    def test_wrong_owner_rejected(self):
        info = self.ai_token.stat()
        wrong = SimpleNamespace(st_mode=info.st_mode, st_nlink=1, st_uid=os.getuid() + 1, st_size=info.st_size)
        with self.assertRaises(JournalError):
            check_file(wrong, 4096, True)

    def test_symlink_database_rejected(self):
        actual = self.base / "actual.sqlite3"
        self.journal.database.rename(actual)
        self.journal.database.symlink_to(actual)
        with self.assertRaises(JournalError):
            Journal(self.root)

    def test_symlink_rollback_journal_rejected(self):
        (self.root / "journal.sqlite3-journal").symlink_to(self.receipt)
        with self.assertRaises(JournalError):
            self.journal.audit()

    def test_hardlinked_database_and_token_rejected(self):
        linked = self.base / "db-copy"
        os.link(self.journal.database, linked)
        with self.assertRaises(JournalError):
            Journal(self.root)
        linked.unlink()
        os.link(self.ai_token, self.base / "token-copy")
        with self.assertRaises(JournalError):
            self.journal.inbox("ai", self.ai_token)

    def test_hardlinked_receipt_rejected(self):
        os.link(self.receipt, self.base / "receipt-copy")
        with self.assertRaises(JournalError):
            self.observe()

    def test_path_escape_rejected(self):
        with self.assertRaises(JournalError):
            Journal(self.root / ".." / "journal")
        with self.assertRaises(JournalError):
            self.observe(workstream_id="../other")
        with self.assertRaises(JournalError):
            self.journal.register("../recipient", self.base / "new.token")

    def test_bad_hash_rejected_without_partial_state(self):
        with self.assertRaises(JournalError):
            self.journal.observe(self.receipt, "sha256:" + "0" * 64, **self.arguments)
        self.assertEqual(self.journal.audit()["events"], 0)
        self.assertEqual(self.journal.audit()["deliveries"], 0)

    def test_malformed_json_rejected(self):
        cases = [b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}', b'{"x":1e999}',
                 b'[]', b'{"x":"\\ud800"}', b'{"x":' + b'[' * 33 + b'0' + b']' * 33 + b'}',
                 b'{"x":' + b'9' * 65 + b'}', b'\xff', b'{} trailing']
        for raw in cases:
            with self.subTest(raw=raw[:30]):
                with self.assertRaises(JournalError):
                    strict_json(raw)

    def test_json_depth_scanner_does_not_count_escaped_string_braces(self):
        value = {"text": "[" * 100 + '\\"' + "}" * 100}
        self.assertEqual(strict_json(json.dumps(value).encode()), value)

    def test_size_bound_and_fifo_rejected_without_blocking(self):
        oversized = self.base / "large"
        oversized.write_bytes(b" " * (MAX_RECEIPT + 1))
        with self.assertRaises(JournalError):
            read_regular(oversized)
        fifo = self.base / "fifo"
        os.mkfifo(fifo, 0o600)
        with self.assertRaises(JournalError):
            read_regular(fifo)

    def test_unknown_future_schema_rejected(self):
        with sqlite3.connect(self.journal.database) as con:
            con.execute("PRAGMA user_version=999")
        with self.assertRaises(JournalError):
            Journal(self.root)

    def test_altered_schema_objects_rejected(self):
        with sqlite3.connect(self.journal.database) as con:
            con.execute("ALTER TABLE outbox ADD COLUMN authority TEXT")
        with self.assertRaises(JournalError):
            Journal(self.root)

    def test_recipient_and_inbox_bounds(self):
        with self.assertRaises(JournalError):
            self.observe(recipients=["id" + str(i) for i in range(65)])
        with self.assertRaises(JournalError):
            self.journal.inbox("ai", self.ai_token, limit=101)
        with self.assertRaises(JournalError):
            self.journal.inbox("ai", self.ai_token, limit=True)

    def test_token_in_journal_rejected_and_secret_parent_must_be_private(self):
        with self.assertRaises(JournalError):
            self.journal.register("third", self.root / "third.token")
        shared = self.base / "shared"
        shared.mkdir(mode=0o755)
        with self.assertRaises(JournalError):
            self.journal.register("third", shared / "third.token")

    def test_foreign_database_rejected_without_initializing(self):
        other_root = self.base / "foreign"
        other_root.mkdir(mode=0o700)
        path = other_root / "journal.sqlite3"
        write_new(path, b"")
        with sqlite3.connect(path) as con:
            con.execute("CREATE TABLE private_data (value TEXT)")
            con.execute("INSERT INTO private_data VALUES ('preserve')")
        before = path.read_bytes()
        with self.assertRaises(JournalError):
            Journal(other_root)
        self.assertEqual(path.read_bytes(), before)

    def test_event_update_and_delete_rejected(self):
        self.observe()
        with sqlite3.connect(self.journal.database) as con:
            with self.assertRaises(sqlite3.IntegrityError):
                con.execute("DELETE FROM events")
            with self.assertRaises(sqlite3.IntegrityError):
                con.execute("UPDATE events SET receipt=X'00'")

    def test_audit_detects_outbox_loss(self):
        self.observe()
        with sqlite3.connect(self.journal.database) as con:
            con.execute("DELETE FROM outbox WHERE recipient='human'")
        with self.assertRaises(JournalError):
            self.journal.audit()

    def test_corrupt_delivery_timestamp_cannot_hide_pending_events(self):
        event = self.observe()
        for invalid in ("corrupt", b"corrupt", -1, 1.5, 253402300800):
            with self.subTest(timestamp=invalid):
                with sqlite3.connect(self.journal.database) as con:
                    con.execute("UPDATE outbox SET received_at=? WHERE recipient='ai'", (invalid,))
                for action in (self.journal.audit,
                               lambda: self.journal.inbox("ai", self.ai_token),
                               lambda: self.journal.received("ai", self.ai_token, event["event_id"]),
                               self.observe):
                    with self.assertRaises(JournalError):
                        action()
        with sqlite3.connect(self.journal.database) as con:
            con.execute("UPDATE outbox SET received_at=NULL")
        self.assertEqual(len(self.journal.inbox("ai", self.ai_token)), 1)

    def test_corrupt_recipient_digest_fails_closed_without_type_error(self):
        self.observe()
        for invalid in (b"\x00", "not-a-digest", "sha256:" + "z" * 64):
            with self.subTest(digest=invalid):
                with sqlite3.connect(self.journal.database) as con:
                    con.execute("UPDATE recipients SET token_hash=? WHERE id='ai'", (invalid,))
                for action in (self.journal.audit, lambda: self.journal.inbox("ai", self.ai_token)):
                    with self.assertRaises(JournalError):
                        action()

    def test_corrupt_journal_identity_fails_closed_without_type_error(self):
        for invalid in (b"\x00", "z" * 64, 1):
            with self.subTest(identity=invalid):
                with sqlite3.connect(self.journal.database) as con:
                    con.execute("UPDATE metadata SET value=?", (invalid,))
                with self.assertRaises(JournalError):
                    Journal(self.root)

    def test_unregistered_or_duplicate_recipient_rejected_atomically(self):
        for recipients in (["human", "missing"], ["human", "human"]):
            with self.assertRaises(JournalError):
                self.observe(recipients=recipients)
        self.assertEqual(self.journal.audit()["events"], 0)

    def test_no_execution_or_human_acknowledgment_command(self):
        for command in ("execute", "human_ack", "claim", "open-gate"):
            with contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as error:
                    main(["--journal", str(self.root), command])
            self.assertEqual(error.exception.code, 2)
        self.assertEqual(self.journal.audit()["events"], 0)

    def test_creation_is_exclusive(self):
        before = self.journal.database.read_bytes()
        with self.assertRaises(JournalError):
            Journal.create(self.root)
        self.assertEqual(before, self.journal.database.read_bytes())

    def _child(self, code, *extra):
        return subprocess.Popen([sys.executable, "-c", code, str(self.root), str(self.receipt),
                                 json.dumps(self.arguments), *extra], cwd=REPO,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    def test_concurrent_process_retry_inserts_one_observation(self):
        code = ("import sys,json; from pathlib import Path; from integrations.workstreams import Journal; "
                "from integrations.workstreams.safeio import digest; p=Path(sys.argv[2]); "
                "print(json.dumps(Journal(sys.argv[1]).observe(p,digest(p.read_bytes()),**json.loads(sys.argv[3]))))")
        children = [self._child(code) for _ in range(6)]
        results = []
        for child in children:
            stdout, stderr = child.communicate(timeout=30)
            self.assertEqual(child.returncode, 0, stderr)
            results.append(json.loads(stdout))
        self.assertEqual(sum(result["new"] for result in results), 1)
        self.assertEqual(len({result["event_id"] for result in results}), 1)
        self.assertEqual(self.journal.audit()["events"], 1)
        self.assertEqual(self.journal.audit()["deliveries"], 2)

    def test_process_killed_before_commit_recovers_without_partial_event(self):
        code = '''
import sys,json,time
from pathlib import Path
from contextlib import contextmanager
from integrations.workstreams import Journal
from integrations.workstreams.safeio import digest
class Interrupted(Journal):
    @contextmanager
    def _transaction(self):
        with super()._transaction() as connection:
            yield connection
            print("uncommitted", flush=True)
            time.sleep(60)
receipt=Path(sys.argv[2])
Interrupted(sys.argv[1]).observe(receipt,digest(receipt.read_bytes()),**json.loads(sys.argv[3]))
'''
        child = self._child(code)
        try:
            ready, _, _ = select.select([child.stdout], [], [], 8)
            self.assertTrue(ready, "child did not reach uncommitted transaction")
            self.assertEqual(child.stdout.readline().strip(), "uncommitted")
        finally:
            child.kill()
            child.communicate(timeout=8)
        restarted = Journal(self.root)
        self.assertEqual(restarted.audit()["events"], 0)
        self.assertEqual(restarted.audit()["deliveries"], 0)
        self.assertTrue(self.observe()["new"])
        self.assertEqual(restarted.audit()["events"], 1)


if __name__ == "__main__":
    unittest.main()
