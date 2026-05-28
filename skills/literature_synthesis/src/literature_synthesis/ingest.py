"""Open-access ingest.

Phase 1 reuses the existing paper-RAG (`chimia_kb.ingestion`) rather than
reinventing arXiv / Crossref / ChemRxiv plumbing. This module is a thin
adapter that:

* Calls `chimia_kb`'s `PaperIngester` for the heavy lifting.
* Filters every retrieved paper through the open-access licence whitelist.
* Renames downloaded PDFs to the user-preferred `title_lastauthorlastname.pdf`
  convention via `chimia_kb.ingestion.utils.sanitize_filename`.
* Emits a `LiteratureIngestManifest` JSON suitable for the Rust constructor.

The function is intentionally lazy: it imports `chimia_kb` only when called
so the offline test path does not require the optional `ingest` extra.
"""

from __future__ import annotations

import time
from pathlib import Path
from typing import List, Optional

import orjson

from .licensing import DEFAULT_LICENSE_WHITELIST, is_open_access, normalize_license
from .schema import LiteratureIngestManifest, LiteratureSource, LiteratureSourceKind


def _import_chimia_kb():
    """Import `chimia_kb` lazily so the offline path stays import-free."""
    try:
        from chimia_kb.ingestion import paper_ingester  # type: ignore

        return paper_ingester
    except Exception as exc:
        raise RuntimeError(
            "chimia_kb is not importable. Install with `uv pip install -e "
            "~/Documents/ChimiaDAO-PaperIngestion` and retry."
        ) from exc


def run_ingest(
    *,
    query: str,
    sector: str,
    max_papers: int,
    out_dir: Path,
    crossref_email: str = "info@chimiadao.io",
    enrich: bool = True,
    license_whitelist: tuple[str, ...] = DEFAULT_LICENSE_WHITELIST,
) -> LiteratureIngestManifest:
    """Run a real arXiv + Crossref ingest and return a typed manifest."""
    import asyncio

    paper_ingester = _import_chimia_kb()

    out_dir = out_dir.expanduser()
    out_dir.mkdir(parents=True, exist_ok=True)

    ingester = paper_ingester.PaperIngester(
        data_dir=out_dir,
        crossref_email=crossref_email,
        download_pdfs=True,
        skip_existing=True,
    )

    asyncio.run(
        ingester.ingest_query(
            query=query,
            max_results=max_papers,
            enrich=enrich,
        )
    )

    sources = _scan_outputs(out_dir, license_whitelist)
    manifest = LiteratureIngestManifest(
        query=query,
        sector=sector,
        requested_at_unix=int(time.time()),
        max_papers=max_papers,
        sources=sources,
        local_dir=str(out_dir),
        license_whitelist=list(license_whitelist),
    )
    write_manifest(manifest, out_dir / "manifest.json")
    return manifest


def _scan_outputs(
    out_dir: Path, license_whitelist: tuple[str, ...]
) -> List[LiteratureSource]:
    """Walk the chimia_kb output directory and emit `LiteratureSource` rows.

    `chimia_kb` already drops a per-paper JSON sidecar with metadata; we read
    those and apply the licence whitelist. Anything not whitelisted is dropped.
    """
    sources: List[LiteratureSource] = []
    for sidecar in sorted(out_dir.rglob("*.json")):
        if sidecar.name == "manifest.json":
            continue
        try:
            record = orjson.loads(sidecar.read_bytes())
        except Exception:
            continue
        kind = _infer_kind(record)
        url = (
            record.get("pdf_url")
            or record.get("url")
            or record.get("source_url")
            or ""
        )
        identifier = (
            record.get("arxiv_id")
            or record.get("doi")
            or record.get("identifier")
            or sidecar.stem
        )
        license_hint = normalize_license(record.get("license") or record.get("license_hint"))
        if not is_open_access(license_hint, license_whitelist):
            continue
        sources.append(
            LiteratureSource(
                kind=kind,
                identifier=str(identifier),
                url=str(url),
                license_hint=license_hint,
            )
        )
    return sources


def _infer_kind(record: dict) -> LiteratureSourceKind:
    """Best-effort: pick a source kind from a chimia_kb sidecar."""
    if record.get("arxiv_id"):
        return LiteratureSourceKind.arxiv
    if record.get("doi"):
        return LiteratureSourceKind.crossref
    source = (record.get("source") or "").lower()
    if "arxiv" in source:
        return LiteratureSourceKind.arxiv
    if "chemrxiv" in source:
        return LiteratureSourceKind.chemrxiv
    if "openalex" in source:
        return LiteratureSourceKind.openalex
    if "unpaywall" in source:
        return LiteratureSourceKind.unpaywall
    if "crossref" in source:
        return LiteratureSourceKind.crossref
    return LiteratureSourceKind.local_pdf


def write_manifest(manifest: LiteratureIngestManifest, path: Optional[Path]) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        orjson.dumps(manifest.model_dump(mode="json"), option=orjson.OPT_INDENT_2)
    )


__all__ = ["run_ingest", "write_manifest"]
