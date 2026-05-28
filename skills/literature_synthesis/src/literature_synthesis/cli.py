"""Click-based CLI: ingest / extract / sign / run-fixture.

The Rust constructor is invoked separately by `chimiaclaw-cli` once the
synthesis JSON is on disk; this CLI is concerned only with producing a
publishable JSON file.
"""

from __future__ import annotations

import sys
import time
from pathlib import Path
from typing import Optional

import click
import orjson

from .bulk_ingest import (
    DEFAULT_BATCH_SIZE,
    DEFAULT_PER_PAPER_TIMEOUT_SECONDS,
    bulk_ingest,
    bulk_ingest_entries,
    write_manifest,
)
from .corpus_extract import (
    DEFAULT_LICENSE as CORPUS_DEFAULT_LICENSE,
    DEFAULT_OUTPUT_ROOT as CORPUS_DEFAULT_OUTPUT_ROOT,
    DEFAULT_QUERY as CORPUS_DEFAULT_QUERY,
    DEFAULT_SECTOR as CORPUS_DEFAULT_SECTOR,
    EXCERPT_MAX_CHARS as CORPUS_DEFAULT_EXCERPT_CHARS,
    extract_corpus_from_manifest,
    write_corpus_manifest,
)
from .extract import extract_synthesis, load_synthesis, write_synthesis
from .ingest import run_ingest
from .manifest_ops import (
    load_failed_entries,
    load_manifest,
    load_missing_cache_entries,
    merge_manifest,
)
from .runtime import select_runtime
from .schema import LiteratureCitation, LiteratureIngestManifest


@click.group(help="ChimiaClaw Literature lane worker.")
@click.version_option(package_name="literature-synthesis")
def main() -> None:
    pass


@main.command()
@click.option("--query", required=True, help="Free-text search query.")
@click.option("--sector", default="general-chemistry", show_default=True)
@click.option("--max-papers", "max_papers", type=int, default=6, show_default=True)
@click.option(
    "--out",
    "out_dir",
    type=click.Path(file_okay=False, path_type=Path),
    required=True,
    help="Output directory for downloaded PDFs and the manifest JSON.",
)
@click.option(
    "--crossref-email",
    default="info@chimiadao.io",
    show_default=True,
    help="Email used for the Crossref polite pool.",
)
@click.option(
    "--enrich/--no-enrich",
    "enrich",
    default=True,
    help="Run or skip the Crossref enrichment pass.",
)
def ingest(
    query: str,
    sector: str,
    max_papers: int,
    out_dir: Path,
    crossref_email: str,
    enrich: bool,
) -> None:
    """Pull open-access papers and emit a science.literature.ingest manifest."""
    manifest = run_ingest(
        query=query,
        sector=sector,
        max_papers=max_papers,
        out_dir=out_dir,
        crossref_email=crossref_email,
        enrich=enrich,
    )
    click.echo(f"Wrote manifest with {len(manifest.sources)} source(s) to {out_dir}/manifest.json")


