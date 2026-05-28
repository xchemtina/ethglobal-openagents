"""Per-paper extraction over a populated Docling cache.

Layered on top of the bulk-ingest manifest. For each cached paper we:

  1. Read ``document.md`` + ``meta.json`` from
     ``cache_root/<2hex>/<content_hash>/``.
  2. Build a single-element ``LiteratureCitation`` and excerpt using cheap,
     deterministic heuristics (no extra LLM calls). The citation carries
     ``license="local-corpus"`` because we are operating on a private corpus
     of already-downloaded PDFs.
  3. Call the existing ``extract_synthesis`` pipeline with whatever runtime
     was selected (typically ``mlx-local`` for overnight runs, or
     ``offline`` for tests).
  4. Persist the resulting ``LiteratureSynthesis`` to
     ``output_root/<2hex>/<content_hash>/synthesis.json`` and a per-paper
     ``synthesis-meta.json`` with timing + provenance. Failures land in
     ``synthesis-error.json`` so a re-run can target them specifically.
  5. Emit a corpus-level manifest summarising counts and artifact paths.

Resumability: a paper with an existing ``synthesis.json`` is reported as
``cached`` and skipped unless ``force=True``.
"""

from __future__ import annotations

import re
import time
import traceback
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable, List, Optional, Sequence

import orjson

from .extract import extract_synthesis, write_synthesis
from .runtime import GenerationRuntime, RuntimeError_
from .schema import (
    LiteratureCitation,
    LiteratureIngestManifest,
    LiteratureSource,
    LiteratureSourceKind,
    LiteratureSynthesis,
)


DEFAULT_OUTPUT_ROOT = Path.home() / ".chimia_kb" / "literature"
DEFAULT_QUERY = "Extract synthesis-relevant chemistry from this paper."
DEFAULT_SECTOR = "general-chemistry"
DEFAULT_LICENSE = "local-corpus"
EXCERPT_MAX_CHARS = 6000

# Digit-only boundary so years embedded in `Doe_2024_paper.pdf` still match.
# `\b` won't work here because `_` is a word character.
_YEAR_RE = re.compile(r"(?<!\d)(?:19|20)\d{2}(?!\d)")
# Match a markdown H1 with same-line content only. `\s+` is too liberal: it
# crosses newlines and grabs the next paragraph when the H1 line is blank.
_TITLE_RE = re.compile(r"^#[ \t]+(\S[^\n\r]*)$", re.MULTILINE)


@dataclass
class CorpusExtractResult:
    """Outcome of one per-paper extraction."""

    content_hash: str
    primary_path: str
    output_dir: str
    status: str  # "ok" | "cached" | "error"
    duration_seconds: float
    runtime: Optional[str]
    model_id: Optional[str]
    prompt_hash: Optional[str]
    molecule_count: int
    reaction_count: int
    claim_count: int
    error: Optional[str]


# ---------------------------------------------------------------------------
# Heuristic title / year / excerpt extraction
# ---------------------------------------------------------------------------


def heuristic_title(markdown: str, fallback: str) -> str:
    """Return the first markdown H1 if present, else ``fallback``."""
    match = _TITLE_RE.search(markdown[:4000])
    if match:
        title = match.group(1).strip()
        if title:
            return title
    # Fallback to the filename stem.
    return fallback


def heuristic_year(markdown: str, primary_path: str) -> int:
    """Return a 4-digit year guessed from the filename or the first lines."""
    name_match = _YEAR_RE.search(Path(primary_path).name)
    if name_match:
        return int(name_match.group(0))
    head_match = _YEAR_RE.search(markdown[:4000])
    if head_match:
        return int(head_match.group(0))
    return 0


def build_excerpt(markdown: str, *, max_chars: int = EXCERPT_MAX_CHARS) -> str:
    """Return a deterministic excerpt of the paper for prompting.

    Strategy: drop empty lines and trivial table-formatting noise, then take
    the first ``max_chars`` characters. This keeps the abstract / intro /
    methods, which is where most of the chemistry usually lives.
    """
    cleaned_lines = []
    for line in markdown.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        # Drop pure markdown table separators like "|---|---|".
        if set(stripped) <= {"|", "-", ":", " "}:
            continue
        cleaned_lines.append(stripped)
    joined = "\n".join(cleaned_lines)
    if len(joined) <= max_chars:
        return joined
    return joined[:max_chars]


