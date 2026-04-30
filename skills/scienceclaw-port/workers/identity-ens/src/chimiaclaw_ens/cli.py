"""Publish ChimiaClaw text records onto an ENS name.

The worker is intentionally narrow:
- one ENS name per invocation;
- one or more `key=value` text records to publish;
- a private key that owns (or is the manager of) the ENS name on the chain
  identified by ``ENS_WRITE_RPC_URL``;
- idempotent re-runs (records already at the desired value are reported as
  ``unchanged``, never re-published).
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from dataclasses import dataclass
from typing import Iterable, List, Sequence

ENV_RPC = "ENS_WRITE_RPC_URL"
ENV_PRIVATE_KEY = "ENS_WRITE_PRIVATE_KEY"
SCHEMA_TAG = "identity.ens.publication"


@dataclass
class RecordSpec:
    key: str
    value: str


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="ens-publish-text-records",
        description=(
            "Publish ChimiaClaw text records onto an ENS name owned by the "
            "configured account, idempotently."
        ),
    )
    parser.add_argument(
        "--ens",
        required=True,
        help="ENS name to publish records on (e.g. dft.service.chimiadao.eth).",
    )
    parser.add_argument(
        "--record",
        action="append",
        dest="records",
        default=[],
        metavar="KEY=VALUE",
        help="Text record to publish. May be passed multiple times.",
    )
    parser.add_argument(
        "--rpc-url",
        help=f"RPC URL with write capability. Defaults to ${ENV_RPC}.",
    )
    parser.add_argument(
        "--resolver",
        help=(
            "Resolver address to write to. Defaults to whatever the registry "
            "currently returns for this name."
        ),
    )
    parser.add_argument(
        "--gas-buffer",
        type=float,
        default=1.20,
        help="Multiplier applied to estimated gas (default: 1.20).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Resolve current state and report intended changes without sending transactions.",
    )
    parser.add_argument(
        "--allow-mainnet",
        action="store_true",
        help=(
            "By default the worker refuses chain id 1 (mainnet); pass this flag "
            "to opt in explicitly."
        ),
    )
    args = parser.parse_args(argv)
    if not args.records:
        parser.error("at least one --record key=value is required")
    return args


def parse_records(raw: Iterable[str]) -> List[RecordSpec]:
    parsed: List[RecordSpec] = []
    for entry in raw:
        if "=" not in entry:
            raise SystemExit(f"--record must be key=value, got {entry!r}")
        key, value = entry.split("=", 1)
        key = key.strip()
        value = value.strip()
        if not key:
            raise SystemExit(f"--record key cannot be empty: {entry!r}")
        parsed.append(RecordSpec(key=key, value=value))
    return parsed


def _resolve_rpc_url(args: argparse.Namespace) -> str:
    rpc = args.rpc_url or os.environ.get(ENV_RPC)
    if not rpc:
        sys.stderr.write(f"missing RPC URL; pass --rpc-url or set {ENV_RPC}\n")
        raise SystemExit(2)
    return rpc


def _resolve_private_key() -> str:
    key = os.environ.get(ENV_PRIVATE_KEY)
    if not key:
        sys.stderr.write(
            f"missing private key; set {ENV_PRIVATE_KEY} (never pass it on the command line)\n"
        )
        raise SystemExit(2)
    return key


def _import_web3():
    try:
        from web3 import Web3  # type: ignore[import-not-found]
        from eth_account import Account  # type: ignore[import-not-found]
    except ModuleNotFoundError as error:
        sys.stderr.write(
            "web3 / eth_account are not importable. Install via uv: "
            "`uv sync --project skills/scienceclaw-port/workers/identity-ens`.\n"
        )
        raise SystemExit(2) from error
    return Web3, Account


def _attach_signing_middleware(w3, account) -> None:
    """Attach the local-account signer middleware in a web3.py-version-tolerant way."""
    try:  # web3.py >= 7
        from web3.middleware import SignAndSendRawMiddlewareBuilder  # type: ignore

        w3.middleware_onion.inject(
            SignAndSendRawMiddlewareBuilder.build(account), layer=0
        )
        return
    except ImportError:
        pass
    try:  # web3.py 6.x
        from web3.middleware import construct_sign_and_send_raw_middleware  # type: ignore

        w3.middleware_onion.add(construct_sign_and_send_raw_middleware(account))
        return
    except ImportError as error:  # pragma: no cover - defensive
        sys.stderr.write("could not load any signing middleware from web3.py\n")
        raise SystemExit(2) from error


def _ens_owner(ens, name: str) -> str:
    try:
        return ens.owner(name)
    except Exception as error:  # pragma: no cover - defensive
        sys.stderr.write(f"failed to read ENS owner for {name!r}: {error}\n")
        raise SystemExit(2) from error


def _existing_text(ens, name: str, key: str) -> str:
    try:
        value = ens.get_text(name, key)
    except AttributeError:  # pragma: no cover - older web3 names
        value = ens.text(name, key)
    return value or ""


def _publish_record(
    w3,
    ens,
    account,
    name: str,
    record: RecordSpec,
    gas_buffer: float,
    dry_run: bool,
) -> dict:
    current = _existing_text(ens, name, record.key)
    if current == record.value:
        return {
            "key": record.key,
            "value": record.value,
            "status": "unchanged",
            "previous_value": current,
        }
    if dry_run:
        return {
            "key": record.key,
            "value": record.value,
            "status": "dry-run",
            "previous_value": current,
        }
    try:
        tx_hash = ens.set_text(
            name,
            record.key,
            record.value,
            transact={"from": account.address},
        )
    except Exception as error:
        return {
            "key": record.key,
            "value": record.value,
            "status": "failed",
            "previous_value": current,
            "error": str(error),
        }
    receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
    return {
        "key": record.key,
        "value": record.value,
        "status": "published",
        "previous_value": current,
        "tx_hash": receipt.transactionHash.hex(),
        "block_number": int(receipt.blockNumber),
        "gas_used": int(receipt.gasUsed),
        "effective_gas_price": int(receipt.get("effectiveGasPrice", 0)),
    }


def main(argv: Iterable[str] | None = None) -> int:
    args = parse_args(list(argv) if argv is not None else sys.argv[1:])
    records = parse_records(args.records)
    rpc_url = _resolve_rpc_url(args)
    private_key = _resolve_private_key()
    Web3, Account = _import_web3()
    account = Account.from_key(private_key)
    w3 = Web3(Web3.HTTPProvider(rpc_url))
    if not w3.is_connected():
        sys.stderr.write(f"failed to connect to RPC {rpc_url}\n")
        raise SystemExit(2)
    chain_id = int(w3.eth.chain_id)
    if chain_id == 1 and not args.allow_mainnet:
        sys.stderr.write(
            "refusing to publish on chain id 1 (mainnet); pass --allow-mainnet to opt in.\n"
        )
        raise SystemExit(2)
    _attach_signing_middleware(w3, account)
    w3.eth.default_account = account.address

    ens = w3.ens
    if ens is None:  # pragma: no cover - web3 always exposes ENS
        sys.stderr.write("web3.ens is unavailable; cannot publish ENS records.\n")
        raise SystemExit(2)
    owner = _ens_owner(ens, args.ens)
    if (owner or "").lower() != account.address.lower():
        sys.stderr.write(
            f"account {account.address} is not the registry owner of {args.ens} "
            f"(current owner: {owner or '<unset>'})\n"
        )
        raise SystemExit(2)

    started_at = int(time.time())
    record_results = [
        _publish_record(
            w3=w3,
            ens=ens,
            account=account,
            name=args.ens,
            record=record,
            gas_buffer=args.gas_buffer,
            dry_run=args.dry_run,
        )
        for record in records
    ]

    payload = {
        "schema_tag": SCHEMA_TAG,
        "ens_name": args.ens,
        "chain_id": chain_id,
        "rpc_url": rpc_url,
        "controller_address": account.address,
        "registry_owner": owner,
        "records": record_results,
        "started_at_unix": started_at,
        "completed_at_unix": int(time.time()),
        "dry_run": bool(args.dry_run),
        "provenance": {
            "source_kind": "ens-publisher-setText",
            "source_ref": (
                "skills/scienceclaw-port/workers/identity-ens::ens-publish-text-records"
            ),
            "notes": [
                f"chain_id: {chain_id}",
                f"controller: {account.address}",
                f"records_attempted: {len(record_results)}",
                f"published: {sum(1 for r in record_results if r['status'] == 'published')}",
                f"unchanged: {sum(1 for r in record_results if r['status'] == 'unchanged')}",
                f"failed: {sum(1 for r in record_results if r['status'] == 'failed')}",
                "Private key was read from ENS_WRITE_PRIVATE_KEY; never passed as argv.",
            ],
        },
    }
    json.dump(payload, sys.stdout, separators=(",", ":"))
    sys.stdout.write("\n")
    if any(record["status"] == "failed" for record in record_results):
        return 3
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
