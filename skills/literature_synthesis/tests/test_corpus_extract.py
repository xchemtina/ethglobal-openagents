"""Corpus-extract tests.

Cover three things:
* The cheap heuristic helpers (title/year/excerpt) behave deterministically
  on edge cases (missing H1, year only in filename, oversize markdown).
* `extract_one_from_cache` produces a synthesis.json + meta.json on success
  and a synthesis-error.json when the runtime explodes, never raising.
* `extract_corpus_from_manifest` is resumable: a second pass over the same
  bulk manifest reports every paper as `cached` and never invokes the
  runtime.
"""

from __future__ import annotations

from pathlib import Path
from typing import Optional

import orjson
import pytest

from literature_synthesis.corpus_extract import (
    DEFAULT_LICENSE,
    DEFAULT_QUERY,
    build_excerpt,
    cache_paths,
    extract_corpus_from_manifest,
    extract_one_from_cache,
    heuristic_title,
    heuristic_year,
    is_extracted,
    output_paths,
)
from literature_synthesis.schema import LiteratureRuntime


# ---------------------------------------------------------------------------
# Stub runtime: returns a deterministic single-citation synthesis blob.
# ---------------------------------------------------------------------------


class _StubRuntime:
    """Runtime that returns a parseable synthesis object every time."""

    runtime = LiteratureRuntime.mlx_local
    model_id = "stub-model"
    model_path = None

    def __init__(self, payload: Optional[str] = None) -> None:
        self._payload = payload or orjson.dumps(
            {
                "summary": "stub summary",
                "extracted_claims": [
                    {
                        "claim": "stub claim",
                        "evidence_span": "stub evidence",
                        "source_citation_index": 0,
                    }
                ],
                "conflicts": [],
                "molecule_candidates": [
                    {
                        "name": "methane",
                        "role": "target",
                        "source_citation_index": 0,
                        "evidence_span": "methane was observed",
                        "molecule": {
                            "atoms": [
                                {
                                    "atom_id": i,
                                    "symbol": s,
                                    "coordinate": {"x": 0.0, "y": 0.0, "z": 0.0},
                                    "formal_charge": 0,
                                }
                                for i, s in enumerate(["C", "H", "H", "H", "H"])
                            ],
                            "local_bonds": [[0, 1], [0, 2], [0, 3], [0, 4]],
                            "systems": [],
                        },
                    }
                ],
                "reaction_candidates": [],
            }
        ).decode()
        self.calls = 0

    def generate(self, prompt: str) -> str:  # noqa: ARG002
        self.calls += 1
        return self._payload


class _BombRuntime:
    runtime = LiteratureRuntime.mlx_local
    model_id = "bomb"
    model_path = None
    calls = 0

    def generate(self, prompt: str) -> str:  # noqa: ARG002
        self.calls += 1
        raise RuntimeError("simulated runtime failure")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _seed_cache(cache_root: Path, content_hash: str, markdown: str) -> Path:
    cache_dir, doc_md, meta = cache_paths(cache_root, content_hash)
    cache_dir.mkdir(parents=True, exist_ok=True)
    doc_md.write_text(markdown, encoding="utf-8")
    meta.write_bytes(orjson.dumps({"content_hash": content_hash, "status": "ok"}))
    return cache_dir


def _bulk_manifest_for(*entries: tuple[str, str]) -> dict:
    return {
        "schema_tag": "chimiaclaw.literature.bulk_ingest.v1",
        "papers": [
            {"content_hash": ch, "primary_path": pp, "status": "ok"}
            for ch, pp in entries
        ],
        "failures": [],
        "totals": {
            "papers_total": len(entries),
            "ok": len(entries),
            "cached": 0,
            "error": 0,
        },
    }


# ---------------------------------------------------------------------------
# Heuristics
# ---------------------------------------------------------------------------


def test_heuristic_title_uses_h1_then_falls_back() -> None:
    assert heuristic_title("# Real Title\nbody", fallback="paper") == "Real Title"
    assert heuristic_title("body without heading", fallback="paper") == "paper"
    # Empty H1 falls back too.
    assert heuristic_title("# \nbody", fallback="paper") == "paper"


def test_heuristic_year_prefers_filename() -> None:
    md = "Some prose mentioning 1987 in the body."
    assert heuristic_year(md, primary_path="/p/Doe_2024_paper.pdf") == 2024
    # No year in filename: text fallback wins.
    assert heuristic_year(md, primary_path="/p/paper.pdf") == 1987
    # Neither: defaults to 0.
    assert heuristic_year("no years here", primary_path="/p/paper.pdf") == 0


def test_build_excerpt_truncates_and_cleans() -> None:
    body = "\n".join(
        [
            "# Title",
            "",
            "Real content paragraph.",
            "|---|---|",  # Markdown table separator: should be dropped.
            "More content.",
        ]
        + ["filler line " + str(i) for i in range(10000)]
    )
    excerpt = build_excerpt(body, max_chars=200)
    assert len(excerpt) == 200
    assert "|---|---|" not in excerpt
    # The leading H1 is preserved (cleaning only drops blanks + table seps).
    assert excerpt.startswith("# Title")


# ---------------------------------------------------------------------------
# extract_one_from_cache
# ---------------------------------------------------------------------------


