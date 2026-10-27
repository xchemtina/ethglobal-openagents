#!/usr/bin/env python3
"""Exercise three isolated local installations using synthetic evidence only.

Requires the built chimiaclaw-federation CLI. All private keys and working
packages live in a temporary directory and are deleted on exit. The optional
receipt contains only check names, public artifact references and fixture roots.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "crates/chimiaclaw-posyn/fixtures/synthetic/bundle"
# Deliberately pinned independently of the manifest supplied to the importer.
EXPECTED_ROOT = "blake3:a48f37a331a0949452b6b3535ff7da81cc28ab89e9b2860c13256369404411cd"


def write_json(path: Path, value):
    with path.open("x", encoding="utf-8") as output:
        os.chmod(path, 0o600)
        json.dump(value, output, indent=2)
        output.write("\n")


def run(binary: Path):
    checks = []

    def cli(label, *args, accepted=True):
        result = subprocess.run([str(binary), *map(str, args)], capture_output=True,
                                text=True, timeout=30, check=False)
        if (result.returncode == 0) != accepted:
            raise RuntimeError(f"{label}: unexpected exit {result.returncode}: {result.stderr[:300]}")
        checks.append({"check": label, "status": "pass", "expected": "accept" if accepted else "reject"})
        return json.loads(result.stdout) if accepted and result.stdout.strip() else None

    with tempfile.TemporaryDirectory(prefix="openagents-three-person-") as temporary:
        base = Path(temporary)
        homes = {role: base / role for role in ("coordinator", "producer", "reviewer")}
        identities = []
        for role, home in homes.items():
            home.mkdir(mode=0o700)
            for kind, name in (("human", role), ("ai", role + "-ai")):
                identity = cli(f"generate independent {name} key", "init", "--id", name,
                               "--kind", kind, "--key", home / (kind + ".key"))
                identities.append(identity)
        if len({identity["public_key"] for identity in identities}) != 6:
            raise RuntimeError("identity key collision")
        by_id = {identity["id"]: identity for identity in identities}
        stamp = int(time.time())
        scope = {"scope_id": "synthetic:three-person-pilot:handoff-1", "revision": 1,
                 "scope_sha256": "sha256:" + hashlib.sha256(b"synthetic scope v1").hexdigest(),
                 "coordinator_id": "coordinator", "producer_id": "producer", "reviewer_id": "reviewer"}
        policy = {"schema": "openagents.federation.policy.v1", "policy_id": "synthetic-local-policy-1",
                  "observed_at_unix": stamp, "expires_at_unix": stamp + 3600,
                  "current_scope": scope, "trusted_identities": identities,
                  "revoked_keys": [], "revoked_delegations": [], "expected_posyn_root": EXPECTED_ROOT}
        for home in homes.values():
            write_json(home / "policy.json", policy)
        producer, reviewer, coordinator = (homes[name] for name in ("producer", "reviewer", "coordinator"))
        delegation = producer / "delegation.json"
        cli("human delegates evidence signing to own AI", "delegate", "--key", producer / "human.key",
            "--policy", producer / "policy.json", "--delegate-id", "producer-ai",
            "--expires-at", stamp + 1800, "--out", delegation)
        evidence = producer / "evidence.json"
        cli("delegated AI imports expected-root PoSyn bundle", "import-posyn", "--bundle", FIXTURE,
            "--expect", EXPECTED_ROOT, "--key", producer / "ai.key", "--policy", producer / "policy.json",
            "--delegation", delegation, "--audience", "synthetic", "--out", evidence)
        for role in ("reviewer", "coordinator"):
            received = homes[role] / "received-evidence.json"
            shutil.copyfile(evidence, received)
            received.chmod(0o600)
            cli(f"{role} verifies received evidence with own policy", "verify", "--package", received,
                "--policy", homes[role] / "policy.json")
        review = reviewer / "review.json"
        cli("named human reviews exact received evidence", "review", "--package", reviewer / "received-evidence.json",
            "--key", reviewer / "human.key", "--policy", reviewer / "policy.json", "--decision", "reviewed", "--out", review)
        shutil.copyfile(review, coordinator / "received-review.json")
        (coordinator / "received-review.json").chmod(0o600)
        receipt = cli("coordinator verifies human review bound to evidence", "verify", "--package",
                      coordinator / "received-review.json", "--evidence", coordinator / "received-evidence.json",
                      "--policy", coordinator / "policy.json")
        cli("reviewer AI cannot replace human review", "review", "--package", reviewer / "received-evidence.json",
            "--key", reviewer / "ai.key", "--policy", reviewer / "policy.json", "--decision", "reviewed",
            "--out", reviewer / "forbidden-review.json", accepted=False)
        cli("coordinator cannot impersonate reviewer", "review", "--package", coordinator / "received-evidence.json",
            "--key", coordinator / "human.key", "--policy", coordinator / "policy.json", "--decision", "reviewed",
            "--out", coordinator / "forbidden-review.json", accepted=False)
        cli("AI cannot issue human delegation", "delegate", "--key", producer / "ai.key",
            "--policy", producer / "policy.json", "--delegate-id", "reviewer-ai", "--expires-at", stamp + 100,
            "--out", producer / "forbidden-delegation.json", accepted=False)
        cli("key creation cannot overwrite existing secret", "init", "--id", "replacement", "--kind", "human",
            "--key", producer / "human.key", accepted=False)

        def reject_policy(label, mutation):
            changed = copy.deepcopy(policy)
            mutation(changed)
            path = base / (label.replace(" ", "-") + ".json")
            write_json(path, changed)
            cli(label, "verify", "--package", evidence, "--policy", path, accepted=False)

        reject_policy("stale scope rejected", lambda p: p["current_scope"].update(revision=2))
        reject_policy("replacement handoff rejected", lambda p: p["current_scope"].update(scope_id="synthetic:handoff-2"))
        reject_policy("revoked AI key rejected", lambda p: p["revoked_keys"].append(by_id["producer-ai"]["public_key"]))
        reject_policy("revoked human delegator rejected", lambda p: p["revoked_keys"].append(by_id["producer"]["public_key"]))
        reject_policy("untrusted AI key rejected", lambda p: p.update(trusted_identities=[i for i in identities if i["id"] != "producer-ai"]))
        reject_policy("expired local policy rejected", lambda p: p.update(observed_at_unix=stamp - 100, expires_at_unix=stamp - 1))
        reject_policy("different pinned PoSyn root rejected", lambda p: p.update(expected_posyn_root="blake3:" + "0" * 64))
        altered = json.loads(evidence.read_text())
        signature = altered["artifact"]["signature"]
        altered["artifact"]["signature"] = ("1" if signature[0] == "0" else "0") + signature[1:]
        write_json(base / "tampered.json", altered)
        cli("tampered package signature rejected", "verify", "--package", base / "tampered.json",
            "--policy", coordinator / "policy.json", accepted=False)
        human_evidence = producer / "human-evidence.json"
        cli("human producer can import the same source bundle", "import-posyn", "--bundle", FIXTURE,
            "--expect", EXPECTED_ROOT, "--key", producer / "human.key", "--policy", producer / "policy.json",
            "--audience", "synthetic", "--out", human_evidence)
        cli("review cannot be moved to another evidence package", "verify", "--package", review,
            "--evidence", human_evidence, "--policy", coordinator / "policy.json", accepted=False)
        for home in homes.values():
            if os.name == "posix":
                for key in home.glob("*.key"):
                    if key.stat().st_mode & 0o077:
                        raise RuntimeError("secret permissions are not owner-only")
            if len(list(home.glob("*.key"))) != 2:
                raise RuntimeError("an installation received another person's private key")
        checks.append({"check": "six private keys remain isolated with owner-only permissions", "status": "pass", "expected": "isolate"})
        return {"schema": "openagents.federation.demo-receipt.v1", "status": "pass", "synthetic_only": True,
                "installations": 3, "human_identities": 3, "ai_identities": 3, "posyn_expected_root": EXPECTED_ROOT,
                "checks": checks, "review_verification": receipt, "network_calls": 0, "blockchain_transactions": 0,
                "private_working_state": "temporary; deleted on exit", "trust_scope": "local policy snapshots only"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/chimiaclaw-federation")
    parser.add_argument("--receipt", type=Path, help="New output path; never overwrites an existing receipt")
    args = parser.parse_args()
    receipt = run(args.binary.resolve())
    if args.receipt:
        write_json(args.receipt, receipt)
    print(json.dumps({"status": receipt["status"], "checks": len(receipt["checks"]),
                      "installations": 3, "human_identities": 3, "ai_identities": 3}, indent=2))


if __name__ == "__main__":
    main()
