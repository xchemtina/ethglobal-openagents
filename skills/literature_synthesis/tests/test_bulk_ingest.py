"""Bulk-ingest tests that don't require docling itself.

The corpus + manifest layer is exercised here. The actual Docling parse path
is covered by the smoke-run against the real corpus, not unit tests.
"""

from __future__ import annotations

import json
from pathlib import Path

import orjson
import pytest

from literature_synthesis.bulk_ingest import (
    bulk_ingest,
    bulk_ingest_entries,
    write_manifest,
)
from literature_synthesis.corpus import CorpusEntry, build_corpus, hash_file
from literature_synthesis.manifest_ops import (
    load_failed_entries,
    load_missing_cache_entries,
    merge_manifest,
)


def _write_pdf(path: Path, body: bytes) -> None:
    """Write a fake 'PDF' payload. Docling won't be invoked here; only the corpus layer is."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(body)


def test_hash_file_is_stable(tmp_path: Path) -> None:
    p = tmp_path / "a.pdf"
    _write_pdf(p, b"%PDF-1.4\n...same body...")
    a = hash_file(p)
    b = hash_file(p)
    assert a == b
    assert len(a) == 64  # blake3 hex


def test_build_corpus_dedups_by_content_hash(tmp_path: Path) -> None:
    body = b"%PDF-1.4\nshared body across two paths\n"
    a = tmp_path / "x" / "paper.pdf"
    b = tmp_path / "y" / "paper-copy.pdf"
    _write_pdf(a, body)
    _write_pdf(b, body)
    other = tmp_path / "z" / "different.pdf"
    _write_pdf(other, b"%PDF-1.4\nunique body\n")

    entries = build_corpus([tmp_path])
    assert len(entries) == 2

    by_hash = {e.content_hash: e for e in entries}
    dup_entry = next(e for e in entries if len(e.source_paths) == 2)
    assert sorted(dup_entry.source_paths) == sorted([str(a.resolve()), str(b.resolve())])

    # Ordering is deterministic by content_hash.
    sorted_hashes = [e.content_hash for e in entries]
    assert sorted_hashes == sorted(sorted_hashes)


def test_build_corpus_returns_empty_for_missing_root(tmp_path: Path) -> None:
    nope = tmp_path / "does-not-exist"
    entries = build_corpus([nope])
    assert entries == []


def test_bulk_ingest_serial_uses_cache(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Force the serial path (workers=1) and stub `parse_one` so docling is never called."""
    body_a = b"%PDF-1.4\nbody A\n"
    body_b = b"%PDF-1.4\nbody B\n"
    pdf_a = tmp_path / "papers" / "a.pdf"
    pdf_b = tmp_path / "papers" / "b.pdf"
    _write_pdf(pdf_a, body_a)
    _write_pdf(pdf_b, body_b)
    cache = tmp_path / "cache"

    call_log: list[str] = []

    def fake_parse_one(entry, cache_root, *, force=False):
        cache_dir = Path(cache_root) / entry.content_hash[:2] / entry.content_hash
        already_cached = (
            (cache_dir / "document.md").exists()
            and (cache_dir / "meta.json").exists()
        )
        from literature_synthesis.docling_parse import ParseResult

        if already_cached and not force:
            return ParseResult(
                content_hash=entry.content_hash,
                primary_path=entry.primary_path,
                cache_dir=str(cache_dir),
                status="cached",
                docling_version="stub",
                duration_seconds=0.0,
                page_count=1,
                char_count=7,
                figure_count=0,
                table_count=0,
                error=None,
            )

        call_log.append(entry.content_hash)
        cache_dir.mkdir(parents=True, exist_ok=True)
        (cache_dir / "document.md").write_text("# stub\n", encoding="utf-8")
        meta = {
            "content_hash": entry.content_hash,
            "primary_path": entry.primary_path,
            "page_count": 1,
            "char_count": 7,
            "figure_count": 0,
            "table_count": 0,
            "duration_seconds": 0.001,
            "status": "ok",
        }
        (cache_dir / "meta.json").write_bytes(orjson.dumps(meta))

        return ParseResult(
            content_hash=entry.content_hash,
            primary_path=entry.primary_path,
            cache_dir=str(cache_dir),
            status="ok",
            docling_version="stub",
            duration_seconds=0.001,
            page_count=1,
            char_count=7,
            figure_count=0,
            table_count=0,
            error=None,
        )

    monkeypatch.setattr("literature_synthesis.bulk_ingest.parse_one", fake_parse_one)

    manifest = bulk_ingest(
        roots=[tmp_path / "papers"],
        cache_root=cache,
        workers=1,
        force=False,
    )
    assert manifest["totals"]["papers_total"] == 2
    assert manifest["totals"]["ok"] == 2
    assert manifest["totals"]["error"] == 0
    assert len(call_log) == 2

    # Second pass should hit the cache and never call parse_one.
    call_log.clear()
    manifest2 = bulk_ingest(
        roots=[tmp_path / "papers"],
        cache_root=cache,
        workers=1,
        force=False,
    )
    assert manifest2["totals"]["cached"] == 2
    assert manifest2["totals"]["ok"] == 0
    assert call_log == []