def build_citation(
    *,
    title: str,
    year: int,
    primary_path: str,
    license: str = DEFAULT_LICENSE,
) -> LiteratureCitation:
    """Construct a single-paper citation from the cache metadata."""
    return LiteratureCitation(
        title=title,
        authors=[],  # heuristic author parsing is unreliable; leave empty
        year=year,
        doi=None,
        source_url=f"file://{primary_path}",
        license=license,
        retrieved_at_unix=int(time.time()),
    )


def build_manifest(
    *,
    query: str,
    sector: str,
    primary_path: str,
    license: str = DEFAULT_LICENSE,
) -> LiteratureIngestManifest:
    """Build a single-source ingest manifest covering one local PDF."""
    return LiteratureIngestManifest(
        query=query,
        sector=sector,
        requested_at_unix=int(time.time()),
        max_papers=1,
        sources=[
            LiteratureSource(
                kind=LiteratureSourceKind.local_pdf,
                identifier=primary_path,
                url=f"file://{primary_path}",
                license_hint=license,
            )
        ],
        local_dir=str(Path(primary_path).parent),
        license_whitelist=[license],
    )


# ---------------------------------------------------------------------------
# Per-paper extraction
# ---------------------------------------------------------------------------


def cache_paths(cache_root: Path, content_hash: str) -> tuple[Path, Path, Path]:
    """Return (cache_dir, document_md_path, meta_json_path) for a paper."""
    cache_dir = cache_root / content_hash[:2] / content_hash
    return cache_dir, cache_dir / "document.md", cache_dir / "meta.json"


def output_paths(output_root: Path, content_hash: str) -> tuple[Path, Path, Path]:
    """Return (out_dir, synthesis_json_path, error_json_path)."""
    out_dir = output_root / content_hash[:2] / content_hash
    return out_dir, out_dir / "synthesis.json", out_dir / "synthesis-error.json"


def is_extracted(output_root: Path, content_hash: str) -> bool:
    _, synthesis_path, _ = output_paths(output_root, content_hash)
    return synthesis_path.exists()


def _result_from_synthesis(
    *,
    content_hash: str,
    primary_path: str,
    output_dir: Path,
    synthesis: LiteratureSynthesis,
    duration_seconds: float,
    status: str,
) -> CorpusExtractResult:
    return CorpusExtractResult(
        content_hash=content_hash,
        primary_path=primary_path,
        output_dir=str(output_dir),
        status=status,
        duration_seconds=duration_seconds,
        runtime=synthesis.model_provenance.runtime.value,
        model_id=synthesis.model_provenance.model_id,
        prompt_hash=synthesis.model_provenance.prompt_hash,
        molecule_count=len(synthesis.molecule_candidates),
        reaction_count=len(synthesis.reaction_candidates),
        claim_count=len(synthesis.extracted_claims),
        error=None,
    )


