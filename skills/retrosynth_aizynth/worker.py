#!/usr/bin/env python3
"""Phase 0 worker placeholder."""

import json
import sys


def main() -> int:
    payload = sys.stdin.read()
    print(json.dumps({"ok": True, "received_bytes": len(payload)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