def test_write_manifest_round_trips(tmp_path: Path) -> None:
    payload = {"hello": "world", "nested": {"a": [1, 2, 3]}}
    out = tmp_path / "manifest.json"
    write_manifest(payload, out)
    loaded = json.loads(out.read_text())
    assert loaded == payload


# ---------------------------------------------------------------------------
# manifest_ops: load_failed_entries / load_missing_cache_entries / merge_manifest
# ---------------------------------------------------------------------------


def _stub_pdf(path: Path, body: bytes = b"%PDF-1.4\nstub\n") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(body)


def test_load_failed_entries_recovers_corpus_entries(tmp_path: Path) -> None:
    pdf_a = tmp_path / "a.pdf"
    pdf_b = tmp_path / "missing.pdf"  # never created on disk
    _stub_pdf(pdf_a)

    manifest = {
        "failures": [
            {
                "content_hash": "a" * 64,
                "primary_path": str(pdf_a),
                "status": "error",
                "error": "worker crash: BrokenProcessPool",
            },
            {
                "content_hash": "b" * 64,
                "primary_path": str(pdf_b),  # source has moved
                "status": "error",
                "error": "worker crash: BrokenProcessPool",
            },
            {
                "content_hash": "a" * 64,  # duplicate row, must be deduped
                "primary_path": str(pdf_a),
                "status": "error",
                "error": "ConversionError",
            },
        ]
    }

    entries = load_failed_entries(manifest)
    assert len(entries) == 2
    by_hash = {e.content_hash: e for e in entries}
    # On-disk file: stat populates size + mtime.
    assert by_hash["a" * 64].size_bytes == pdf_a.stat().st_size
    # Missing file: size_bytes falls back to 0, but the entry still exists.
    assert by_hash["b" * 64].size_bytes == 0
    assert by_hash["b" * 64].source_paths == (str(pdf_b),)
    # Deterministic ordering.
    assert [e.content_hash for e in entries] == sorted(e.content_hash for e in entries)


def test_load_missing_cache_entries_only_returns_uncached(tmp_path: Path) -> None:
    cache_root = tmp_path / "cache"
    pdf = tmp_path / "paper.pdf"
    _stub_pdf(pdf)

    cached_hash = "c" * 64
    missing_hash = "d" * 64
    cached_dir = cache_root / cached_hash[:2] / cached_hash
    cached_dir.mkdir(parents=True, exist_ok=True)
    (cached_dir / "document.md").write_text("stub\n")
    (cached_dir / "meta.json").write_text("{}\n")

    manifest = {
        "papers": [
            {
                "content_hash": cached_hash,
                "primary_path": str(pdf),
                "status": "ok",
            },
            {
                "content_hash": missing_hash,
                "primary_path": str(pdf),
                "status": "ok",
            },
        ]
    }
    entries = load_missing_cache_entries(manifest, cache_root)
    assert [e.content_hash for e in entries] == [missing_hash]


def test_merge_manifest_replaces_failures_and_recomputes_totals() -> None:
    base = {
        "schema_tag": "chimiaclaw.literature.bulk_ingest.v1",
        "papers": [
            {"content_hash": "a" * 64, "primary_path": "/p/a.pdf", "status": "ok"},
            {"content_hash": "b" * 64, "primary_path": "/p/b.pdf", "status": "cached"},
        ],
        "failures": [
            {
                "content_hash": "c" * 64,
                "primary_path": "/p/c.pdf",
                "status": "error",
                "error": "worker crash",
            },
            {
                "content_hash": "d" * 64,
                "primary_path": "/p/d.pdf",
                "status": "error",
                "error": "ConversionError",
            },
        ],
        "totals": {"papers_total": 4, "ok": 1, "cached": 1, "error": 2},
    }
    # Retry results: c now succeeds, d still fails, a fresh row e arrives.
    new_results = [
        {"content_hash": "c" * 64, "primary_path": "/p/c.pdf", "status": "ok"},
        {
            "content_hash": "d" * 64,
            "primary_path": "/p/d.pdf",
            "status": "error",
            "error": "ConversionError",
        },
        {"content_hash": "e" * 64, "primary_path": "/p/e.pdf", "status": "ok"},
    ]

    merged = merge_manifest(base, new_results)
    totals = merged["totals"]
    assert totals == {"papers_total": 5, "ok": 3, "cached": 1, "error": 1}

    paper_hashes = sorted(p["content_hash"] for p in merged["papers"])
    failure_hashes = sorted(f["content_hash"] for f in merged["failures"])
    assert paper_hashes == ["a" * 64, "b" * 64, "c" * 64, "e" * 64]
    assert failure_hashes == ["d" * 64]

    # Idempotent: re-merging the same results should not move counters.
    again = merge_manifest(merged, new_results)
    assert again["totals"] == merged["totals"]
    assert {p["content_hash"] for p in again["papers"]} == {
        p["content_hash"] for p in merged["papers"]
    }