@main.command()
@click.option(
    "--manifest",
    "manifest_path",
    type=click.Path(dir_okay=False, exists=True, path_type=Path),
    required=True,
    help="Path to a science.literature.ingest manifest JSON.",
)
@click.option(
    "--excerpts",
    "excerpts_path",
    type=click.Path(dir_okay=False, exists=True, path_type=Path),
    required=True,
    help="JSON file with {citations: [...], excerpts: [...] } aligned by index.",
)
@click.option(
    "--out",
    "out_path",
    type=click.Path(dir_okay=False, path_type=Path),
    required=True,
    help="Path to write the science.literature.synthesis JSON.",
)
@click.option(
    "--offline-fixture",
    type=click.Path(dir_okay=False, exists=True, path_type=Path),
    default=None,
    help="When set, force the offline runtime and read raw model output from this file.",
)
@click.option(
    "--runtime",
    "forced_runtime",
    type=str,
    default=None,
    help="Force a specific runtime (mlx-local, local-ollama, openai, openrouter, offline).",
)
def extract(
    manifest_path: Path,
    excerpts_path: Path,
    out_path: Path,
    offline_fixture: Optional[Path],
    forced_runtime: Optional[str],
) -> None:
    """Run extraction over the supplied excerpts and write the synthesis JSON."""
    manifest = LiteratureIngestManifest.model_validate(
        orjson.loads(manifest_path.read_bytes())
    )
    excerpts_doc = orjson.loads(excerpts_path.read_bytes())
    citations = [LiteratureCitation.model_validate(c) for c in excerpts_doc["citations"]]
    excerpts = [str(s) for s in excerpts_doc["excerpts"]]

    forced = forced_runtime or ("offline" if offline_fixture else None)
    runtime = select_runtime(offline_fixture=offline_fixture, forced=forced)
    synthesis = extract_synthesis(
        manifest=manifest,
        citations=citations,
        excerpts=excerpts,
        runtime=runtime,
    )
    write_synthesis(synthesis, out_path)
    click.echo(
        f"Wrote synthesis with {len(synthesis.extracted_claims)} claim(s), "
        f"{len(synthesis.molecule_candidates)} molecule(s), "
        f"{len(synthesis.reaction_candidates)} reaction(s) to {out_path}"
    )


@main.command()
@click.option(
    "--synthesis",
    "synthesis_path",
    type=click.Path(dir_okay=False, exists=True, path_type=Path),
    required=True,
)
def show(synthesis_path: Path) -> None:
    """Pretty-print a synthesis JSON, useful in pipelines."""
    synthesis = load_synthesis(synthesis_path)
    click.echo(
        orjson.dumps(synthesis.model_dump(mode="json"), option=orjson.OPT_INDENT_2).decode()
    )


