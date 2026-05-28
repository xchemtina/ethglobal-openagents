"""Corpus discovery for the bulk-ingest pass.

Walks a list of root directories, finds every PDF, and deduplicates by Blake3
content hash so the same paper showing up in multiple project trees only
parses once. Returns a deterministically ordered list of `CorpusEntry`
records that downstream stages key on.
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable, List, Sequence

import blake3


CHUNK = 1 << 20  # 1 MiB read chunks; PDFs are usually a few MiB.


@dataclass(frozen=True)
class CorpusEntry:
    """A single physical paper, keyed by canonical Blake3 content hash."""

    content_hash: str
    source_paths: tuple[str, ...]
    size_bytes: int
    mtime_unix: int

    @property
    def primary_path(self) -> str:
        return self.source_paths[0]


def hash_file(path: Path) -> str:
    """Return the canonical Blake3 hex digest of a file's bytes."""
    h = blake3.blake3()
    with path.open("rb") as fh:
        while True:
            chunk = fh.read(CHUNK)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def discover_pdfs(roots: Sequence[Path]) -> List[Path]:
    """Walk each root and return every readable .pdf path, sorted lexicographically."""
    seen: set[Path] = set()
    out: List[Path] = []
    for root in roots:
        root = Path(root).expanduser().resolve()
        if not root.exists():
            continue
        if root.is_file() and root.suffix.lower() == ".pdf":
            seen.add(root)
            out.append(root)
            continue
        for path in root.rglob("*.pdf"):
            if path.is_file():
                resolved = path.resolve()
                if resolved in seen:
                    continue
                seen.add(resolved)
                out.append(resolved)
    out.sort()
    return out


def build_corpus(
    roots: Sequence[Path],
    *,
    progress_cb=None,
) -> List[CorpusEntry]:
    """Discover all PDFs under `roots`, hash each one, and dedup by content hash.

    `progress_cb`, if provided, is called as `progress_cb(current, total, path)`
    after each hash so the caller can render a progress bar.

    Returns a list ordered by content_hash for reproducibility.
    """
    paths = discover_pdfs(roots)
    by_hash: dict[str, dict] = {}
    total = len(paths)
    for index, path in enumerate(paths, 1):
        try:
            digest = hash_file(path)
        except OSError:
            if progress_cb is not None:
                progress_cb(index, total, path)
            continue
        stat = path.stat()
        record = by_hash.setdefault(
            digest,
            {
                "content_hash": digest,
                "source_paths": [],
                "size_bytes": stat.st_size,
                "mtime_unix": int(stat.st_mtime),
            },
        )
        record["source_paths"].append(str(path))
        # Track the most-recent mtime across duplicates.
        record["mtime_unix"] = max(record["mtime_unix"], int(stat.st_mtime))
        if progress_cb is not None:
            progress_cb(index, total, path)

    entries = [
        CorpusEntry(
            content_hash=record["content_hash"],
            source_paths=tuple(sorted(record["source_paths"])),
            size_bytes=record["size_bytes"],
            mtime_unix=record["mtime_unix"],
        )
        for record in by_hash.values()
    ]
    entries.sort(key=lambda e: e.content_hash)
    return entries


__all__ = ["CorpusEntry", "build_corpus", "discover_pdfs", "hash_file"]
