"""Docling parse wrapper with a content-hash-keyed cache.

Layout under `cache_dir/<content_hash>/`:

    document.md        - canonical markdown export
    document.json      - rich Docling structured JSON (`DoclingDocument.export_to_dict`)
    meta.json          - parse metadata (timing, version, source paths, status)
    error.json         - present iff the parse failed; contains stage + traceback

The wrapper imports `docling` lazily so the rest of the worker stays importable
on machines without it (CI, the offline test path, etc).
"""

from __future__ import annotations

import json
import os
import time
import traceback
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Optional

import orjson

from .corpus import CorpusEntry


@dataclass
class ParseResult:
    """One parse attempt's outcome, persisted as `meta.json`."""

    content_hash: str
    primary_path: str
    cache_dir: str
    status: str  # "ok" | "cached" | "error"
    docling_version: Optional[str]
    duration_seconds: float
    page_count: Optional[int]
    char_count: Optional[int]
    figure_count: Optional[int]
    table_count: Optional[int]
    error: Optional[str]


def _docling_version() -> str:
    try:
        from importlib.metadata import version

        return version("docling")
    except Exception:
        return "unknown"


def cache_dir_for(cache_root: Path, content_hash: str) -> Path:
    """Return (and create) the per-paper cache dir."""
    cache_dir = cache_root / content_hash[:2] / content_hash
    cache_dir.mkdir(parents=True, exist_ok=True)
    return cache_dir


def is_cached(cache_root: Path, content_hash: str) -> bool:
    cache_dir = cache_root / content_hash[:2] / content_hash
    return (cache_dir / "document.md").exists() and (cache_dir / "meta.json").exists()


def load_cached_meta(cache_root: Path, content_hash: str) -> Optional[dict]:
    cache_dir = cache_root / content_hash[:2] / content_hash
    meta_path = cache_dir / "meta.json"
    if not meta_path.exists():
        return None
    try:
        return orjson.loads(meta_path.read_bytes())
    except Exception:
        return None


def parse_one(
    entry: CorpusEntry,
    cache_root: Path,
    *,
    force: bool = False,
) -> ParseResult:
    """Parse one corpus entry through Docling and write its cache.

    Idempotent: if a cached parse already exists, skip and return status=cached
    (unless `force=True`).
    """
    cache_dir = cache_dir_for(cache_root, entry.content_hash)
    docling_v = _docling_version()
    started_at = time.monotonic()

    if not force and (cache_dir / "document.md").exists() and (cache_dir / "meta.json").exists():
        cached = load_cached_meta(cache_root, entry.content_hash) or {}
        return ParseResult(
            content_hash=entry.content_hash,
            primary_path=entry.primary_path,
            cache_dir=str(cache_dir),
            status="cached",
            docling_version=docling_v,
            duration_seconds=0.0,
            page_count=cached.get("page_count"),
            char_count=cached.get("char_count"),
            figure_count=cached.get("figure_count"),
            table_count=cached.get("table_count"),
            error=None,
        )

    error_path = cache_dir / "error.json"
    if error_path.exists() and not force:
        # An earlier run already failed; leave the artefact alone unless asked.
        prev = orjson.loads(error_path.read_bytes())
        return ParseResult(
            content_hash=entry.content_hash,
            primary_path=entry.primary_path,
            cache_dir=str(cache_dir),
            status="error",
            docling_version=docling_v,
            duration_seconds=0.0,
            page_count=None,
            char_count=None,
            figure_count=None,
            table_count=None,
            error=prev.get("error"),
        )

    try:
        from docling.document_converter import DocumentConverter  # type: ignore
    except Exception as exc:  # pragma: no cover -- env-specific
        result = ParseResult(
            content_hash=entry.content_hash,
            primary_path=entry.primary_path,
            cache_dir=str(cache_dir),
            status="error",
            docling_version=docling_v,
            duration_seconds=time.monotonic() - started_at,
            page_count=None,
            char_count=None,
            figure_count=None,
            table_count=None,
            error=f"docling unavailable: {exc}",
        )
        _persist_error(cache_dir, result, exc)
        return result

    try:
        converter = DocumentConverter()
        conversion = converter.convert(entry.primary_path)
        doc = conversion.document
        markdown = doc.export_to_markdown()
        rich = doc.export_to_dict()
        page_count = len(getattr(doc, "pages", []) or []) or rich.get("num_pages")
        figures = rich.get("pictures", []) if isinstance(rich, dict) else []
        tables = rich.get("tables", []) if isinstance(rich, dict) else []
        char_count = len(markdown)

        (cache_dir / "document.md").write_text(markdown, encoding="utf-8")
        (cache_dir / "document.json").write_bytes(
            orjson.dumps(rich, option=orjson.OPT_INDENT_2)
        )

        meta = {
            "content_hash": entry.content_hash,
            "primary_path": entry.primary_path,
            "source_paths": list(entry.source_paths),
            "size_bytes": entry.size_bytes,
            "mtime_unix": entry.mtime_unix,
            "docling_version": docling_v,
            "page_count": page_count,
            "char_count": char_count,
            "figure_count": len(figures),
            "table_count": len(tables),
            "duration_seconds": round(time.monotonic() - started_at, 3),
            "status": "ok",
        }
        (cache_dir / "meta.json").write_bytes(orjson.dumps(meta, option=orjson.OPT_INDENT_2))
        if error_path.exists():
            error_path.unlink(missing_ok=True)

        return ParseResult(
            content_hash=entry.content_hash,
            primary_path=entry.primary_path,
            cache_dir=str(cache_dir),
            status="ok",
            docling_version=docling_v,
            duration_seconds=meta["duration_seconds"],
            page_count=page_count,
            char_count=char_count,
            figure_count=len(figures),
            table_count=len(tables),
            error=None,
        )
    except Exception as exc:
        result = ParseResult(
            content_hash=entry.content_hash,
            primary_path=entry.primary_path,
            cache_dir=str(cache_dir),
            status="error",
            docling_version=docling_v,
            duration_seconds=round(time.monotonic() - started_at, 3),
            page_count=None,
            char_count=None,
            figure_count=None,
            table_count=None,
            error=f"{type(exc).__name__}: {exc}",
        )
        _persist_error(cache_dir, result, exc)
        return result


def _persist_error(cache_dir: Path, result: ParseResult, exc: BaseException) -> None:
    payload = {
        **asdict(result),
        "traceback": "".join(
            traceback.format_exception(type(exc), exc, exc.__traceback__)
        ),
    }
    (cache_dir / "error.json").write_bytes(orjson.dumps(payload, option=orjson.OPT_INDENT_2))


__all__ = [
    "ParseResult",
    "cache_dir_for",
    "is_cached",
    "load_cached_meta",
    "parse_one",
]
