#!/usr/bin/env python3
"""Verify the source identities bound to this integration validation receipt."""
import hashlib
import json
from pathlib import Path


def main():
    directory = Path(__file__).resolve().parent
    root = directory.parents[2]
    manifest = json.loads((directory / "SOURCE_MANIFEST.json").read_text())
    failures = []
    for name, expected in manifest["files"].items():
        relative = Path(name)
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError("unsafe source manifest path")
        path = root / relative
        if path.is_symlink() or not path.is_file():
            failures.append(name)
        elif hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            failures.append(name)
    print(json.dumps({"status": "fail" if failures else "pass", "files": len(manifest["files"]),
                      "mismatches": failures}, indent=2))
    return bool(failures)


if __name__ == "__main__":
    raise SystemExit(main())
