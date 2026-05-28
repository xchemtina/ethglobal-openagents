"""Bulk ingest orchestrator.

Drives `parse_one` across a deduped corpus using a process pool, writes a
single `manifest.json` describing every parsed paper, and a `failures.json`
with the per-paper errors. Resumable: cached entries are reported as
`status=cached` and skipped without re-parsing.

Resilience contract (post-OOM-crash hardening, 2026-05):
  * The pool is created per *batch*; if a worker dies (OOM, segfault, etc.)
    only the in-flight batch is lost, not the whole run.
  * `as_completed` is wrapped in a no-progress timeout. If no future finishes
    within `per_paper_timeout_seconds`, the current batch is aborted and any
    unfinished entries are recorded as `worker crash: batch aborted`.
  * Optional `max_rss_mb` installs an `RLIMIT_AS` cap on each worker, turning
    runaway memory growth into a normal `MemoryError` instead of a system
    hang.
"""

from __future__ import annotations

import multiprocessing as mp
import os
import time
from concurrent.futures import ProcessPoolExecutor, as_completed
from concurrent.futures import TimeoutError as FutureTimeoutError
from concurrent.futures.process import BrokenProcessPool
from dataclasses import asdict
from pathlib import Path
from typing import Callable, List, Optional, Sequence

import orjson

from .corpus import CorpusEntry, build_corpus
from .docling_parse import ParseResult, is_cached, parse_one


DEFAULT_BATCH_SIZE = 32
DEFAULT_PER_PAPER_TIMEOUT_SECONDS = 600.0


