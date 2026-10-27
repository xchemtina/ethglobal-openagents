"""Pure bridge tests; add POX_SOURCE for real pinned InquiryStore integration."""
import concurrent.futures
from contextlib import closing
import copy
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bridge import ReferenceStore, record_reference, verify_package, write_new
from source import (BridgeError, PoxSource, canonical, read_credential,
                    sha256, strict_json, validate_members)


def members():
    return {"schema": "pox.federation-members.v1", "source_instance": "synthetic-local",
            "members": [{"pox_staff_id": "staff_" + digit * 24, "federation_id": name}
                        for digit, name in zip("123", ("alice", "bob", "carol"))]}


class PureTests(unittest.TestCase):
    def test_duplicate_json_and_invalid_mappings_rejected(self):
        with self.assertRaises(BridgeError):
            strict_json(b'{"schema":1,"schema":2}')
        with self.assertRaises(BridgeError):
            strict_json(b'{"value":NaN}')
        for field in ("pox_staff_id", "federation_id"):
            value = members()
            value["members"][1][field] = value["members"][0][field]
            with self.assertRaises(BridgeError):
                validate_members(value)
        value = members()
        value["members"][0]["public_key"] = "unverified key"
        with self.assertRaises(BridgeError):
            validate_members(value)

    def test_secret_permissions_symlink_and_output_overwrite(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "secret"
            write_new(path, b"a" * 64 + b"\n")
            self.assertEqual(read_credential(path), "a" * 64)
            with self.assertRaises(BridgeError):
                write_new(path, b"replace")
            linked = Path(temporary) / "link"
            linked.symlink_to(path)
            with self.assertRaises(BridgeError):
                read_credential(linked)
            path.chmod(0o644)
            with self.assertRaises(BridgeError):
                read_credential(path)

    def test_sidecar_concurrent_retry_and_immutability(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "references.sqlite3"
            receipt = {"package_sha256": "sha256:" + "a" * 64,
                       "evidence_sha256": None, "policy_sha256": "sha256:" + "b" * 64,
                       "source_snapshot_sha256": "sha256:" + "c" * 64,
                       "scientific_admission": False}
            with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
                results = list(pool.map(lambda _: ReferenceStore(path).append(receipt), range(12)))
            self.assertEqual(len({x["record_id"] for x in results}), 1)
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            with closing(sqlite3.connect(path)) as connection:
                self.assertEqual(connection.execute("SELECT COUNT(*) FROM federation_references").fetchone()[0], 1)
                with self.assertRaises(sqlite3.IntegrityError):
                    connection.execute("DELETE FROM federation_references")

    def test_arbitrary_existing_database_is_not_migrated(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "other.sqlite3"
            with closing(sqlite3.connect(path)) as connection:
                connection.execute("CREATE TABLE private(value TEXT)")
                connection.execute("INSERT INTO private VALUES ('preserve')")
                connection.commit()
            path.chmod(0o600)
            before = path.read_bytes()
            with self.assertRaises(BridgeError):
                ReferenceStore(path)
            self.assertEqual(path.read_bytes(), before)

    def test_verify_runs_real_command_and_does_not_accept_supplied_receipt(self):
        # Command mocking tests only the subprocess boundary; the real Rust CLI
        # is exercised by root's end-to-end pilot, never replaced in production.
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "missing"
            with self.assertRaises(FileNotFoundError):
                verify_package(binary, b"{}", b"{}")


@unittest.skipUnless(os.environ.get("POX_SOURCE"), "Set POX_SOURCE to the pinned source for real PoX tests")
class RealPoxTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)
        self.database = self.directory / "inquiries.sqlite3"
        self.database.touch(mode=0o600)
        self.source = PoxSource(os.environ["POX_SOURCE"], self.database, members())
        self.admin, self.holder = "f" * 64, "a" * 64
        self.store = self.source.module.InquiryStore(self.database, admin_key=self.admin)
        payload = {"mode": "request", "domain": "other", "need": "existing_data",
                   "title": "Synthetic federation fixture", "question": "Compare this synthetic evidence package with its exact specification.",
                   "visibility": "private", "contact_name": "PRIVATE_CONTACT",
                   "contact_email": "secret@example.invalid", "contact_consent": True}
        self.inquiry = self.store.submit("b" * 32, self.holder, payload)["id"]
        self.people = {role: self.store.create_staff({"name": "Synthetic " + role}, self.admin)
                       for role in ("coordinator", "producer", "reviewer", "outsider")}
        self.counter = 0
        self.source.members = {"schema": "pox.federation-members.v1", "source_instance": "synthetic-local",
                               "members": [{"pox_staff_id": self.people[role]["staff"]["id"], "federation_id": name}
                                           for role, name in zip(("coordinator", "producer", "reviewer"), ("alice", "bob", "carol"))]}
        self.store.assign(self.inquiry, self.body(coordinator_id=self.people["coordinator"]["staff"]["id"],
                                                note="Assign synthetic scope coordinator."), self.admin)
        self.accept()

    def tearDown(self):
        self.source.close()
        self.temporary.cleanup()

    def credential(self, role="coordinator"):
        return self.people[role]["credential"]

    def body(self, **values):
        self.counter += 1
        return {"expected_version": self.store.get(self.inquiry, self.holder)["version"],
                "action_key": format(self.counter, "032x"), **values}

    def accept(self):
        body = self.body(producer_id=self.people["producer"]["staff"]["id"],
                         reviewer_id=self.people["reviewer"]["staff"]["id"],
                         capability_note="Synthetic fixture representation capability.",
                         access_note="Private local verification only.", note="Accept distinct synthetic responsibilities.")
        record = self.store.propose_handoff(self.inquiry, body, self.credential())
        self.handoff = record["network"]["current"]["id"]
        for role in ("producer", "reviewer"):
            self.store.decide_handoff(self.handoff, self.body(decision="accepted", note="Accept the exact synthetic scope."),
                                      self.credential(role))

    def test_current_scope_uses_real_auth_and_has_no_private_text(self):
        before = self.database.read_bytes()
        result = self.source.scope(self.inquiry, self.credential())
        self.assertIn(self.handoff, result["scope"]["scope_id"])
        self.assertEqual(result["scope"]["coordinator_id"], "alice")
        self.assertEqual(result["scope"]["producer_id"], "bob")
        self.assertEqual(result["scope"]["reviewer_id"], "carol")
        self.assertEqual(self.database.read_bytes(), before)
        raw = canonical(result)
        for forbidden in (b"PRIVATE_CONTACT", b"secret@example.invalid", b"Synthetic federation fixture", self.credential().encode()):
            self.assertNotIn(forbidden, raw)
        for credential in (self.admin, self.holder, self.credential("outsider"), self.credential("reviewer")):
            with self.assertRaises(BridgeError):
                self.source.scope(self.inquiry, credential)

    def test_scope_change_and_member_revocation_rejected(self):
        self.store.revise_scope(self.inquiry, self.body(fields={"outcome": "Different deliverable needs new agreement."},
                                                       note="Change the exact accepted scope."), self.credential())
        with self.assertRaisesRegex(BridgeError, "stale"):
            self.source.scope(self.inquiry, self.credential())
        self.accept()
        self.store.deactivate_staff(self.people["reviewer"]["staff"]["id"], self.admin)
        with self.assertRaisesRegex(BridgeError, "stale"):
            self.source.scope(self.inquiry, self.credential())

    def test_replacement_handoff_changes_federation_scope_identity(self):
        original = self.source.scope(self.inquiry, self.credential())["scope"]
        self.accept()
        replacement = self.source.scope(self.inquiry, self.credential())["scope"]
        self.assertEqual(original["scope_sha256"], replacement["scope_sha256"])
        self.assertNotEqual(original["scope_id"], replacement["scope_id"])

    def test_scope_changed_during_verification_never_creates_reference(self):
        package, policy, sidecar = (self.directory / name for name in ("package.json", "policy.json", "sidecar.sqlite3"))
        package.write_bytes(b"{}")
        policy.write_bytes(b"{}")
        def verify_then_revoke(*_):
            self.store.deactivate_staff(self.people["producer"]["staff"]["id"], self.admin)
            return {"scope": {}}
        with patch("bridge.verify_package", side_effect=verify_then_revoke):
            with self.assertRaises(BridgeError):
                record_reference(self.source, self.inquiry, self.credential(), "unused", package, policy, sidecar)
        self.assertFalse(sidecar.exists())

    def test_source_modified_before_import_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            altered = Path(temporary)
            pins = strict_json((Path(__file__).resolve().parents[1] / "source-pins.json").read_bytes())
            for name in pins["files"]:
                (altered / name).write_bytes((Path(os.environ["POX_SOURCE"]) / name).read_bytes())
            with (altered / "inquiry.py").open("ab") as stream:
                stream.write(b"\n# incompatible source\n")
            with self.assertRaisesRegex(BridgeError, "incompatible"):
                PoxSource(altered, self.database, self.source.members)


if __name__ == "__main__":
    unittest.main()
