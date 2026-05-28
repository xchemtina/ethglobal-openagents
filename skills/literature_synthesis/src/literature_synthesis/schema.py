"""Pydantic mirrors of the literature-synthesis payload types.

Molecule and reaction candidates now carry MolADT structural types (see
``moladt.py``) rather than SMILES strings, mirroring the Haskell ADTs in
``MolADT-Bayes``. SMILES is intentionally absent from this schema.
"""

from __future__ import annotations

from enum import Enum
from typing import List, Optional

from pydantic import BaseModel, ConfigDict, Field

from .moladt import Molecule, Reaction


class LiteratureSourceKind(str, Enum):
    arxiv = "arxiv"
    chemrxiv = "chemrxiv"
    crossref = "crossref"
    openalex = "openalex"
    unpaywall = "unpaywall"
    local_pdf = "local_pdf"


class MoleculeRole(str, Enum):
    target = "target"
    precursor = "precursor"
    catalyst = "catalyst"
    reagent = "reagent"
    solvent = "solvent"
    byproduct = "byproduct"
    other = "other"


class LiteratureRuntime(str, Enum):
    mlx_local = "mlx-local"
    local_ollama = "local-ollama"
    openrouter = "openrouter"
    openai = "openai"
    clojure_rag = "clojure-rag"
    rlm = "rlm"


class _Frozen(BaseModel):
    """Base for all payload models. Strict, frozen, exclude-defaults-friendly."""

    model_config = ConfigDict(extra="forbid", frozen=True)


class LiteratureSource(_Frozen):
    kind: LiteratureSourceKind
    identifier: str
    url: str
    license_hint: str


class LiteratureCitation(_Frozen):
    title: str
    authors: List[str]
    year: int
    doi: Optional[str] = None
    source_url: Optional[str] = None
    license: str
    retrieved_at_unix: int


class ExtractedClaim(_Frozen):
    claim: str
    evidence_span: str
    source_citation_index: int


class MoleculeCandidate(_Frozen):
    """A molecule extracted from a paper, represented as a MolADT ``Molecule``.

    ``name`` is a free-text label for human use only (e.g. "benzene") and
    plays no role in the structural representation. ``role`` retains the same
    metadata semantics as before.
    """

    name: str
    molecule: Molecule
    role: MoleculeRole
    source_citation_index: int
    evidence_span: str


class ReactionCandidate(_Frozen):
    """A reaction extracted from a paper, represented as a MolADT ``Reaction``.

    The wrapper preserves the evidence span and confidence the LLM used to
    propose it; the chemistry itself lives entirely inside ``reaction``.
    """

    reaction: Reaction
    confidence: float = Field(ge=0.0, le=1.0)
    evidence_span: str
    source_citation_index: int


class ModelProvenance(_Frozen):
    runtime: LiteratureRuntime
    model_id: str
    model_version: Optional[str] = None
    model_path: Optional[str] = None
    temperature: float
    prompt_hash: str
    deterministic: bool


class LiteratureIngestManifest(_Frozen):
    query: str
    sector: str
    requested_at_unix: int
    max_papers: int
    sources: List[LiteratureSource]
    local_dir: Optional[str] = None
    license_whitelist: List[str]


class LiteratureSynthesis(_Frozen):
    query: str
    sector: str
    summary: str
    citations: List[LiteratureCitation]
    extracted_claims: List[ExtractedClaim]
    conflicts: List[str]
    molecule_candidates: List[MoleculeCandidate]
    reaction_candidates: List[ReactionCandidate]
    model_provenance: ModelProvenance


__all__ = [
    "LiteratureSourceKind",
    "MoleculeRole",
    "LiteratureRuntime",
    "LiteratureSource",
    "LiteratureCitation",
    "ExtractedClaim",
    "MoleculeCandidate",
    "ReactionCandidate",
    "ModelProvenance",
    "LiteratureIngestManifest",
    "LiteratureSynthesis",
]
