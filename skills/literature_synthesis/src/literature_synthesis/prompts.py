"""Deterministic prompt templates.

A `prompt_hash` (Blake3) is recorded in every `ModelProvenance` so an audit
can reproduce the exact prompt without storing the canonicalised template
inline. The template version string is part of the hashed payload, so a
template change forces a new hash even if the input bytes are identical.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import List

import blake3
import orjson

PROMPT_TEMPLATE_VERSION = "v4"

PROMPT_TEMPLATE = """You are a chemistry literature analyst. You read open-access \
papers and produce a JSON object summarising what they say about the user query.

This pipeline does NOT use SMILES. Every molecule must be expressed as a \
structural MolADT object: a list of atoms with explicit integer ids, a list \
of undirected sigma-bond edges, and an optional list of Dietz bonding systems \
(electron pools shared across one or more edges, e.g. an aromatic pi ring).

Rules you must follow without exception:
1. Every claim, molecule candidate, and reaction candidate MUST cite a paper \
by index into the citations list you are given. The first paper has index 0.
2. Every claim MUST have a non-empty `evidence_span` that is a literal \
substring of the source paper's text.
3. Output STRICT JSON; no commentary, no markdown fences, no chain-of-thought.
4. Values shown in angle brackets below (e.g. <string>) are TYPE TAGS, not \
literal content. Never copy them. Either replace each tag with the real \
extracted value or return an empty array for that category.
5. If a category has nothing extractable, return `[]`. Never fabricate placeholders.
6. Atomic symbols MUST be one of (periods 1-6 + Th, U): \
H, He, Li, Be, B, C, N, O, F, Ne, Na, Mg, Al, Si, P, S, Cl, Ar, K, Ca, Sc, Ti, \
V, Cr, Mn, Fe, Co, Ni, Cu, Zn, Ga, Ge, As, Se, Br, Kr, Rb, Sr, Y, Zr, Nb, Mo, \
Tc, Ru, Rh, Pd, Ag, Cd, In, Sn, Sb, Te, I, Xe, Cs, Ba, La, Ce, Pr, Nd, Pm, \
Sm, Eu, Gd, Tb, Dy, Ho, Er, Tm, Yb, Lu, Hf, Ta, W, Re, Os, Ir, Pt, Au, Hg, \
Tl, Pb, Bi, Th, U. If a molecule contains any element outside this set (e.g. \
At, Po, Fr, transuranic), OMIT THE MOLECULE ENTIRELY. Do not substitute or \
transmute elements.
7. Atom ids are integers, unique within their molecule, starting at 0.
8. Each edge is a two-element list `[i, j]` with `i <= j` and `i != j`. \
Every edge must reference atom ids that exist in the molecule. Do not repeat \
an edge.
9. A bonding system has a non-negative integer `shared_electrons` count and a \
list of `member_edges` over which those electrons are delocalised. Use bonding \
systems to encode pi pools (aromatic rings, multiple bonds with shared pi \
density, multicenter bonds). Single-pair sigma bonds belong in `local_bonds`, \
NOT in a bonding system.
10. Conditions are tagged. Available variants:
    - `{{"kind": "temperature", "kelvin": <K>}}` for temperature in Kelvin.
    - `{{"kind": "pressure", "bar": <bar>}}` for pressure in bar.
    - `{{"kind": "catalyst", "molecule": <Molecule>}}` for a catalyst, expressed \
as a structural MolADT Molecule (NOT a string name). Catalysts are not \
consumed and so live in `conditions`, not `reactants`.
    - `{{"kind": "solvent", "molecule": <Molecule>}}` for a solvent, expressed \
the same way.
    Use only conditions explicitly reported in the paper. If you cannot \
represent a catalyst or solvent structurally (e.g. it contains an unsupported \
element or is too complex to encode), omit that condition entirely.
11. Stoichiometric coefficients are strictly positive floats.

