"""ChimiaClaw 0G upload wrapper.

Reads a `ZeroGUploadRequest` JSON document on stdin, performs the upload via
the operator-installed `0g-storage-client` binary (or a deterministic stub for
local development), and emits a `ZeroGUploadReceipt` JSON document on stdout.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Sequence

ENV_BINARY = "ZEROG_BINARY"
ENV_STUB = "ZEROG_STUB"
DEFAULT_BINARY = "0g-storage-client"


@dataclass
class UploadRequest:
    file_path: Path
    network: str
    rpc_url: str
    indexer_url: str
    private_key_env: str
    expected_replica: int
    finality_required: bool

    @classmethod
    def from_dict(cls, raw: dict) -> "UploadRequest":
        try:
            return cls(
                file_path=Path(raw["file_path"]).expanduser(),
                network=str(raw.get("network", "0g-galileo-turbo")),
                rpc_url=str(raw["rpc_url"]),
                indexer_url=str(raw["indexer_url"]),
                private_key_env=str(raw.get("private_key_env", "ZEROG_PRIVATE_KEY")),
                expected_replica=int(raw.get("expected_replica", 1)),
                finality_required=bool(raw.get("finality_required", False)),
            )
        except KeyError as error:
            raise SystemExit(f"upload request missing required field: {error}") from error


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="zerog-upload",
        description="Upload a payload file to 0G Storage and emit a receipt JSON.",
    )
    parser.add_argument(
        "--metadata-file",
        help="Read metadata from a file instead of stdin (mostly useful for tests).",
    )
    return parser.parse_args(argv)


def _read_request(args: argparse.Namespace) -> UploadRequest:
    if args.metadata_file:
        raw = Path(args.metadata_file).read_text(encoding="utf-8")
    else:
        raw = sys.stdin.read()
    if not raw.strip():
        sys.stderr.write("no upload metadata received on stdin\n")
        raise SystemExit(2)
    try:
        document = json.loads(raw)
    except json.JSONDecodeError as error:
        sys.stderr.write(f"upload metadata is not valid JSON: {error}\n")
        raise SystemExit(2) from error
    return UploadRequest.from_dict(document)


def _resolve_private_key(request: UploadRequest, *, allow_missing: bool) -> str:
    key = os.environ.get(request.private_key_env)
    if key:
        return key
    if allow_missing:
        return ""
    sys.stderr.write(
        f"private key not set in {request.private_key_env}; refusing to upload.\n"
    )
    raise SystemExit(2)


def _stub_receipt(request: UploadRequest) -> dict:
    if not request.file_path.is_file():
        sys.stderr.write(f"stub mode: file not found: {request.file_path}\n")
        raise SystemExit(2)
    digest = hashlib.blake2b(request.file_path.read_bytes(), digest_size=32).hexdigest()
    fake_root = f"0x{digest}"
    fake_tx = f"0x{digest[:64]}"
    return {
        "network": request.network,
        "indexer_url": request.indexer_url,
        "root_hashes": [fake_root],
        "tx_hashes": [fake_tx],
        "uploaded_at_unix": int(time.time()),
        "audit_notes": [
            "STUB MODE: no real 0G upload was performed.",
            f"file: {request.file_path}",
            f"file_blake2b_32: {digest}",
            f"network: {request.network}",
            "Set ZEROG_STUB=0 (or unset) and provide ZEROG_BINARY to run a real upload.",
        ],
    }


_HEX_LINE = re.compile(r"0x[0-9a-fA-F]{16,}")


def _extract_hashes(stdout_text: str) -> tuple[List[str], List[str]]:
    """Best-effort extraction of merkle root and tx hash from the binary's stdout.

    The official `0g-storage-client` CLI prints both values on labelled lines.
    We accept either ``root: 0x...`` or ``Root hash: 0x...`` and the same for
    ``tx``. If the binary changes its output format the operator can replace
    this wrapper without touching the Rust adapter.
    """
    roots: List[str] = []
    txs: List[str] = []
    for line in stdout_text.splitlines():
        lower = line.lower()
        candidates = _HEX_LINE.findall(line)
        if not candidates:
            continue
        if "root" in lower and "hash" in lower:
            roots.extend(candidates)
        elif lower.strip().startswith("root") or lower.strip().startswith("merkle"):
            roots.extend(candidates)
        elif "tx" in lower or "transaction" in lower:
            txs.extend(candidates)
    return roots, txs


def _real_upload(request: UploadRequest, private_key: str) -> dict:
    binary = os.environ.get(ENV_BINARY, DEFAULT_BINARY)
    if not shutil.which(binary):
        sys.stderr.write(
            f"could not find 0G binary {binary!r} on PATH; install it or set {ENV_BINARY}.\n"
        )
        raise SystemExit(2)
    cmd = [
        binary,
        "upload",
        "--url",
        request.rpc_url,
        "--indexer",
        request.indexer_url,
        "--file",
        str(request.file_path),
        "--key",
        private_key,
        "--expected-replica",
        str(request.expected_replica),
    ]
    if request.finality_required:
        cmd.append("--finality-required")
    try:
        proc = subprocess.run(
            cmd,
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stderr or str(error))
        raise SystemExit(2) from error
    roots, txs = _extract_hashes(proc.stdout + "\n" + (proc.stderr or ""))
    if not roots:
        sys.stderr.write(
            "could not extract a root hash from 0g-storage-client output; "
            "set ZEROG_STUB=1 to use the stub uploader, or update the parser.\n"
        )
        raise SystemExit(2)
    return {
        "network": request.network,
        "indexer_url": request.indexer_url,
        "root_hashes": roots,
        "tx_hashes": txs,
        "uploaded_at_unix": int(time.time()),
        "audit_notes": [
            f"binary: {binary}",
            f"file: {request.file_path}",
            f"network: {request.network}",
            f"expected_replica: {request.expected_replica}",
            f"finality_required: {request.finality_required}",
        ],
    }


def main(argv: Iterable[str] | None = None) -> int:
    args = parse_args(list(argv) if argv is not None else sys.argv[1:])
    request = _read_request(args)
    stub = os.environ.get(ENV_STUB, "").strip().lower() in {"1", "true", "yes"}
    if stub:
        receipt = _stub_receipt(request)
    else:
        private_key = _resolve_private_key(request, allow_missing=False)
        receipt = _real_upload(request, private_key)
    json.dump(receipt, sys.stdout, separators=(",", ":"))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