def _default_workers() -> int:
    cpu = os.cpu_count() or 2
    # Docling is heavy (layout transformer + table model); leave half the cores
    # for system + other tasks. Floor at 1.
    return max(1, cpu // 2)


def _parse_worker(args: tuple[CorpusEntry, str, bool]) -> dict:
    entry, cache_root, force = args
    result = parse_one(entry, Path(cache_root), force=force)
    return asdict(result)


def _pool_init(max_rss_mb: Optional[int]) -> None:
    """Per-worker initializer. Caps the worker's address space if requested."""
    if max_rss_mb is None:
        return
    try:
        import resource  # POSIX only

        bytes_limit = int(max_rss_mb) * 1024 * 1024
        # RLIMIT_AS bounds total virtual memory; oversteps surface as MemoryError.
        resource.setrlimit(resource.RLIMIT_AS, (bytes_limit, bytes_limit))
    except Exception:
        # Best-effort. If the platform doesn't support it, just continue.
        pass


def _crash_record(entry: CorpusEntry, message: str) -> dict:
    return {
        "content_hash": entry.content_hash,
        "primary_path": entry.primary_path,
        "cache_dir": "",
        "status": "error",
        "docling_version": None,
        "duration_seconds": 0.0,
        "page_count": None,
        "char_count": None,
        "figure_count": None,
        "table_count": None,
        "error": message,
    }


def _bump_counters(result: dict, counters: dict) -> None:
    status = result.get("status")
    if status == "cached":
        counters["cached"] += 1
    elif status == "ok":
        counters["ok"] += 1
    else:
        counters["error"] += 1


def _run_one_batch(
    batch: Sequence[CorpusEntry],
    *,
    cache_root: Path,
    workers: int,
    force: bool,
    per_paper_timeout: Optional[float],
    max_rss_mb: Optional[int],
    progress_cb: Optional[Callable],
    parse_offset: int,
    parse_total: int,
) -> tuple[List[dict], int]:
    """Run one pool batch. Always returns a result for every entry in the batch."""
    results: List[dict] = []
    pending = {entry.content_hash: entry for entry in batch}

    ctx = mp.get_context("spawn")

    def _record(result: dict) -> None:
        nonlocal parse_offset
        results.append(result)
        pending.pop(result["content_hash"], None)
        parse_offset += 1
        if progress_cb is not None:
            progress_cb("parse", parse_offset, parse_total, result)

    try:
        with ProcessPoolExecutor(
            max_workers=workers,
            mp_context=ctx,
            initializer=_pool_init,
            initargs=(max_rss_mb,),
        ) as pool:
            future_to_entry = {
                pool.submit(_parse_worker, (entry, str(cache_root), force)): entry
                for entry in batch
            }
            try:
                for future in as_completed(future_to_entry, timeout=per_paper_timeout):
                    entry = future_to_entry[future]
                    try:
                        result = future.result(timeout=1.0)
                    except BrokenProcessPool:
                        # Re-raised by the outer except so we abort the batch.
                        raise
                    except Exception as exc:
                        result = _crash_record(
                            entry,
                            f"worker crash: {type(exc).__name__}: {exc}",
                        )
                    _record(result)
            except FutureTimeoutError:
                # No future completed within the no-progress window; abort.
                # The `with` block tears down the pool below.
                pass
    except BrokenProcessPool:
        # A worker died. The pool is unusable; drop through and record the
        # remaining entries as worker-crash so we never lose a slot silently.
        pass

    # Anything still pending was either timed out or part of a poisoned pool.
    for content_hash, entry in list(pending.items()):
        _record(
            _crash_record(
                entry,
                "worker crash: batch aborted (BrokenProcessPool/timeout)",
            )
        )

    return results, parse_offset


def bulk_ingest_entries(
    entries: Sequence[CorpusEntry],
    cache_root: Path,
    *,
    workers: Optional[int] = None,
    force: bool = False,
    per_paper_timeout_seconds: Optional[float] = DEFAULT_PER_PAPER_TIMEOUT_SECONDS,
    max_rss_mb: Optional[int] = None,
    batch_size: int = DEFAULT_BATCH_SIZE,
    progress_cb: Optional[Callable] = None,
    roots: Optional[Sequence[Path]] = None,
) -> dict:
    """Parse a precomputed list of corpus entries through Docling.

    The discovery + hash stage is skipped, which is what we want for retry
    runs: load the manifest, project failed rows back to `CorpusEntry`s, and
    feed them in here.
    """
    cache_root = Path(cache_root).expanduser().resolve()
    cache_root.mkdir(parents=True, exist_ok=True)

    started = time.monotonic()
    workers = workers or _default_workers()
    total = len(entries)
    counters = {"ok": 0, "cached": 0, "error": 0}
    results: List[dict] = []

    if progress_cb is not None:
        progress_cb("parse", 0, total, "starting")

    if workers <= 1 or total <= 1:
        # Serial path. Easier to debug; same caching behaviour.
        for index, entry in enumerate(entries, 1):
            if not force and is_cached(cache_root, entry.content_hash):
                result = asdict(parse_one(entry, cache_root, force=False))
            else:
                try:
                    result = _parse_worker((entry, str(cache_root), force))
                except Exception as exc:
                    result = _crash_record(
                        entry, f"worker crash: {type(exc).__name__}: {exc}"
                    )
            results.append(result)
            _bump_counters(result, counters)
            if progress_cb is not None:
                progress_cb("parse", index, total, result)
    else:
        parse_offset = 0
        for batch_start in range(0, total, batch_size):
            batch = list(entries[batch_start : batch_start + batch_size])
            batch_results, parse_offset = _run_one_batch(
                batch,
                cache_root=cache_root,
                workers=workers,
                force=force,
                per_paper_timeout=per_paper_timeout_seconds,
                max_rss_mb=max_rss_mb,
                progress_cb=progress_cb,
                parse_offset=parse_offset,
                parse_total=total,
            )
            for result in batch_results:
                results.append(result)
                _bump_counters(result, counters)

    results.sort(key=lambda r: r["content_hash"])
    failures = [r for r in results if r["status"] == "error"]
    successes = [r for r in results if r["status"] in ("ok", "cached")]

    elapsed = round(time.monotonic() - started, 2)
    manifest = {
        "schema_tag": "chimiaclaw.literature.bulk_ingest.v1",
        "cache_root": str(cache_root),
        "roots": [str(Path(r).expanduser().resolve()) for r in (roots or [])],
        "elapsed_seconds": elapsed,
        "totals": {
            "papers_total": total,
            "ok": counters["ok"],
            "cached": counters["cached"],
            "error": counters["error"],
        },
        "papers": successes,
        "failures": failures,
    }
    return manifest


def bulk_ingest(
    roots: Sequence[Path],
    cache_root: Path,
    *,
    workers: Optional[int] = None,
    limit: Optional[int] = None,
    force: bool = False,
    per_paper_timeout_seconds: Optional[float] = DEFAULT_PER_PAPER_TIMEOUT_SECONDS,
    max_rss_mb: Optional[int] = None,
    batch_size: int = DEFAULT_BATCH_SIZE,
    progress_cb=None,
) -> dict:
    """Discover PDFs under `roots`, parse each through Docling, and return a manifest dict.

    `progress_cb`, if provided, is called as `progress_cb(stage, current, total, info)`
    with stage in {"discover", "hash", "parse"}.
    """

    def hash_progress(current: int, total: int, path: Path) -> None:
        if progress_cb is not None:
            progress_cb("hash", current, total, str(path))

    entries: List[CorpusEntry] = build_corpus(
        [Path(r) for r in roots], progress_cb=hash_progress
    )
    if limit is not None:
        entries = entries[:limit]

    return bulk_ingest_entries(
        entries,
        cache_root=cache_root,
        workers=workers,
        force=force,
        per_paper_timeout_seconds=per_paper_timeout_seconds,
        max_rss_mb=max_rss_mb,
        batch_size=batch_size,
        progress_cb=progress_cb,
        roots=roots,
    )


def write_manifest(manifest: dict, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(orjson.dumps(manifest, option=orjson.OPT_INDENT_2))


__all__ = [
    "bulk_ingest",
    "bulk_ingest_entries",
    "write_manifest",
    "DEFAULT_BATCH_SIZE",
    "DEFAULT_PER_PAPER_TIMEOUT_SECONDS",
]
