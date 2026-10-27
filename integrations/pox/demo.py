#!/usr/bin/env python3
"""Real PoX + Rust federation smoke test, always in disposable synthetic state."""
from __future__ import annotations

import argparse
import copy
import json
import os
from pathlib import Path
import secrets
import sqlite3
import subprocess
import tempfile
import time

from bridge import record_reference, write_new
from source import BridgeError, PoxSource, canonical, sha256, strict_json


def run(binary, *arguments, reject=False):
    result = subprocess.run([str(binary), *map(str, arguments)], capture_output=True,
                            stdin=subprocess.DEVNULL, timeout=30, check=False)
    if reject:
        if result.returncode == 0:
            raise AssertionError("expected_federation_rejection")
        return None
    if result.returncode != 0:
        raise AssertionError("federation_command_failed")
    return strict_json(result.stdout) if result.stdout.strip() else None


def reject(function, label, checks):
    try:
        function()
    except BridgeError:
        checks.append({"check": label, "result": "rejected"})
    else:
        raise AssertionError("expected_bridge_rejection_" + label)


def database_digest(database):
    """Include committed WAL state, not only the main SQLite file's bytes."""
    connection = sqlite3.connect(database.absolute().as_uri() + "?mode=ro", uri=True)
    try:
        return sha256("\n".join(connection.iterdump()).encode("utf-8"))
    finally:
        connection.close()


