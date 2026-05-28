"""Operations on bulk-ingest manifests: load, project to entries, merge.

Used by the `--retry-failed` / `--from-manifest` mode so that a long-running
ingest crash leaves a recoverable manifest on disk. We can then:

  1. Read the manifest.
  2. Project the `failures` rows (and optionally any other rows whose cache
     directory is missing on disk) back to ``CorpusEntry`` objects.
  3. Run a fresh bulk-ingest pass over just those entries.
  4. Merge the new results into the original manifest, replacing matching
     ``content_hash`` rows and recomputing totals.

The merge is content-hash keyed and idempotent: replaying the same retry
results against the same manifest produces an identical manifest.
"""

from __future__ import annotations

import os
import time
from pathlib import Path
from typing import Iterable, List, Optional, Sequence

import orjson

from .corpus import CorpusEntry
from .docling_parse import is_cached


def load_manifest(path: Path) -> dict:
    """Read a bulk-ingest manifest from disk."""
    return orjson.loads(Path(path).read_bytes())


def _entry_from_row(row: dict) -> Optional[CorpusEntry]:
    """Build a CorpusEntry from a manifest row.

    The manifest doesn't always carry the full discovery metadata
    (``source_paths``, ``size_bytes``, ``mtime_unix``); we recover it by
    statting ``primary_path`` if the file still exists, otherwise we synthesize
    safe defaults so the retry can still proceed.
    """
    content_hash = row.get("content_hash")
    primary_path = row.get("primary_path")
    if not content_hash or not primary_path:
        return None
    p = Path(primary_path)
    if p.exists() and p.is_file():
        try:
            stat = p.stat()
            size_bytes = stat.st_size
            mtime_unix = int(stat.st_mtime)
        except OSError:
            size_bytes = int(row.get("size_bytes") or 0)
            mtime_unix = int(row.get("mtime_unix") or 0)
    else:
        # Source has moved/disappeared; we can't actually re-parse, but
        # callers may still want the entry recorded. parse_one will surface
        # the missing-file error.
        size_bytes = int(row.get("size_bytes") or 0)
        mtime_unix = int(row.get("mtime_unix") or 0)
    source_paths = tuple(row.get("source_paths") or (primary_path,))
    return CorpusEntry(
        content_hash=content_hash,
        source_paths=source_paths,
        size_bytes=size_bytes,
        mtime_unix=mtime_unix,
    )


def load_failed_entries(manifest: dict) -> List[CorpusEntry]:
    """Return CorpusEntry objects for every `failures` row in a manifest."""
    out: List[CorpusEntry] = []
    seen: set[str] = set()
    for row in manifest.get("failures", []) or []:
        entry = _entry_from_row(row)
        if entry is None or entry.content_hash in seen:
            continue
        seen.add(entry.content_hash)
        out.append(entry)
    out.sort(key=lambda e: e.content_hash)
    return out


def load_missing_cache_entries(
    manifest: dict, cache_root: Path
) -> List[CorpusEntry]:
    """Return entries whose cache dir is missing on disk despite a manifest row.

    Useful when the cache was partially wiped: any row marked ok/cached but
    whose ``document.md`` is gone needs reparsing.
    """
    cache_root = Path(cache_root).expanduser().resolve()
    out: List[CorpusEntry] = []
    seen: set[str] = set()
    for row in manifest.get("papers", []) or []:
        content_hash = row.get("content_hash")
        if not content_hash or content_hash in seen:
            continue
        if is_cached(cache_root, content_hash):
            continue
        entry = _entry_from_row(row)
        if entry is None:
            continue
        seen.add(entry.content_hash)
        out.append(entry)
    out.sort(key=lambda e: e.content_hash)
    return out


def merge_manifest(base: dict, new_results: Sequence[dict]) -> dict:
    """Return a new manifest with `new_results` overlaid onto `base`.

    Rows are keyed by ``content_hash``: any incoming row replaces the base
    row with the same hash. Totals and `papers`/`failures` partitions are
    recomputed deterministically.
    """
    rows: dict[str, dict] = {}
    for row in (base.get("papers") or []) + (base.get("failures") or []):
        ch = row.get("content_hash")
        if ch:
            rows[ch] = row
    for row in new_results:
        ch = row.get("content_hash")
        if ch:
            rows[ch] = row

    ok = cached = error = 0
    successes: List[dict] = []
    failures: List[dict] = []
    for row in sorted(rows.values(), key=lambda r: r.get("content_hash", "")):
        status = row.get("status")
        if status == "ok":
            ok += 1
            successes.append(row)
        elif status == "cached":
            cached += 1
            successes.append(row)
        else:
            error += 1
            failures.append(row)

    merged = dict(base)
    merged["papers"] = successes
    merged["failures"] = failures
    merged["totals"] = {
        "papers_total": len(rows),
        "ok": ok,
        "cached": cached,
        "error": error,
    }
    merged["last_merged_unix"] = int(time.time())
    return merged


__all__ = [
    "load_manifest",
    "load_failed_entries",
    "load_missing_cache_entries",
    "merge_manifest",
]
