#!/usr/bin/env python3
"""Check reviewed source identities; this is not a test suite or security audit."""
import hashlib
import json
from pathlib import Path

here = Path(__file__).resolve().parent
root = here.parents[2]
manifest = json.loads((here / "SOURCE_MANIFEST.json").read_text())
failures = []
for item in manifest["files"]:
    path = root / item["path"]
    if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != item["sha256"]:
        failures.append(item["path"])
if failures:
    raise SystemExit("Source identity mismatch: " + ", ".join(failures))
print(f"Verified {len(manifest['files'])} source files; no scientific or deployment claim.")
