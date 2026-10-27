#!/usr/bin/env python3
"""Local PoX scope and verified federation-reference sidecar. No network calls."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import sys
import tempfile
import time

from source import (BridgeError, PoxSource, canonical, read_credential,
                    read_regular, sha256, strict_json)

MAX_PACKAGE = 48 * 1024 * 1024
APPLICATION_ID = 0x504F5846


def write_new(path, raw):
    """No overwrite; permissions are private from the instant of creation."""
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
    except OSError:
        raise BridgeError("output_exists_or_unavailable") from None


def verify_package(binary, package_bytes, policy_bytes, evidence_bytes=None):
    """Run the actual Rust verifier on immutable staged inputs, not a supplied receipt."""
    binary = Path(binary).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="pox-federation-verify-") as directory:
        directory = Path(directory)
        package, policy = directory / "package.json", directory / "policy.json"
        write_new(package, package_bytes)
        write_new(policy, policy_bytes)
        command = [str(binary), "verify", "--package", str(package), "--policy", str(policy)]
        if evidence_bytes is not None:
            evidence = directory / "evidence.json"
            write_new(evidence, evidence_bytes)
            command.extend(("--evidence", str(evidence)))
        with tempfile.TemporaryFile() as output:
            try:
                result = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=output,
                                        stderr=subprocess.DEVNULL, timeout=30, check=False)
            except (OSError, subprocess.TimeoutExpired):
                raise BridgeError("federation_verifier_unavailable") from None
            if result.returncode != 0:
                raise BridgeError("federation_verification_rejected")
            output.seek(0)
            raw = output.read(65537)
            if len(raw) > 65536:
                raise BridgeError("federation_verifier_output_invalid")
    receipt = strict_json(raw)
    if (not isinstance(receipt, dict)
            or receipt.get("schema") != "openagents.federation.verification.v1"
            or receipt.get("statement_type") not in ("evidence", "review")
            or receipt.get("policy_sha256") != sha256(policy_bytes)
            or receipt.get("trust_state") != "local_snapshot"
            or receipt.get("scientific_review") != "not_established"
            or receipt.get("publication_authorized") is not False
            or not isinstance(receipt.get("content_hash"), str)
            or not re.fullmatch(r"blake3:[a-f0-9]{64}", receipt["content_hash"])):
        raise BridgeError("federation_verifier_output_invalid")
    if (receipt["statement_type"] == "review") != (evidence_bytes is not None):
        raise BridgeError("review_requires_exact_evidence")
    return receipt


class ReferenceStore:
    """Append-only local reference log; not the PoX delivery or authorization DB."""

    def __init__(self, path):
        self.path = Path(path).absolute()
        if not self.path.exists():
            try:
                fd = os.open(self.path, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
                os.close(fd)
            except FileExistsError:
                pass
        # Also validates no final symlink, owner-only permissions, or oversized file.
        read_regular(self.path, 64 * 1024 * 1024, secret=True)
        connection = self.connect()
        try:
            connection.execute("BEGIN IMMEDIATE")
            tables = {x[0] for x in connection.execute("SELECT name FROM sqlite_master WHERE type='table'")}
            application_id = connection.execute("PRAGMA application_id").fetchone()[0]
            if tables and application_id != APPLICATION_ID:
                raise BridgeError("sidecar_not_a_federation_store")
            connection.execute("""CREATE TABLE IF NOT EXISTS federation_references (
                record_id TEXT PRIMARY KEY, package_sha256 TEXT NOT NULL,
                evidence_sha256 TEXT NOT NULL, policy_sha256 TEXT NOT NULL,
                scope_sha256 TEXT NOT NULL, receipt TEXT NOT NULL,
                UNIQUE(package_sha256,evidence_sha256,policy_sha256,scope_sha256))""")
            connection.execute("""CREATE TRIGGER IF NOT EXISTS no_reference_update
                BEFORE UPDATE ON federation_references BEGIN
                SELECT RAISE(ABORT,'immutable_federation_reference'); END""")
            connection.execute("""CREATE TRIGGER IF NOT EXISTS no_reference_delete
                BEFORE DELETE ON federation_references BEGIN
                SELECT RAISE(ABORT,'immutable_federation_reference'); END""")
            connection.execute(f"PRAGMA application_id={APPLICATION_ID}")
            connection.execute("PRAGMA user_version=1")
            connection.commit()
        finally:
            connection.close()

    def connect(self):
        connection = sqlite3.connect(self.path, timeout=10)
        connection.execute("PRAGMA synchronous=FULL")
        return connection

    def append(self, receipt):
        key = (receipt["package_sha256"], receipt["evidence_sha256"] or "",
               receipt["policy_sha256"], receipt["source_snapshot_sha256"])
        connection = self.connect()
        try:
            connection.execute("BEGIN IMMEDIATE")
            existing = connection.execute("""SELECT receipt FROM federation_references
                WHERE package_sha256=? AND evidence_sha256=? AND policy_sha256=? AND scope_sha256=?""", key).fetchone()
            if existing:
                return json.loads(existing[0])
            record_id = sha256(canonical(receipt))
            receipt = {"record_id": record_id, **receipt}
            connection.execute("INSERT INTO federation_references VALUES (?,?,?,?,?,?)",
                               (record_id, *key, canonical(receipt).decode("utf-8")))
            connection.commit()
            return receipt
        finally:
            connection.close()


def record_reference(source, inquiry_id, credential, binary, package_path,
                     policy_path, sidecar_path, evidence_path=None):
    """Fresh cryptographic and local-scope checks also apply to exact retries."""
    if (Path(sidecar_path).absolute() == source.database
            or (Path(sidecar_path).exists() and os.path.samefile(sidecar_path, source.database))):
        raise BridgeError("sidecar_must_be_separate")
    initial = source.scope(inquiry_id, credential)
    package = read_regular(package_path, MAX_PACKAGE)
    policy = read_regular(policy_path)
    evidence = read_regular(evidence_path, MAX_PACKAGE) if evidence_path else None
    verification = verify_package(binary, package, policy, evidence)
    # No source transaction is held during subprocess work. Re-read authorization,
    # scope, participant status and accepted handoff immediately before recording.
    current = source.scope(inquiry_id, credential)
    if initial != current:
        raise BridgeError("pox_scope_changed_during_verification")
    if verification.get("scope") != current["scope"]:
        raise BridgeError("pox_scope_mismatch")
    receipt = {
        "schema": "pox.federation-reference.v1",
        "checked_at_unix": int(time.time()),
        "source_snapshot": current,
        "source_snapshot_sha256": sha256(canonical(current)),
        "package_sha256": sha256(package),
        "evidence_sha256": sha256(evidence) if evidence is not None else None,
        "policy_sha256": sha256(policy),
        "verification": verification,
        "applicability": "checked_at_local_snapshot_only",
        "scientific_admission": False,
        "distribution_authorized": False,
        "holder_delivery_authorized": False,
        "publication_authorized": False,
    }
    return ReferenceStore(sidecar_path).append(receipt)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pox-source", required=True, type=Path,
                        help="Installed PoX data-inquiry source directory matching source-pins.json")
    parser.add_argument("--database", required=True, type=Path, help="Existing PoX SQLite v4 database (opened read-only)")
    parser.add_argument("--members", required=True, type=Path, help="Explicit PoX staff to federation human ID mapping")
    parser.add_argument("--credential-file", required=True, type=Path, help="0600 file holding the current coordinator's PoX credential")
    parser.add_argument("--inquiry", required=True)
    commands = parser.add_subparsers(dest="command", required=True)
    export = commands.add_parser("scope", help="Export current accepted scope without contact details or scope text")
    export.add_argument("--out", required=True, type=Path, help="New private JSON scope file for federation policy.current_scope")
    export.add_argument("--receipt-out", type=Path, help="Optional new private PoX authentication/source snapshot receipt")
    record = commands.add_parser("record", help="Verify a signed package and append a local reference; grants no access")
    record.add_argument("--federation-bin", required=True, type=Path)
    record.add_argument("--package", required=True, type=Path)
    record.add_argument("--policy", required=True, type=Path)
    record.add_argument("--evidence", type=Path, help="Exact evidence package required for a review")
    record.add_argument("--sidecar", required=True, type=Path)
    args = parser.parse_args()
    try:
        credential = read_credential(args.credential_file)
        members = strict_json(read_regular(args.members))
        with PoxSource(args.pox_source, args.database, members) as source:
            if args.command == "scope":
                receipt = source.scope(args.inquiry, credential)
                write_new(args.out, canonical(receipt["scope"]) + b"\n")
                if args.receipt_out:
                    write_new(args.receipt_out, canonical(receipt) + b"\n")
                print(json.dumps({"status": "scope_exported", "trust_state": "local_snapshot"}))
            else:
                receipt = record_reference(source, args.inquiry, credential, args.federation_bin,
                                           args.package, args.policy, args.sidecar, args.evidence)
                print(json.dumps(receipt, sort_keys=True))
        return 0
    except (BridgeError, OSError, ValueError) as error:
        code = str(error) if isinstance(error, BridgeError) else "local_io_error"
        print(json.dumps({"status": "rejected", "code": code}), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