def demonstrate(pox_source, binary):
    binary = binary.resolve(strict=True)
    repository = Path(__file__).resolve().parents[2]
    fixture = repository / "crates/chimiaclaw-posyn/fixtures/synthetic"
    root = strict_json((fixture / "UPSTREAM_CHECK.json").read_bytes())["verification"]["attestation_hash"]
    checks = []
    with tempfile.TemporaryDirectory(prefix="openagents-pox-synthetic-") as temporary:
        directory = Path(temporary)
        database = directory / "inquiries.sqlite3"
        database.touch(mode=0o600)
        initial_members = {"schema": "pox.federation-members.v1", "source_instance": "synthetic-pilot",
                           "members": [{"pox_staff_id": "staff_" + digit * 24, "federation_id": name}
                                       for digit, name in zip("123", ("alice", "bob", "carol"))]}
        with PoxSource(pox_source, database, initial_members) as source:
            # This constructor touches only the empty temporary synthetic database.
            admin, holder = secrets.token_hex(32), secrets.token_hex(32)
            store = source.module.InquiryStore(database, admin_key=admin)
            inquiry = store.submit(secrets.token_hex(16), holder, {
                "mode": "request", "domain": "other", "need": "existing_data",
                "title": "Synthetic federation integration",
                "question": "Does this synthetic evidence bundle retain its exact specification and refusal records?",
                "visibility": "private", "contact_consent": True,
                "contact_name": "Synthetic fixture holder",
                "contact_email": "fixture@example.invalid",
            })["id"]
            people = {name: store.create_staff({"name": "Synthetic " + name}, admin)
                      for name in ("alice", "bob", "carol")}
            source.members = {"schema": "pox.federation-members.v1", "source_instance": "synthetic-pilot",
                              "members": [{"pox_staff_id": people[name]["staff"]["id"], "federation_id": name}
                                          for name in people]}
            credential = people["alice"]["credential"]

            def body(**fields):
                return {"expected_version": store.get(inquiry, holder)["version"],
                        "action_key": secrets.token_hex(16), **fields}

            store.assign(inquiry, body(coordinator_id=people["alice"]["staff"]["id"],
                                       note="Assign synthetic coordinator."), admin)

            def accept():
                record = store.propose_handoff(inquiry, body(
                    producer_id=people["bob"]["staff"]["id"], reviewer_id=people["carol"]["staff"]["id"],
                    capability_note="Synthetic evidence verification only.",
                    access_note="Local private fixture; no real scientific data.",
                    note="Propose exact-scope synthetic roles."), credential)
                handoff = record["network"]["current"]["id"]
                for name in ("bob", "carol"):
                    store.decide_handoff(handoff, body(decision="accepted", note="Accept the synthetic scope responsibility."),
                                         people[name]["credential"])

            accept()
            scope_receipt = source.scope(inquiry, credential)
            source_pin = scope_receipt["source_manifest_sha256"]
            write_new(directory / "scope.json", canonical(scope_receipt["scope"]) + b"\n")
            identities = []
            for name in people:
                for identity, kind in ((name, "human"), (name + "-ai", "ai")):
                    identities.append(run(binary, "init", "--id", identity, "--kind", kind,
                                          "--key", directory / (identity + ".key")))
            now = int(time.time())
            policy = {"schema": "openagents.federation.policy.v1", "policy_id": "synthetic-pox-pilot",
                      "observed_at_unix": now, "expires_at_unix": now + 3600,
                      "current_scope": scope_receipt["scope"], "expected_posyn_root": root,
                      "trusted_identities": identities, "revoked_keys": [], "revoked_delegations": []}
            policy_path, evidence, review, delegation = (directory / name for name in (
                "policy.json", "evidence.json", "review.json", "delegation.json"))
            write_new(policy_path, canonical(policy) + b"\n")
            run(binary, "delegate", "--key", directory / "bob.key", "--policy", policy_path,
                "--delegate-id", "bob-ai", "--expires-at", now + 1800, "--out", delegation)
            run(binary, "import-posyn", "--bundle", fixture / "bundle", "--expect", root,
                "--key", directory / "bob-ai.key", "--policy", policy_path, "--delegation", delegation,
                "--audience", "synthetic", "--out", evidence)
            run(binary, "review", "--package", evidence, "--key", directory / "carol.key",
                "--policy", policy_path, "--decision", "reviewed", "--out", review)
            corrections = directory / "corrections.json"
            run(binary, "review", "--package", evidence, "--key", directory / "carol.key",
                "--policy", policy_path, "--decision", "corrections_required", "--out", corrections)
            checks.append({"check": "three_human_three_ai_signed_exchange", "result": "passed"})
            sidecar = directory / "federation-references.sqlite3"
            before = database_digest(database)
            imported = record_reference(source, inquiry, credential, binary, evidence, policy_path, sidecar)
            reviewed = record_reference(source, inquiry, credential, binary, review, policy_path, sidecar, evidence)
            corrected = record_reference(source, inquiry, credential, binary, corrections, policy_path, sidecar, evidence)
            assert imported["verification"]["signer_id"] == "bob-ai"
            assert reviewed["verification"]["signer_id"] == "carol"
            assert reviewed["verification"]["review_decision"] == "reviewed"
            assert corrected["verification"]["review_decision"] == "corrections_required"
            assert not reviewed["scientific_admission"]
            assert record_reference(source, inquiry, credential, binary, evidence, policy_path, sidecar) == imported
            assert database_digest(database) == before
            checks.extend({"check": name, "result": "passed"} for name in (
                "real_pox_scope_authenticated", "verified_evidence_and_review_recorded",
                "corrections_required_preserved", "exact_retry_idempotent", "pox_database_unchanged_by_adapter"))
            tampered = strict_json(evidence.read_bytes())
            tampered["artifact"]["signature"] = "00" * 64
            tampered_path = directory / "tampered.json"
            write_new(tampered_path, canonical(tampered) + b"\n")
            reject(lambda: record_reference(source, inquiry, credential, binary, tampered_path, policy_path, sidecar),
                   "tampered_signature", checks)
            for identity, package, parent in (("bob-ai", evidence, None), ("carol", review, evidence)):
                revoked = copy.deepcopy(policy)
                revoked["revoked_keys"] = [next(x["public_key"] for x in identities if x["id"] == identity)]
                revoked_path = directory / (identity + "-revoked.json")
                write_new(revoked_path, canonical(revoked) + b"\n")
                reject(lambda: record_reference(source, inquiry, credential, binary, package, revoked_path, sidecar, parent),
                       "revoked_" + identity, checks)
            for identity in ("carol-ai", "bob"):
                run(binary, "review", "--package", evidence, "--key", directory / (identity + ".key"),
                    "--policy", policy_path, "--decision", "reviewed", "--out", directory / (identity + "-invalid-review.json"),
                    reject=True)
                checks.append({"check": "review_authority_" + identity, "result": "rejected"})
            original_members = source.members
            source.members = copy.deepcopy(original_members)
            source.members["members"][1]["federation_id"], source.members["members"][2]["federation_id"] = "carol", "bob"
            reject(lambda: record_reference(source, inquiry, credential, binary, evidence, policy_path, sidecar),
                   "changed_human_account_mapping", checks)
            source.members = original_members
            accept()
            reject(lambda: record_reference(source, inquiry, credential, binary, evidence, policy_path, sidecar),
                   "replaced_handoff", checks)
            store.revise_scope(inquiry, body(fields={"outcome": "Changed synthetic deliverable."},
                                             note="Change accepted scope for rejection control."), credential)
            reject(lambda: record_reference(source, inquiry, credential, binary, evidence, policy_path, sidecar),
                   "stale_scope", checks)
            connection = sqlite3.connect(sidecar)
            try:
                assert connection.execute("SELECT COUNT(*) FROM federation_references").fetchone()[0] == 3
            finally:
                connection.close()
    return {"schema": "pox.federation-demo.v1", "synthetic": True,
            "source_manifest_sha256": source_pin, "expected_posyn_root": root,
            "checks": checks, "passed": len(checks), "failed": 0,
            "source_database": "temporary_synthetic_only", "blockchain_transactions": 0,
            "trust_state": "local_snapshot", "scientific_admission": False,
            "publication_authorized": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pox-source", required=True, type=Path)
    parser.add_argument("--federation-bin", required=True, type=Path)
    parser.add_argument("--receipt-out", type=Path)
    args = parser.parse_args()
    try:
        receipt = demonstrate(args.pox_source, args.federation_bin)
        if args.receipt_out:
            write_new(args.receipt_out, canonical(receipt) + b"\n")
        print(json.dumps(receipt, indent=2))
        return 0
    except (BridgeError, OSError, ValueError, AssertionError, subprocess.TimeoutExpired):
        print(json.dumps({"status": "failed", "detail": "synthetic_pox_demo_failed"}))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
