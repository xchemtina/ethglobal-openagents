"""Content-hashed disk cache for ASKCOS template-relevance responses.

The cache key is derived from the inputs that should make two calls
interchangeable: the endpoint, the target SMILES, the sorted template-set
list, and the top-k value. Provenance fields like the seed are intentionally
excluded so that two callers with different reproducibility seeds still hit
the same cache entry.

Each cache entry is a small JSON file containing only the worker's
``proposals`` payload plus a schema version and a write-timestamp. The
worker re-emits a fresh response on every invocation that splices in the
cached proposals; this keeps cached files small and lets us refresh the
top-level provenance, seed, and cache record with each run.
"""
from __future__ import annotations

import hashlib
import json
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional

CACHE_SCHEMA_VERSION = 1
ENV_CACHE_DIR = "CHIMIACLAW_ASKCOS_CACHE_DIR"
DEFAULT_CACHE_SUBPATH = ".cache/chimiaclaw/askcos"


def default_cache_dir() -> Path:
    """Return ``~/.cache/chimiaclaw/askcos`` (or the override env var)."""
    override = os.environ.get(ENV_CACHE_DIR)
    if override:
        return Path(override).expanduser().resolve()
    return Path.home().joinpath(DEFAULT_CACHE_SUBPATH).resolve()


def derive_cache_key(
    endpoint: str,
    target_smiles: str,
    template_sets: Iterable[str],
    top_k: int,
) -> str:
    """Return a stable hex digest derived from the inputs that affect API output.

    Sorted template-set list keeps the key stable under user reordering of
    ``--template-set`` flags. The endpoint is normalised by trimming the
    trailing slash so ``http://duck/`` and ``http://duck`` collide as expected.
    """
    canonical = json.dumps(
        {
            "endpoint": endpoint.rstrip("/"),
            "target_smiles": target_smiles.strip(),
            "template_sets": sorted(template_sets),
            "top_k": int(top_k),
            "schema_version": CACHE_SCHEMA_VERSION,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
    digest = hashlib.blake2b(canonical.encode("utf-8"), digest_size=16).hexdigest()
    return digest


@dataclass
class CacheRecord:
    hit: bool
    key: str
    path: Path

    def to_dict(self) -> Dict[str, Any]:
        return {"hit": self.hit, "key": self.key, "path": str(self.path)}


def _entry_path(cache_dir: Path, key: str) -> Path:
    # Shard by the first two characters so we don't accumulate a single huge
    # directory once the cache grows.
    return cache_dir.joinpath(key[:2], f"{key}.json")


def load_cached_proposals(cache_dir: Path, key: str) -> Optional[List[dict]]:
    path = _entry_path(cache_dir, key)
    if not path.is_file():
        return None
    try:
        with path.open("r", encoding="utf-8") as fh:
            payload = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None
    if payload.get("schema_version") != CACHE_SCHEMA_VERSION:
        return None
    proposals = payload.get("proposals")
    if not isinstance(proposals, list):
        return None
    return proposals


def store_cached_proposals(
    cache_dir: Path,
    key: str,
    proposals: List[dict],
    target_smiles: str,
    template_sets: Iterable[str],
    top_k: int,
    endpoint: str,
    written_at_unix: int,
) -> Path:
    path = _entry_path(cache_dir, key)
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema_version": CACHE_SCHEMA_VERSION,
        "endpoint": endpoint.rstrip("/"),
        "target_smiles": target_smiles.strip(),
        "template_sets": sorted(template_sets),
        "top_k": int(top_k),
        "written_at_unix": int(written_at_unix),
        "proposals": proposals,
    }
    tmp_path = path.with_suffix(".json.tmp")
    with tmp_path.open("w", encoding="utf-8") as fh:
        json.dump(payload, fh, separators=(",", ":"), sort_keys=True)
    os.replace(tmp_path, path)
    return path
