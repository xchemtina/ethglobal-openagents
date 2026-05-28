"""Citation-grounded extraction.

Takes a `LiteratureIngestManifest` plus a small set of source excerpts, runs
the configured LLM runtime, validates molecule and reaction candidates as
MolADT structural ADTs, and returns a typed `LiteratureSynthesis`. Candidates
that fail the structural validator are silently dropped to keep the noise
out of the persisted artifact.
"""

from __future__ import annotations

from pathlib import Path
from typing import Iterable, List, Mapping

import blake3
import orjson
from pydantic import ValidationError

from .moladt_validate import is_valid_molecule, is_valid_reaction
from .prompts import PromptInputs, prompt_hash, render_prompt
from .runtime import GenerationRuntime, parse_json_object
from .schema import (
    ExtractedClaim,
    LiteratureCitation,
    LiteratureIngestManifest,
    LiteratureRuntime,
    LiteratureSynthesis,
    ModelProvenance,
    MoleculeCandidate,
    MoleculeRole,
    ReactionCandidate,
)


def _digest(text: str) -> str:
    return f"blake3:{blake3.blake3(text.encode('utf-8')).hexdigest()}"


def _format_block(items: Iterable[str]) -> str:
    lines = []
    for index, item in enumerate(items):
        lines.append(f"[{index}] {item}")
    return "\n".join(lines)


def build_prompt(
    manifest: LiteratureIngestManifest,
    citation_titles: List[str],
    excerpts: List[str],
) -> tuple[str, str]:
    """Render the prompt + return its deterministic hash."""
    inputs = PromptInputs(
        query=manifest.query,
        sector=manifest.sector,
        citation_titles=citation_titles,
        excerpt_digests=[_digest(excerpt) for excerpt in excerpts],
    )
    prompt = render_prompt(
        inputs,
        citations_block=_format_block(citation_titles),
        excerpts_block=_format_block(excerpts),
    )
    return prompt, prompt_hash(inputs)


def extract_synthesis(
    *,
    manifest: LiteratureIngestManifest,
    citations: List[LiteratureCitation],
    excerpts: List[str],
    runtime: GenerationRuntime,
    drop_invalid_adt: bool = True,
) -> LiteratureSynthesis:
    """Run extraction end to end and return a validated `LiteratureSynthesis`.

    `excerpts` is a list of source text excerpts indexed positionally with
    `citations`; the LLM is told to cite by index. Candidates that violate a
    MolADT structural invariant are dropped silently when
    `drop_invalid_adt=True` so the artifact stays publishable.
    """
    if len(citations) != len(excerpts):
        raise ValueError("citations and excerpts must align by index")
    if not citations:
        raise ValueError("at least one citation is required")

    prompt, ph = build_prompt(manifest, [c.title for c in citations], excerpts)
    raw = runtime.generate(prompt)
    parsed = parse_json_object(raw)

    claims = _materialise_claims(parsed.get("extracted_claims", []), len(citations))
    molecules = _materialise_molecules(
        parsed.get("molecule_candidates", []),
        len(citations),
        drop_invalid_adt=drop_invalid_adt,
    )
    reactions = _materialise_reactions(
        parsed.get("reaction_candidates", []),
        len(citations),
        drop_invalid_adt=drop_invalid_adt,
    )

    provenance = ModelProvenance(
        runtime=runtime.runtime,
        model_id=runtime.model_id,
        model_version=None,
        model_path=runtime.model_path,
        temperature=0.0,
        prompt_hash=ph,
        deterministic=runtime.runtime
        in {LiteratureRuntime.mlx_local, LiteratureRuntime.local_ollama},
    )
    return LiteratureSynthesis(
        query=manifest.query,
        sector=manifest.sector,
        summary=str(parsed.get("summary", "")).strip(),
        citations=citations,
        extracted_claims=claims,
        conflicts=[str(c) for c in parsed.get("conflicts", []) if str(c).strip()],
        molecule_candidates=molecules,
        reaction_candidates=reactions,
        model_provenance=provenance,
    )


def _materialise_claims(items: Iterable[Mapping], count: int) -> List[ExtractedClaim]:
    out: List[ExtractedClaim] = []
    for item in items:
        try:
            claim = ExtractedClaim(**dict(item))
        except (ValidationError, TypeError):
            continue
        if 0 <= claim.source_citation_index < count and claim.evidence_span.strip():
            out.append(claim)
    return out


def _materialise_molecules(
    items: Iterable[Mapping],
    count: int,
    *,
    drop_invalid_adt: bool,
) -> List[MoleculeCandidate]:
    out: List[MoleculeCandidate] = []
    for item in items:
        try:
            payload = dict(item)
            role_value = str(payload.get("role", "other")).lower()
            payload["role"] = (
                role_value if role_value in MoleculeRole.__members__ else "other"
            )
            cand = MoleculeCandidate(**payload)
        except (ValidationError, TypeError):
            continue
        if not (0 <= cand.source_citation_index < count):
            continue
        if not cand.evidence_span.strip():
            continue
        if drop_invalid_adt and not is_valid_molecule(cand.molecule):
            continue
        out.append(cand)
    return out


def _materialise_reactions(
    items: Iterable[Mapping],
    count: int,
    *,
    drop_invalid_adt: bool,
) -> List[ReactionCandidate]:
    out: List[ReactionCandidate] = []
    for item in items:
        try:
            cand = ReactionCandidate(**dict(item))
        except (ValidationError, TypeError):
            continue
        if not (0 <= cand.source_citation_index < count):
            continue
        if not cand.evidence_span.strip():
            continue
        if drop_invalid_adt and not is_valid_reaction(cand.reaction):
            continue
        out.append(cand)
    return out


def write_synthesis(synthesis: LiteratureSynthesis, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        orjson.dumps(synthesis.model_dump(mode="json"), option=orjson.OPT_INDENT_2)
    )


def load_synthesis(path: Path) -> LiteratureSynthesis:
    return LiteratureSynthesis.model_validate(orjson.loads(path.read_bytes()))


__all__ = [
    "build_prompt",
    "extract_synthesis",
    "write_synthesis",
    "load_synthesis",
]
