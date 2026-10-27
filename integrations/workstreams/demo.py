"""Synthetic observer journey; private tokens and journal are deleted on exit."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import tempfile

from .journal import Journal
from .safeio import JournalError, digest, write_new


def run():
    checks = []

    def require(name, condition):
        if not condition:
            raise RuntimeError(name)
        checks.append({"check": name, "status": "pass"})

    with tempfile.TemporaryDirectory(prefix="openagents-observer-") as temporary:
        home = Path(temporary).resolve()
        home.chmod(0o700)
        journal = Journal.create(home / "journal")
        journal.register("human-inbox", home / "human.token")
        journal.register("ai-inbox", home / "ai.token")
        require("distinct delivery capabilities", (home / "human.token").read_bytes() !=
                (home / "ai.token").read_bytes())
        receipt = home / "receipt.json"
        write_new(receipt, b'{"schema":"synthetic.test.v1","status":"pass"}\n')
        kwargs = dict(workstream_id="synthetic-observer", source_host="producer-host",
                      source_session="synthetic-session", owner_last_reported="owner",
                      gate_reported="paused", next_step_proposed="Inspect the synthetic receipt",
                      recipients=["human-inbox", "ai-inbox"])
        result = journal.observe(receipt, digest(receipt.read_bytes()), **kwargs)
        require("first observation retained", result["new"])
        repeated = journal.observe(receipt, digest(receipt.read_bytes()), **kwargs)
        require("retry deduplicated", not repeated["new"] and result["event_id"] == repeated["event_id"])
        journal = Journal(home / "journal")
        require("restart retains two recipients", journal.audit()["pending"] == 2)
        items = journal.inbox("ai-inbox", home / "ai.token")
        require("paused gate remains reported paused", items[0]["observation"]["gate_reported"] == "paused")
        require("receipt claims remain unverified", items[0]["observation"]["assurances"]["receipt_claims"] == "unverified")
        delivery = journal.received("ai-inbox", home / "ai.token", result["event_id"])
        require("delivery grants no human awareness", delivery["human_awareness"] == "unknown")
        require("delivery retry idempotent", delivery == journal.received(
            "ai-inbox", home / "ai.token", result["event_id"]))
        require("offline human inbox catches up", len(journal.inbox("human-inbox", home / "human.token")) == 1)
        try:
            journal.received("human-inbox", home / "ai.token", result["event_id"])
        except JournalError:
            checks.append({"check": "AI capability cannot receipt another inbox", "status": "pass"})
        else:
            raise RuntimeError("cross-recipient capability accepted")
        require("no execution authority", result["assurances"]["execution_authorized"] is False)
        audit = journal.audit()
        require("journal and outbox integrity", audit["status"] == "pass" and audit["events"] == 1)
    return {"schema": "openagents.workstream.demo-receipt.v1", "status": "pass",
            "synthetic_only": True, "checks": checks, "audit": audit,
            "network_calls": 0, "notifications_sent": 0, "jobs_launched": 0,
            "human_acknowledgments": 0, "private_state": "temporary; deleted on exit"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, help="New output file in an existing private directory")
    args = parser.parse_args()
    result = run()
    if args.receipt:
        write_new(args.receipt, json.dumps(result, indent=2).encode() + b"\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