Worked example -- benzene (C6H6) as a MolADT Molecule:
{{
  "name": "benzene",
  "role": "other",
  "source_citation_index": 0,
  "evidence_span": "benzene was used as the solvent",
  "molecule": {{
    "atoms": [
      {{"atom_id": 0, "symbol": "C", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 1, "symbol": "C", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 2, "symbol": "C", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 3, "symbol": "C", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 4, "symbol": "C", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 5, "symbol": "C", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 6, "symbol": "H", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 7, "symbol": "H", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 8, "symbol": "H", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 9, "symbol": "H", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 10, "symbol": "H", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}},
      {{"atom_id": 11, "symbol": "H", "coordinate": {{"x": 0, "y": 0, "z": 0}}, "formal_charge": 0}}
    ],
    "local_bonds": [
      [0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [0, 5],
      [0, 6], [1, 7], [2, 8], [3, 9], [4, 10], [5, 11]
    ],
    "systems": [
      {{"system_id": 0, "shared_electrons": 6,
        "member_edges": [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [0, 5]],
        "tag": "pi_ring"}}
    ]
  }}
}}

Full output schema (replace each angle-bracketed tag with a real value):
{{
  "summary": <2-4 sentence string describing what the paper reports>,
  "extracted_claims": [
    {{
      "claim": <string>,
      "evidence_span": <literal substring of the excerpt>,
      "source_citation_index": <integer 0..N-1>
    }}
  ],
  "conflicts": [<string>],
  "molecule_candidates": [
    {{
      "name": <chemical name string>,
      "role": <"target"|"precursor"|"catalyst"|"reagent"|"solvent"|"byproduct"|"other">,
      "source_citation_index": <integer 0..N-1>,
      "evidence_span": <literal substring of the excerpt>,
      "molecule": {{
        "atoms": [{{"atom_id": <int>, "symbol": <element>, "coordinate": {{"x": <float>, "y": <float>, "z": <float>}}, "formal_charge": <int>}}],
        "local_bonds": [[<int i>, <int j>]],
        "systems": [{{"system_id": <int>, "shared_electrons": <int>=0>, "member_edges": [[<int>, <int>]], "tag": <string or null>}}]
      }}
    }}
  ],
  "reaction_candidates": [
    {{
      "confidence": <float 0..1>,
      "evidence_span": <literal substring of the excerpt>,
      "source_citation_index": <integer 0..N-1>,
      "reaction": {{
        "reactants": [{{"coefficient": <float>0>, "molecule": <Molecule object as above>}}],
        "products":  [{{"coefficient": <float>0>, "molecule": <Molecule object as above>}}],
        "conditions": [
          {{"kind": "temperature", "kelvin": <float>}},
          {{"kind": "pressure", "bar": <float>}},
          {{"kind": "catalyst", "molecule": <Molecule object as above>}},
          {{"kind": "solvent",   "molecule": <Molecule object as above>}}
        ],
        "rate": <float>=0, defaults to 0.0>
      }}
    }}
  ]
}}

User query: {query}
Sector: {sector}
Citations (numbered):
{citations_block}

Source excerpts (numbered to match the citation indices):
{excerpts_block}

Return a single JSON object now. Begin with `{{` and end with `}}`. Do not \
echo the schema. Do not output the literal string "..." anywhere.
"""


@dataclass(frozen=True)
class PromptInputs:
    query: str
    sector: str
    citation_titles: List[str]
    excerpt_digests: List[str]


def render_prompt(
    inputs: PromptInputs,
    citations_block: str,
    excerpts_block: str,
) -> str:
    return PROMPT_TEMPLATE.format(
        query=inputs.query,
        sector=inputs.sector,
        citations_block=citations_block,
        excerpts_block=excerpts_block,
    )


def prompt_hash(inputs: PromptInputs) -> str:
    """Hash a canonical, deterministic representation of the prompt inputs.

    We hash the template version + structured inputs rather than the rendered
    string so a whitespace tweak in the template does not invalidate every
    historical artifact. Excerpts are pre-digested by the caller (Blake3 of
    the canonical PDF text bytes) so the prompt hash itself stays compact.
    """
    canonical = orjson.dumps(
        {
            "template_version": PROMPT_TEMPLATE_VERSION,
            "query": inputs.query,
            "sector": inputs.sector,
            "citation_titles": inputs.citation_titles,
            "excerpt_digests": inputs.excerpt_digests,
        },
        option=orjson.OPT_SORT_KEYS,
    )
    return f"blake3:{blake3.blake3(canonical).hexdigest()}"


__all__ = [
    "PROMPT_TEMPLATE_VERSION",
    "PROMPT_TEMPLATE",
    "PromptInputs",
    "render_prompt",
    "prompt_hash",
]