def extract_one_from_cache(
    *,
    content_hash: str,
    primary_path: str,
    cache_root: Path,
    output_root: Path,
    runtime: GenerationRuntime,
    query: str = DEFAULT_QUERY,
    sector: str = DEFAULT_SECTOR,
    license: str = DEFAULT_LICENSE,
    excerpt_max_chars: int = EXCERPT_MAX_CHARS,
    force: bool = False,
) -> CorpusExtractResult:
    """Run one per-paper extraction. Idempotent unless ``force=True``."""
    cache_dir, doc_md, _meta = cache_paths(cache_root, content_hash)
    out_dir, synthesis_path, error_path = output_paths(output_root, content_hash)
    out_dir.mkdir(parents=True, exist_ok=True)

    started = time.monotonic()

    if not force and synthesis_path.exists():
        try:
            synthesis = LiteratureSynthesis.model_validate(
                orjson.loads(synthesis_path.read_bytes())
            )
            return _result_from_synthesis(
                content_hash=content_hash,
                primary_path=primary_path,
                output_dir=out_dir,
                synthesis=synthesis,
                duration_seconds=0.0,
                status="cached",
            )
        except Exception:
            # Existing artifact is corrupt; fall through and re-extract.
            pass

    if not doc_md.exists():
        message = f"missing cache: {doc_md} not found"
        _persist_error(error_path, content_hash, primary_path, message, traceback="")
        return CorpusExtractResult(
            content_hash=content_hash,
            primary_path=primary_path,
            output_dir=str(out_dir),
            status="error",
            duration_seconds=round(time.monotonic() - started, 3),
            runtime=None,
            model_id=None,
            prompt_hash=None,
            molecule_count=0,
            reaction_count=0,
            claim_count=0,
            error=message,
        )

    try:
        markdown = doc_md.read_text(encoding="utf-8")
    except OSError as exc:
        message = f"failed to read {doc_md}: {exc}"
        _persist_error(error_path, content_hash, primary_path, message, "")
        return CorpusExtractResult(
            content_hash=content_hash,
            primary_path=primary_path,
            output_dir=str(out_dir),
            status="error",
            duration_seconds=round(time.monotonic() - started, 3),
            runtime=None,
            model_id=None,
            prompt_hash=None,
            molecule_count=0,
            reaction_count=0,
            claim_count=0,
            error=message,
        )

    title = heuristic_title(markdown, fallback=Path(primary_path).stem)
    year = heuristic_year(markdown, primary_path=primary_path)
    excerpt = build_excerpt(markdown, max_chars=excerpt_max_chars)
    citation = build_citation(
        title=title, year=year, primary_path=primary_path, license=license
    )
    manifest = build_manifest(
        query=query, sector=sector, primary_path=primary_path, license=license
    )

    try:
        synthesis = extract_synthesis(
            manifest=manifest,
            citations=[citation],
            excerpts=[excerpt],
            runtime=runtime,
        )
    except (RuntimeError_, ValueError, RuntimeError, OSError) as exc:
        tb = "".join(traceback.format_exception(type(exc), exc, exc.__traceback__))
        message = f"{type(exc).__name__}: {exc}"
        _persist_error(error_path, content_hash, primary_path, message, tb)
        return CorpusExtractResult(
            content_hash=content_hash,
            primary_path=primary_path,
            output_dir=str(out_dir),
            status="error",
            duration_seconds=round(time.monotonic() - started, 3),
            runtime=None,
            model_id=None,
            prompt_hash=None,
            molecule_count=0,
            reaction_count=0,
            claim_count=0,
            error=message,
        )

    write_synthesis(synthesis, synthesis_path)
    if error_path.exists():
        try:
            error_path.unlink()
        except OSError:
            pass

    duration = round(time.monotonic() - started, 3)
    _persist_meta(
        out_dir / "synthesis-meta.json",
        content_hash=content_hash,
        primary_path=primary_path,
        title=title,
        year=year,
        excerpt_chars=len(excerpt),
        duration_seconds=duration,
        synthesis=synthesis,
    )
    return _result_from_synthesis(
        content_hash=content_hash,
        primary_path=primary_path,
        output_dir=out_dir,
        synthesis=synthesis,
        duration_seconds=duration,
        status="ok",
    )


def _persist_error(
    error_path: Path,
    content_hash: str,
    primary_path: str,
    error_message: str,
    traceback: str,
) -> None:
    error_path.parent.mkdir(parents=True, exist_ok=True)
    error_path.write_bytes(
        orjson.dumps(
            {
                "content_hash": content_hash,
                "primary_path": primary_path,
                "error": error_message,
                "traceback": traceback,
                "recorded_at_unix": int(time.time()),
            },
            option=orjson.OPT_INDENT_2,
        )
    )


def _persist_meta(
    meta_path: Path,
    *,
    content_hash: str,
    primary_path: str,
    title: str,
    year: int,
    excerpt_chars: int,
    duration_seconds: float,
    synthesis: LiteratureSynthesis,
) -> None:
    payload = {
        "content_hash": content_hash,
        "primary_path": primary_path,
        "title": title,
        "year": year,
        "excerpt_chars": excerpt_chars,
        "duration_seconds": duration_seconds,
        "molecule_count": len(synthesis.molecule_candidates),
        "reaction_count": len(synthesis.reaction_candidates),
        "claim_count": len(synthesis.extracted_claims),
        "model_provenance": synthesis.model_provenance.model_dump(mode="json"),
        "extracted_at_unix": int(time.time()),
    }
    meta_path.parent.mkdir(parents=True, exist_ok=True)
    meta_path.write_bytes(orjson.dumps(payload, option=orjson.OPT_INDENT_2))


# ---------------------------------------------------------------------------
# Corpus-level orchestration
# ---------------------------------------------------------------------------


def _eligible_papers(bulk_manifest: dict) -> List[dict]:
    """Return rows from the bulk-ingest manifest that have a usable cache."""
    rows = bulk_manifest.get("papers") or []
    return [r for r in rows if r.get("status") in {"ok", "cached"}]


