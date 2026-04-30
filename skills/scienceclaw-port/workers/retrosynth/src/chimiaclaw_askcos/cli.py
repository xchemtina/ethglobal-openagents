"""ASKCOS template-relevance retrosynthesis client.

The worker is intentionally tiny: it forwards a SMILES query to a user-managed
ASKCOS endpoint, normalizes the response into a stable JSON shape, and exits.
The Rust adapter (`chimiaclaw-retrosynth-askcos`) signs the result as a
`chem.retrosynth.template_suggestions` artifact.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Iterable, List, Sequence

from . import _cache

DEFAULT_TEMPLATE_SETS = (
    "reaxys",
    "pistachio",
    "pistachio_ringbreaker",
    "bkms_metabolic",
    "reaxys_biocatalysis",
)

ENV_ENDPOINT = "CHIMIACLAW_ASKCOS_ENDPOINT"


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="askcos-retro",
        description="Submit a SMILES target to a user-managed ASKCOS endpoint.",
    )
    parser.add_argument(
        "--smiles",
        help="Target SMILES. If omitted, the worker reads it from stdin.",
    )
    parser.add_argument(
        "--endpoint",
        help=(
            f"ASKCOS base URL (e.g. http://duck.olympus.local:9410). "
            f"Defaults to ${ENV_ENDPOINT}."
        ),
    )
    parser.add_argument(
        "--template-set",
        action="append",
        dest="template_sets",
        help=(
            "Template set to query; may be passed multiple times. "
            f"Defaults to {','.join(DEFAULT_TEMPLATE_SETS)}."
        ),
    )
    parser.add_argument(
        "--top-k",
        type=int,
        default=20,
        help="Number of ranked precursor suggestions to keep per set (default: 20).",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=30.0,
        help="HTTP timeout per request (default: 30 seconds).",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=2025,
        help="Pass-through seed recorded in provenance for reproducibility.",
    )
    parser.add_argument(
        "--cache-dir",
        help=(
            "Directory used as a content-hashed cache for ASKCOS responses. "
            "Defaults to $CHIMIACLAW_ASKCOS_CACHE_DIR or ~/.cache/chimiaclaw/askcos."
        ),
    )
    parser.add_argument(
        "--no-cache",
        action="store_true",
        help="Bypass the disk cache for this invocation (still writes to it on success).",
    )
    parser.add_argument(
        "--cache-only",
        action="store_true",
        help="Refuse to call the endpoint; succeed only on a cache hit, otherwise exit non-zero.",
    )
    return parser.parse_args(argv)


def _resolve_smiles(args: argparse.Namespace) -> str:
    smiles = args.smiles
    if smiles is None:
        smiles = sys.stdin.read().strip()
    if not smiles:
        sys.stderr.write("no SMILES provided on stdin or --smiles\n")
        raise SystemExit(2)
    return smiles


def _resolve_endpoint(args: argparse.Namespace) -> str:
    endpoint = args.endpoint or os.environ.get(ENV_ENDPOINT)
    if not endpoint:
        sys.stderr.write(
            f"no ASKCOS endpoint configured; set {ENV_ENDPOINT} or pass --endpoint.\n"
        )
        raise SystemExit(2)
    return endpoint.rstrip("/")


def _resolve_cache_dir(args: argparse.Namespace) -> Path:
    if args.cache_dir:
        return Path(args.cache_dir).expanduser().resolve()
    return _cache.default_cache_dir()


def _request_template_relevance(
    httpx_module,
    endpoint: str,
    smiles: str,
    template_sets: Sequence[str],
    top_k: int,
    timeout_seconds: float,
) -> List[dict]:
    proposals: List[dict] = []
    url = f"{endpoint}/api/v2/template-relevance/"
    with httpx_module.Client(timeout=timeout_seconds) as client:
        for template_set in template_sets:
            payload = {
                "smiles": smiles,
                "template_set": template_set,
                "top_k": top_k,
            }
            try:
                response = client.post(url, json=payload)
                response.raise_for_status()
                body = response.json()
            except httpx_module.HTTPError as error:
                sys.stderr.write(
                    f"ASKCOS request failed for template set {template_set!r}: {error}\n"
                )
                raise SystemExit(2) from error
            proposals.append(
                {
                    "template_set": template_set,
                    "request": payload,
                    "response": body,
                }
            )
    return proposals


def main(argv: Iterable[str] | None = None) -> int:
    args = parse_args(list(argv) if argv is not None else sys.argv[1:])
    smiles = _resolve_smiles(args)
    endpoint = _resolve_endpoint(args)
    template_sets = tuple(args.template_sets) if args.template_sets else DEFAULT_TEMPLATE_SETS
    cache_dir = _resolve_cache_dir(args)
    cache_key = _cache.derive_cache_key(
        endpoint=endpoint,
        target_smiles=smiles,
        template_sets=template_sets,
        top_k=args.top_k,
    )
    cache_path = cache_dir.joinpath(cache_key[:2], f"{cache_key}.json")
    cache_record: _cache.CacheRecord

    proposals: List[dict] = []
    if not args.no_cache:
        cached = _cache.load_cached_proposals(cache_dir, cache_key)
        if cached is not None:
            proposals = cached
            cache_record = _cache.CacheRecord(hit=True, key=cache_key, path=cache_path)

    if not proposals:
        if args.cache_only:
            sys.stderr.write(
                f"--cache-only set but no cached entry at {cache_path}.\n"
            )
            raise SystemExit(2)
        try:
            import httpx  # type: ignore[import-not-found]
        except ModuleNotFoundError as error:
            sys.stderr.write(
                "httpx is not importable. Install via uv: `uv pip install httpx` "
                "(or run this worker through `uvx --from <path> askcos-retro`).\n"
            )
            raise SystemExit(2) from error
        proposals = _request_template_relevance(
            httpx,
            endpoint=endpoint,
            smiles=smiles,
            template_sets=template_sets,
            top_k=args.top_k,
            timeout_seconds=args.timeout_seconds,
        )
        if not args.no_cache:
            cache_path = _cache.store_cached_proposals(
                cache_dir=cache_dir,
                key=cache_key,
                proposals=proposals,
                target_smiles=smiles,
                template_sets=template_sets,
                top_k=args.top_k,
                endpoint=endpoint,
                written_at_unix=int(time.time()),
            )
        cache_record = _cache.CacheRecord(hit=False, key=cache_key, path=cache_path)

    payload = {
        "schema_tag": "chem.retrosynth.template_suggestions",
        "target_smiles": smiles,
        "endpoint": endpoint,
        "template_sets": list(template_sets),
        "top_k": args.top_k,
        "seed": args.seed,
        "proposals": proposals,
        "provenance": {
            "source_kind": "askcos-template-relevance",
            "source_ref": "skills/scienceclaw-port/workers/retrosynth::askcos-retro",
            "notes": [
                f"endpoint: {endpoint}",
                f"template_sets: {','.join(template_sets)}",
                f"seed: {args.seed}",
                f"top_k: {args.top_k}",
                f"cache_hit: {cache_record.hit}",
                f"cache_key: {cache_record.key}",
                f"cache_path: {cache_record.path}",
                "ChimiaClaw refuses to sign ASKCOS results without an explicit "
                "operator-confirmed endpoint configuration.",
            ],
        },
        "cache": cache_record.to_dict(),
    }
    json.dump(payload, sys.stdout, separators=(",", ":"))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
