#!/usr/bin/env python3
"""Synthetic signed-attempt journey using the actual private observer journal.

Two local directories simulate independent installations. No scientific engine,
network, human review, or execution authorization is exercised. Fresh random
keys, journals and packages are temporary and deleted on exit. A create-only
optional receipt retains public outcomes only.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from integrations.workstreams import Journal
from integrations.workstreams.safeio import canonical

FIXTURE = ROOT / "crates/chimiaclaw-posyn/fixtures/synthetic/bundle"
EXPECTED_ROOT = "blake3:a48f37a331a0949452b6b3535ff7da81cc28ab89e9b2860c13256369404411cd"
FAILURE_FIXTURE = ROOT / "demo/federation/fixtures/terminal-no-results"
FAILURE_ROOT = "blake3:f8bd1eee8d1820c91a92a82d8f54c7227992fac822076f75bd833ebc18d04790"


def digest(raw):
    return "sha256:" + hashlib.sha256(raw).hexdigest()


def write(path, raw):
    with path.open("xb") as out:
        os.chmod(path, 0o600)
        out.write(raw)


def write_json(path, data):
    write(path, json.dumps(data, sort_keys=True, indent=2).encode() + b"\n")


def run(binary):
    checks = []

    def cli(label, *args, accepted=True):
        result = subprocess.run([str(binary), *map(str, args)], text=True,
                                capture_output=True, timeout=30, check=False)
        if (result.returncode == 0) != accepted:
            raise RuntimeError(f"{label}: unexpected return {result.returncode}: {result.stderr[:250]}")
        checks.append({"check": label, "status": "pass", "expected": "accept" if accepted else "reject"})
        return json.loads(result.stdout) if accepted else None

    with tempfile.TemporaryDirectory(prefix="openagents-signed-attempt-") as temp:
        base = Path(temp).resolve()
        producer, receiver = base / "producer", base / "receiver"
        producer.mkdir(mode=0o700); receiver.mkdir(mode=0o700)
        identities = []
        for name, kind in [("coordinator", "human"), ("producer", "human"),
                           ("reviewer", "human"), ("producer-ai", "ai")]:
            identities.append(cli(f"initialize synthetic {name}", "init", "--id", name,
                                  "--kind", kind, "--key", producer / (name + ".key")))
        stamp = int(time.time())
        scope = {"scope_id": "synthetic:inquiry:handoff-1", "revision": 1,
                 "scope_sha256": digest(b"synthetic agreed scope"), "coordinator_id": "coordinator",
                 "producer_id": "producer", "reviewer_id": "reviewer"}
        policy = {"schema": "openagents.federation.policy.v1", "policy_id": "synthetic-attempt",
                  "observed_at_unix": stamp, "expires_at_unix": stamp + 3600,
                  "current_scope": scope, "expected_posyn_root": EXPECTED_ROOT,
                  "trusted_identities": identities, "revoked_keys": [], "revoked_delegations": []}
        for home in (producer, receiver):
            write_json(home / "policy.json", policy)
        cli("bounded AI evidence delegation", "delegate", "--policy", producer / "policy.json",
            "--key", producer / "producer.key", "--delegate-id", "producer-ai",
            "--expires-at", stamp + 1800, "--out", producer / "delegation.json")
        journal = Journal.create(producer / "journal")
        journal.register("producer-ai", producer / "inbox.token")
        receive_journal = Journal.create(receiver / "journal")
        receive_journal.register("reviewer-ai", receiver / "inbox.token")
        summaries = []
        for outcome in ("succeeded", "timed_out", "failed"):
            left, right = producer / outcome, receiver / outcome
            left.mkdir(mode=0o700); right.mkdir(mode=0o700)
            no_output = outcome == "failed"
            fixture = FAILURE_FIXTURE if no_output else FIXTURE
            expected_root = FAILURE_ROOT if no_output else EXPECTED_ROOT
            manifest = json.loads((fixture / "manifest.json").read_text())
            result = next(a for a in manifest["payload"]["artifacts"] if a["role"] == "result")
            identity = {"name": result["name"], "blake3": result["digest"], "size_bytes": result["size_bytes"]}
            case_policy = copy.deepcopy(policy)
            case_policy["expected_posyn_root"] = expected_root
            for home in (left, right):
                write_json(home / "policy.json", case_policy)
            receipt = {"schema": "openagents.terminal-attempt.receipt.v1", "scope": scope,
                       "job_id": "synthetic-job", "attempt_id": "attempt-1" if no_output else "attempt-" + outcome,
                       "authorization_sha256": digest(b"synthetic work-order reference; no execution grant"),
                       "inputs_sha256": digest(b"synthetic input manifest"),
                       "environment_sha256": digest(b"synthetic runtime identity"),
                       "started_at_unix": stamp, "finished_at_unix": stamp,
                       "outcome": outcome,
                       "failure_code": "missing_output" if no_output else "task_timeout" if outcome == "timed_out" else None,
                       "outputs": [] if no_output else [identity],
                       "missing_outputs": ["result.json"] if no_output else ["complete.json"] if outcome == "timed_out" else [],
                       "retained_failure_record": identity if no_output else None}
            write_json(left / "receipt.json", receipt)
            raw = (left / "receipt.json").read_bytes()
            observation_args = dict(workstream_id=receipt["job_id"], source_host="synthetic-producer",
                                    source_session=receipt["attempt_id"], owner_last_reported="producer",
                                    gate_reported="paused", next_step_proposed="Review the exact synthetic terminal outcome",
                                    recipients=["producer-ai"])
            event = journal.observe(left / "receipt.json", digest(raw), **observation_args)
            entry = next(x for x in journal.inbox("producer-ai", producer / "inbox.token")
                         if x["event_id"] == event["event_id"])
            event_bytes = canonical(entry["observation"])
            assert digest(event_bytes) == event["event_id"]
            write(left / "event.json", event_bytes)
            duplicate = Journal(producer / "journal").observe(left / "receipt.json", digest(raw), **observation_args)
            assert duplicate["event_id"] == event["event_id"] and not duplicate["new"]
            checks.append({"check": outcome + " real observer event and restart deduplication", "status": "pass"})
            expected = {"schema": "openagents.terminal-attempt.expectation.v1", "job_id": receipt["job_id"],
                        "attempt_id": receipt["attempt_id"], "authorization_sha256": receipt["authorization_sha256"],
                        "inputs_sha256": receipt["inputs_sha256"], "environment_sha256": receipt["environment_sha256"],
                        "event_sha256": digest(event_bytes), "receipt_sha256": digest(raw),
                        "expected_outputs": ["result.json"] if no_output else ["result.json", *receipt["missing_outputs"]],
                        "not_before_unix": stamp, "expires_at_unix": stamp + 1800}
            # The common synthetic test harness is the independent trust source;
            # real recipients must establish these pins through their own channel.
            for home in (left, right):
                write_json(home / "expectation.json", expected)
            cli(outcome + " sign original fixture evidence", "import-posyn", "--bundle", fixture,
                "--expect", expected_root, "--key", producer / "producer-ai.key", "--policy", left / "policy.json",
                "--delegation", producer / "delegation.json", "--audience", "synthetic", "--out", left / "evidence.json")

            def context(home):
                return ["--policy", home / "policy.json", "--expect-attempt", home / "expectation.json",
                        "--evidence", home / "evidence.json", "--receipt", home / "receipt.json", "--event", home / "event.json"]

            signed = cli(outcome + " bind exact terminal attempt", "bind-attempt", *context(left),
                         "--key", producer / "producer-ai.key", "--out", left / "attempt.json")
            for name in ("receipt.json", "event.json", "evidence.json", "attempt.json"):
                write(right / name, (left / name).read_bytes())
            checked = cli(outcome + " recipient independently verifies", "verify-attempt", *context(right),
                          "--package", right / "attempt.json")
            assert checked["outcome"] == outcome and checked["content_hash"] == signed["content_hash"]
            assert not checked["execution_authorized"] and checked["execution"] == "not_verified"
            assert checked["human_awareness"] == "unknown" and checked["scientific_review"] == "not_established"
            write_json(right / "verification.json", checked)
            verified_raw = (right / "verification.json").read_bytes()
            observe_args = dict(workstream_id=receipt["job_id"], source_host="synthetic-receiver",
                                source_session=receipt["attempt_id"], owner_last_reported="reviewer",
                                gate_reported="paused", next_step_proposed="Await human review; no execution authority",
                                recipients=["reviewer-ai"])
            observed = receive_journal.observe(right / "verification.json", digest(verified_raw), **observe_args)
            repeated = Journal(receiver / "journal").observe(right / "verification.json", digest(verified_raw), **observe_args)
            assert observed["event_id"] == repeated["event_id"] and not repeated["new"]
            delivery = receive_journal.received("reviewer-ai", receiver / "inbox.token", observed["event_id"])
            assert delivery["human_awareness"] == "unknown"
            checks.append({"check": outcome + " retained verification retry deduplicates and AI delivery grants no human awareness", "status": "pass"})
            cli(outcome + " cannot overwrite signed package", "bind-attempt", *context(left),
                "--key", producer / "producer-ai.key", "--out", left / "attempt.json", accepted=False)
            bad = json.loads((right / "attempt.json").read_text())
            bad["artifact"]["signature"] = "00" * 64
            write_json(right / "bad-signature.json", bad)
            cli(outcome + " altered signature rejected", "verify-attempt", *context(right),
                "--package", right / "bad-signature.json", accepted=False)
            changed = copy.deepcopy(expected); changed["attempt_id"] += "-replacement"
            write_json(right / "wrong-attempt.json", changed)
            wrong_context = context(right)
            wrong_context[wrong_context.index("--expect-attempt") + 1] = right / "wrong-attempt.json"
            cli(outcome + " old package rejected for different expected attempt", "verify-attempt", *wrong_context,
                "--package", right / "attempt.json", accepted=False)
            summaries.append({"outcome": outcome, "content_hash": checked["content_hash"],
                              "event_sha256": checked["event_sha256"], "receipt_sha256": checked["receipt_sha256"],
                              "produced_outputs": len(receipt["outputs"]), "missing_outputs": receipt["missing_outputs"],
                              "retained_failure_record": no_output,
                              "scientific_review": checked["scientific_review"], "execution_authorized": False})
        audits = {"producer": journal.audit(), "receiver": receive_journal.audit()}
        assert audits["producer"]["events"] == audits["receiver"]["events"] == 3
        return {"schema": "openagents.terminal-attempt.demo.v1", "status": "pass", "checks": checks,
                "outcomes": summaries, "observer_audits": audits, "synthetic_only": True,
                "installations": "two temporary directories on one host", "network_calls": 0,
                "scientific_execution": False, "human_review": "not_established",
                "private_working_state": "deleted on exit"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/chimiaclaw-federation")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    receipt = run(args.binary.resolve())
    if args.receipt:
        write_json(args.receipt, receipt)
    print(json.dumps({"status": receipt["status"], "checks": len(receipt["checks"]),
                      "outcomes": [x["outcome"] for x in receipt["outcomes"]]}, indent=2))


if __name__ == "__main__":
    main()