def test_extract_one_from_cache_writes_synthesis_and_meta(tmp_path: Path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "out"
    content_hash = "a" * 64
    primary_path = str(tmp_path / "fake.pdf")
    _seed_cache(cache, content_hash, "# Stub Paper 2023\nBody mentioning methane.")

    rt = _StubRuntime()
    result = extract_one_from_cache(
        content_hash=content_hash,
        primary_path=primary_path,
        cache_root=cache,
        output_root=out,
        runtime=rt,
    )
    assert result.status == "ok"
    assert result.molecule_count == 1
    assert result.claim_count == 1
    assert rt.calls == 1

    out_dir, synthesis_path, _ = output_paths(out, content_hash)
    assert synthesis_path.exists()
    assert (out_dir / "synthesis-meta.json").exists()
    # The next call hits the on-disk cache: runtime not invoked.
    second = extract_one_from_cache(
        content_hash=content_hash,
        primary_path=primary_path,
        cache_root=cache,
        output_root=out,
        runtime=rt,
    )
    assert second.status == "cached"
    assert rt.calls == 1


def test_extract_one_from_cache_missing_doc_md_records_error(tmp_path: Path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "out"
    content_hash = "b" * 64
    # Deliberately do NOT seed cache.
    result = extract_one_from_cache(
        content_hash=content_hash,
        primary_path=str(tmp_path / "missing.pdf"),
        cache_root=cache,
        output_root=out,
        runtime=_StubRuntime(),
    )
    assert result.status == "error"
    assert "missing cache" in (result.error or "")
    _, synthesis_path, error_path = output_paths(out, content_hash)
    assert not synthesis_path.exists()
    assert error_path.exists()


def test_extract_one_from_cache_runtime_failure_records_error(tmp_path: Path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "out"
    content_hash = "c" * 64
    primary_path = str(tmp_path / "p.pdf")
    _seed_cache(cache, content_hash, "# Paper\nbody")

    bomb = _BombRuntime()
    result = extract_one_from_cache(
        content_hash=content_hash,
        primary_path=primary_path,
        cache_root=cache,
        output_root=out,
        runtime=bomb,
    )
    assert result.status == "error"
    assert "RuntimeError" in (result.error or "")
    assert bomb.calls == 1
    _, synthesis_path, error_path = output_paths(out, content_hash)
    assert not synthesis_path.exists()
    assert error_path.exists()


# ---------------------------------------------------------------------------
# extract_corpus_from_manifest
# ---------------------------------------------------------------------------


def test_extract_corpus_is_resumable(tmp_path: Path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "out"
    a = "a" * 64
    b = "b" * 64
    pa = str(tmp_path / "a.pdf")
    pb = str(tmp_path / "b.pdf")
    _seed_cache(cache, a, "# Alpha 2022\nbody A")
    _seed_cache(cache, b, "# Beta 2023\nbody B")

    bulk = _bulk_manifest_for((a, pa), (b, pb))
    rt = _StubRuntime()
    manifest = extract_corpus_from_manifest(
        bulk_manifest=bulk,
        cache_root=cache,
        output_root=out,
        runtime=rt,
    )
    assert manifest["totals"] == {
        "papers_total": 2,
        "ok": 2,
        "cached": 0,
        "error": 0,
    }
    assert rt.calls == 2

    # Re-run: every paper is cached, runtime never called again.
    manifest2 = extract_corpus_from_manifest(
        bulk_manifest=bulk,
        cache_root=cache,
        output_root=out,
        runtime=rt,
    )
    assert manifest2["totals"]["cached"] == 2
    assert manifest2["totals"]["ok"] == 0
    assert rt.calls == 2  # unchanged


def test_extract_corpus_continues_on_runtime_error(tmp_path: Path) -> None:
    cache = tmp_path / "cache"
    out = tmp_path / "out"
    a = "a" * 64
    b = "b" * 64
    pa = str(tmp_path / "a.pdf")
    pb = str(tmp_path / "b.pdf")
    _seed_cache(cache, a, "# Alpha\nbody A")
    _seed_cache(cache, b, "# Beta\nbody B")

    bulk = _bulk_manifest_for((a, pa), (b, pb))

    class _SwitchRuntime:
        runtime = LiteratureRuntime.mlx_local
        model_id = "switch"
        model_path = None

        def __init__(self) -> None:
            self.calls = 0
            self._stub = _StubRuntime()

        def generate(self, prompt: str) -> str:
            self.calls += 1
            if self.calls == 1:
                raise RuntimeError("first paper bombs")
            return self._stub.generate(prompt)

    rt = _SwitchRuntime()
    manifest = extract_corpus_from_manifest(
        bulk_manifest=bulk,
        cache_root=cache,
        output_root=out,
        runtime=rt,
    )
    assert manifest["totals"]["error"] == 1
    assert manifest["totals"]["ok"] == 1
    # Both papers got attempted, in deterministic content_hash order.
    assert rt.calls == 2
    assert len(manifest["failures"]) == 1
    assert len(manifest["papers"]) == 1
    assert is_extracted(out, manifest["papers"][0]["content_hash"])
    assert not is_extracted(out, manifest["failures"][0]["content_hash"])