# ---------------------------------------------------------------------------
# bulk_ingest_entries: skip discovery and parse a precomputed list
# ---------------------------------------------------------------------------


def test_bulk_ingest_entries_serial_uses_cache(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Stub parse_one and confirm bulk_ingest_entries skips discovery cleanly."""
    pdf = tmp_path / "p.pdf"
    _stub_pdf(pdf, b"%PDF-1.4\nbody\n")
    cache = tmp_path / "cache"

    # Build an entry by hashing the file (same way bulk_ingest would).
    entry = CorpusEntry(
        content_hash=hash_file(pdf),
        source_paths=(str(pdf),),
        size_bytes=pdf.stat().st_size,
        mtime_unix=int(pdf.stat().st_mtime),
    )

    call_log: list[str] = []

    def fake_parse_one(e, cache_root, *, force=False):
        cache_dir = Path(cache_root) / e.content_hash[:2] / e.content_hash
        already_cached = (
            (cache_dir / "document.md").exists()
            and (cache_dir / "meta.json").exists()
        )
        from literature_synthesis.docling_parse import ParseResult

        if already_cached and not force:
            return ParseResult(
                content_hash=e.content_hash,
                primary_path=e.primary_path,
                cache_dir=str(cache_dir),
                status="cached",
                docling_version="stub",
                duration_seconds=0.0,
                page_count=1,
                char_count=4,
                figure_count=0,
                table_count=0,
                error=None,
            )
        call_log.append(e.content_hash)
        cache_dir.mkdir(parents=True, exist_ok=True)
        (cache_dir / "document.md").write_text("# stub\n", encoding="utf-8")
        (cache_dir / "meta.json").write_bytes(orjson.dumps({"status": "ok"}))
        return ParseResult(
            content_hash=e.content_hash,
            primary_path=e.primary_path,
            cache_dir=str(cache_dir),
            status="ok",
            docling_version="stub",
            duration_seconds=0.001,
            page_count=1,
            char_count=4,
            figure_count=0,
            table_count=0,
            error=None,
        )

    monkeypatch.setattr(
        "literature_synthesis.bulk_ingest.parse_one", fake_parse_one
    )

    manifest = bulk_ingest_entries([entry], cache_root=cache, workers=1)
    assert manifest["totals"] == {
        "papers_total": 1,
        "ok": 1,
        "cached": 0,
        "error": 0,
    }
    assert call_log == [entry.content_hash]

    # Second pass: parse_one returns cached without recording a new call.
    call_log.clear()
    manifest2 = bulk_ingest_entries([entry], cache_root=cache, workers=1)
    assert manifest2["totals"]["cached"] == 1
    assert call_log == []


def test_bulk_ingest_entries_serial_records_worker_crash(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A worker raising in serial mode is recorded as a worker-crash error row."""
    pdf = tmp_path / "p.pdf"
    _stub_pdf(pdf)
    entry = CorpusEntry(
        content_hash=hash_file(pdf),
        source_paths=(str(pdf),),
        size_bytes=pdf.stat().st_size,
        mtime_unix=int(pdf.stat().st_mtime),
    )

    def bomb(args):
        raise MemoryError("simulated OOM")

    monkeypatch.setattr("literature_synthesis.bulk_ingest._parse_worker", bomb)

    manifest = bulk_ingest_entries(
        [entry], cache_root=tmp_path / "cache", workers=1
    )
    assert manifest["totals"]["error"] == 1
    assert manifest["totals"]["ok"] == 0
    assert manifest["failures"][0]["error"].startswith("worker crash:")
    assert "MemoryError" in manifest["failures"][0]["error"]