@main.command("bulk-ingest")
@click.option(
    "--root",
    "roots",
    type=click.Path(exists=True, path_type=Path),
    multiple=True,
    required=False,
    help="Root directory (or single PDF) to ingest. Repeatable. Required unless --from-manifest is used.",
)
@click.option(
    "--cache-dir",
    type=click.Path(file_okay=False, path_type=Path),
    default=Path.home() / ".chimia_kb" / "docling",
    show_default=True,
    help="Per-paper Docling cache root.",
)
@click.option(
    "--manifest",
    "manifest_path",
    type=click.Path(dir_okay=False, path_type=Path),
    required=True,
    help="Path to write the bulk-ingest manifest JSON. With --from-manifest this is also the input file (results are merged in place).",
)
@click.option(
    "--from-manifest",
    "from_manifest_path",
    type=click.Path(dir_okay=False, exists=True, path_type=Path),
    default=None,
    help="Resume from an existing manifest. Without --retry-failed/--retry-missing-cache, this is just used to seed cache_root + roots.",
)
@click.option(
    "--retry-failed",
    is_flag=True,
    default=False,
    help="With --from-manifest, only re-parse the manifest's `failures` rows.",
)
@click.option(
    "--retry-missing-cache",
    is_flag=True,
    default=False,
    help="With --from-manifest, also re-parse rows whose cache directory is missing on disk.",
)
@click.option(
    "--limit",
    type=int,
    default=None,
    help="Stop after N unique papers (useful for validation runs).",
)
@click.option(
    "--workers",
    type=int,
    default=None,
    help="Process pool size. Defaults to cpu_count // 2; set to 1 for serial.",
)
@click.option(
    "--per-paper-timeout-seconds",
    type=float,
    default=DEFAULT_PER_PAPER_TIMEOUT_SECONDS,
    show_default=True,
    help="No-progress timeout per pool batch. If no future completes within this window the batch is aborted and surviving entries marked as worker crashes.",
)
@click.option(
    "--max-rss-mb",
    type=int,
    default=None,
    help="If set, install RLIMIT_AS on each worker (turns runaway memory growth into MemoryError instead of system OOM-kill).",
)
@click.option(
    "--batch-size",
    type=int,
    default=DEFAULT_BATCH_SIZE,
    show_default=True,
    help="Number of papers per pool. Smaller batches isolate OOM crashes; larger batches amortize pool startup.",
)
@click.option(
    "--force/--no-force",
    default=False,
    help="Re-parse cached papers (default: skip).",
)
@click.option(
    "--dry-run",
    is_flag=True,
    default=False,
    help="Discover + dedup but do not invoke Docling.",
)
def bulk_ingest_cmd(
    roots: tuple[Path, ...],
    cache_dir: Path,
    manifest_path: Path,
    from_manifest_path: Optional[Path],
    retry_failed: bool,
    retry_missing_cache: bool,
    limit: Optional[int],
    workers: Optional[int],
    per_paper_timeout_seconds: float,
    max_rss_mb: Optional[int],
    batch_size: int,
    force: bool,
    dry_run: bool,
) -> None:
    """Discover PDFs across `--root`s, dedup, parse with Docling, write a manifest.

    Use `--from-manifest <path> --retry-failed` to resume after a crashed run
    without re-discovering or re-hashing the corpus.
    """
    last_stage = {"name": None, "total": 0, "start": time.monotonic()}

    def progress(stage: str, current: int, total: int, info) -> None:
        if stage != last_stage["name"]:
            last_stage["name"] = stage
            last_stage["total"] = total
            last_stage["start"] = time.monotonic()
            click.echo(f"[{stage}] starting ({total} item(s))", err=True)
        if total and (current % max(1, total // 50) == 0 or current == total):
            elapsed = time.monotonic() - last_stage["start"]
            rate = current / elapsed if elapsed > 0 else 0.0
            label = info if isinstance(info, str) else info.get("primary_path", "?")
            click.echo(
                f"[{stage}] {current}/{total} ({rate:.1f}/s) {Path(label).name if label else ''}",
                err=True,
            )

    # ---- Retry path: resume from an existing manifest -----------------------
    if from_manifest_path is not None and (retry_failed or retry_missing_cache):
        if dry_run:
            raise click.UsageError("--dry-run is not supported with --retry-* modes.")
        base_manifest = load_manifest(from_manifest_path)
        entries = []
        if retry_failed:
            entries.extend(load_failed_entries(base_manifest))
        if retry_missing_cache:
            entries.extend(
                load_missing_cache_entries(base_manifest, cache_dir)
            )
        # Dedup by content_hash; keep deterministic order.
        seen: set[str] = set()
        deduped = []
        for entry in entries:
            if entry.content_hash in seen:
                continue
            seen.add(entry.content_hash)
            deduped.append(entry)
        deduped.sort(key=lambda e: e.content_hash)
        if limit is not None:
            deduped = deduped[:limit]

        click.echo(
            f"[retry] {len(deduped)} entries to re-parse from {from_manifest_path}",
            err=True,
        )
        if not deduped:
            click.echo("[retry] nothing to do", err=True)
            return

        retry_manifest = bulk_ingest_entries(
            deduped,
            cache_root=cache_dir,
            workers=workers,
            force=force,
            per_paper_timeout_seconds=per_paper_timeout_seconds,
            max_rss_mb=max_rss_mb,
            batch_size=batch_size,
            progress_cb=progress,
            roots=base_manifest.get("roots"),
        )
        # Write retry-only sidecar for forensics.
        sidecar = manifest_path.with_suffix(
            f".retry-{int(time.time())}.json"
        )
        write_manifest(retry_manifest, sidecar)

        merged = merge_manifest(
            base_manifest,
            (retry_manifest.get("papers") or [])
            + (retry_manifest.get("failures") or []),
        )
        write_manifest(merged, manifest_path)
        totals = merged["totals"]
        click.echo(
            f"retry merge complete: {totals['ok']} ok, {totals['cached']} cached, "
            f"{totals['error']} errored / {totals['papers_total']} total. "
            f"merged manifest -> {manifest_path} (sidecar -> {sidecar})"
        )
        return

    # ---- Normal path: full discover + parse ---------------------------------
    if not roots:
        raise click.UsageError(
            "Either --root <path> (repeatable) or --from-manifest <path> --retry-* must be provided."
        )

    if dry_run:
        from .corpus import build_corpus

        entries = build_corpus(
            [Path(r) for r in roots],
            progress_cb=lambda c, t, p: progress("hash", c, t, str(p)),
        )
        if limit is not None:
            entries = entries[:limit]
        manifest = {
            "schema_tag": "chimiaclaw.literature.bulk_ingest.v1",
            "cache_root": str(cache_dir.expanduser().resolve()),
            "roots": [str(Path(r).expanduser().resolve()) for r in roots],
            "dry_run": True,
            "totals": {"papers_total": len(entries)},
            "papers": [
                {
                    "content_hash": entry.content_hash,
                    "primary_path": entry.primary_path,
                    "source_paths": list(entry.source_paths),
                    "size_bytes": entry.size_bytes,
                    "mtime_unix": entry.mtime_unix,
                    "status": "discovered",
                }
                for entry in entries
            ],
            "failures": [],
        }
        write_manifest(manifest, manifest_path)
        click.echo(
            f"discovered {len(entries)} unique papers (dry-run); manifest written to {manifest_path}"
        )
        return

    manifest = bulk_ingest(
        roots=[Path(r) for r in roots],
        cache_root=cache_dir,
        workers=workers,
        limit=limit,
        force=force,
        per_paper_timeout_seconds=per_paper_timeout_seconds,
        max_rss_mb=max_rss_mb,
        batch_size=batch_size,
        progress_cb=progress,
    )
    write_manifest(manifest, manifest_path)
    totals = manifest["totals"]
    click.echo(
        f"bulk-ingest complete: {totals['ok']} parsed, {totals['cached']} cached, "
        f"{totals['error']} errored / {totals['papers_total']} total in "
        f"{manifest['elapsed_seconds']}s. manifest -> {manifest_path}"
    )


@main.command("extract-corpus")
@click.option(
    "--from-manifest",
    "from_manifest_path",
    type=click.Path(dir_okay=False, exists=True, path_type=Path),
    required=True,
    help="Bulk-ingest manifest produced by `bulk-ingest`. Eligible papers (status ok/cached) are extracted.",
)
@click.option(
    "--cache-dir",
    type=click.Path(file_okay=False, path_type=Path),
    default=Path.home() / ".chimia_kb" / "docling",
    show_default=True,
    help="Docling cache root (must match the bulk-ingest run).",
)
@click.option(
    "--output-dir",
    type=click.Path(file_okay=False, path_type=Path),
    default=CORPUS_DEFAULT_OUTPUT_ROOT,
    show_default=True,
    help="Per-paper synthesis artifact root. One <2hex>/<hash>/synthesis.json per paper.",
)
@click.option(
    "--corpus-manifest",
    "corpus_manifest_path",
    type=click.Path(dir_okay=False, path_type=Path),
    default=None,
    help="Where to write the corpus-level manifest. Defaults to <output-dir>/corpus-extract.json.",
)
@click.option(
    "--query",
    default=CORPUS_DEFAULT_QUERY,
    show_default=True,
    help="Free-text query passed to the prompt for every paper.",
)
@click.option("--sector", default=CORPUS_DEFAULT_SECTOR, show_default=True)
@click.option(
    "--license",
    "corpus_license",
    default=CORPUS_DEFAULT_LICENSE,
    show_default=True,
    help="License sentinel applied to every per-paper LiteratureCitation.",
)
@click.option(
    "--excerpt-max-chars",
    type=int,
    default=CORPUS_DEFAULT_EXCERPT_CHARS,
    show_default=True,
)
@click.option(
    "--runtime",
    "forced_runtime",
    type=str,
    default=None,
    help="Force a specific runtime (mlx-local, local-ollama, openai, openrouter, offline). Default: env/CHIMIACLAW_LITERATURE_RUNTIME.",
)
@click.option(
    "--offline-fixture",
    type=click.Path(dir_okay=False, exists=True, path_type=Path),
    default=None,
    help="With --runtime offline, force the runtime to read this fixture for every paper (used by tests + smoke runs).",
)
@click.option(
    "--limit",
    type=int,
    default=None,
    help="Stop after N eligible papers (smoke runs).",
)
@click.option(
    "--force/--no-force",
    default=False,
    help="Re-extract even if a synthesis.json already exists.",
)
@click.option(
    "--skip-existing/--no-skip-existing",
    default=True,
    help="If a synthesis.json already exists, count it as cached without re-reading.",
)
def extract_corpus_cmd(
    from_manifest_path: Path,
    cache_dir: Path,
    output_dir: Path,
    corpus_manifest_path: Optional[Path],
    query: str,
    sector: str,
    corpus_license: str,
    excerpt_max_chars: int,
    forced_runtime: Optional[str],
    offline_fixture: Optional[Path],
    limit: Optional[int],
    force: bool,
    skip_existing: bool,
) -> None:
    """Layer extraction on top of a Docling cache, one synthesis.json per paper."""
    bulk_manifest = load_manifest(from_manifest_path)
    forced = forced_runtime or ("offline" if offline_fixture else None)
    runtime = select_runtime(offline_fixture=offline_fixture, forced=forced)

    last_status: dict[str, int] = {}

    def progress(index: int, total: int, result) -> None:
        last_status["index"] = index
        last_status["total"] = total
        if total and (index % max(1, total // 50) == 0 or index == total):
            click.echo(
                f"[extract] {index}/{total} {result.status}: "
                f"{Path(result.primary_path).name} "
                f"({result.molecule_count}m/{result.reaction_count}r/{result.claim_count}c)",
                err=True,
            )
        if result.status == "error":
            click.echo(
                f"[extract] error on {Path(result.primary_path).name}: {result.error}",
                err=True,
            )

    manifest = extract_corpus_from_manifest(
        bulk_manifest=bulk_manifest,
        cache_root=cache_dir,
        output_root=output_dir,
        runtime=runtime,
        query=query,
        sector=sector,
        license=corpus_license,
        excerpt_max_chars=excerpt_max_chars,
        force=force,
        skip_existing=skip_existing,
        limit=limit,
        progress_cb=progress,
    )
    out_path = corpus_manifest_path or (output_dir / "corpus-extract.json")
    write_corpus_manifest(manifest, out_path)
    totals = manifest["totals"]
    click.echo(
        f"extract-corpus complete: {totals['ok']} extracted, {totals['cached']} cached, "
        f"{totals['error']} errored / {totals['papers_total']} total in "
        f"{manifest['elapsed_seconds']}s. corpus manifest -> {out_path}"
    )


@main.command("run-fixture")
@click.option(
    "--fixture",
    type=click.Path(dir_okay=False, exists=True, path_type=Path),
    required=True,
    help="A LiteratureSynthesis JSON fixture used by the offline runtime.",
)
@click.option(
    "--out",
    "out_path",
    type=click.Path(dir_okay=False, path_type=Path),
    required=True,
)
def run_fixture(fixture: Path, out_path: Path) -> None:
    """Convenience wrapper: read a synthesis fixture, validate it, write it back.

    Used by `cargo run -p chimiaclaw-cli -- science-literature-demo` to feed the
    Rust constructor an offline-deterministic LiteratureSynthesis without
    spinning up the LLM runtime at all.
    """
    synthesis = load_synthesis(fixture)
    write_synthesis(synthesis, out_path)
    click.echo(f"Wrote {out_path} (offline fixture passthrough)")


if __name__ == "__main__":  # pragma: no cover
    main()