def extract_corpus_from_manifest(
    *,
    bulk_manifest: dict,
    cache_root: Path,
    output_root: Path,
    runtime: GenerationRuntime,
    query: str = DEFAULT_QUERY,
    sector: str = DEFAULT_SECTOR,
    license: str = DEFAULT_LICENSE,
    excerpt_max_chars: int = EXCERPT_MAX_CHARS,
    force: bool = False,
    skip_existing: bool = True,
    limit: Optional[int] = None,
    progress_cb: Optional[Callable[[int, int, CorpusExtractResult], None]] = None,
) -> dict:
    """Extract synthesis artifacts for every cached paper in ``bulk_manifest``.

    Runs serially because the default runtime (``mlx-local``) holds a GPU
    model and cannot safely be invoked from multiple processes. Callers that
    pick an HTTP-based runtime can layer their own concurrency.
    """
    cache_root = Path(cache_root).expanduser().resolve()
    output_root = Path(output_root).expanduser().resolve()
    output_root.mkdir(parents=True, exist_ok=True)

    rows = _eligible_papers(bulk_manifest)
    rows.sort(key=lambda r: r.get("content_hash", ""))
    if limit is not None:
        rows = rows[:limit]

    started = time.monotonic()
    results: List[CorpusExtractResult] = []
    counters = {"ok": 0, "cached": 0, "error": 0}

    for index, row in enumerate(rows, 1):
        content_hash = row.get("content_hash") or ""
        primary_path = row.get("primary_path") or ""
        if not content_hash or not primary_path:
            continue

        if skip_existing and not force and is_extracted(output_root, content_hash):
            cache_dir, _, _ = output_paths(output_root, content_hash)
            try:
                synthesis = LiteratureSynthesis.model_validate(
                    orjson.loads((cache_dir / "synthesis.json").read_bytes())
                )
                result = _result_from_synthesis(
                    content_hash=content_hash,
                    primary_path=primary_path,
                    output_dir=cache_dir,
                    synthesis=synthesis,
                    duration_seconds=0.0,
                    status="cached",
                )
            except Exception:
                # Corrupt prior artifact: fall back to a real run.
                result = extract_one_from_cache(
                    content_hash=content_hash,
                    primary_path=primary_path,
                    cache_root=cache_root,
                    output_root=output_root,
                    runtime=runtime,
                    query=query,
                    sector=sector,
                    license=license,
                    excerpt_max_chars=excerpt_max_chars,
                    force=True,
                )
        else:
            result = extract_one_from_cache(
                content_hash=content_hash,
                primary_path=primary_path,
                cache_root=cache_root,
                output_root=output_root,
                runtime=runtime,
                query=query,
                sector=sector,
                license=license,
                excerpt_max_chars=excerpt_max_chars,
                force=force,
            )

        counters[result.status] += 1
        results.append(result)
        if progress_cb is not None:
            progress_cb(index, len(rows), result)

    elapsed = round(time.monotonic() - started, 2)
    return {
        "schema_tag": "chimiaclaw.literature.corpus_extract.v1",
        "cache_root": str(cache_root),
        "output_root": str(output_root),
        "query": query,
        "sector": sector,
        "license": license,
        "excerpt_max_chars": excerpt_max_chars,
        "runtime": runtime.runtime.value,
        "model_id": runtime.model_id,
        "elapsed_seconds": elapsed,
        "totals": {
            "papers_total": len(results),
            "ok": counters["ok"],
            "cached": counters["cached"],
            "error": counters["error"],
        },
        "papers": [
            {
                "content_hash": r.content_hash,
                "primary_path": r.primary_path,
                "output_dir": r.output_dir,
                "status": r.status,
                "duration_seconds": r.duration_seconds,
                "runtime": r.runtime,
                "model_id": r.model_id,
                "prompt_hash": r.prompt_hash,
                "molecule_count": r.molecule_count,
                "reaction_count": r.reaction_count,
                "claim_count": r.claim_count,
            }
            for r in results
            if r.status in {"ok", "cached"}
        ],
        "failures": [
            {
                "content_hash": r.content_hash,
                "primary_path": r.primary_path,
                "output_dir": r.output_dir,
                "status": "error",
                "duration_seconds": r.duration_seconds,
                "error": r.error,
            }
            for r in results
            if r.status == "error"
        ],
    }


def write_corpus_manifest(manifest: dict, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(orjson.dumps(manifest, option=orjson.OPT_INDENT_2))


__all__ = [
    "DEFAULT_OUTPUT_ROOT",
    "DEFAULT_QUERY",
    "DEFAULT_SECTOR",
    "DEFAULT_LICENSE",
    "EXCERPT_MAX_CHARS",
    "CorpusExtractResult",
    "build_citation",
    "build_excerpt",
    "build_manifest",
    "cache_paths",
    "extract_corpus_from_manifest",
    "extract_one_from_cache",
    "heuristic_title",
    "heuristic_year",
    "is_extracted",
    "output_paths",
    "write_corpus_manifest",
]
