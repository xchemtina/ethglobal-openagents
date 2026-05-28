"""Offline tests: no MLX, no network, no chimia_kb.

Validates the worker's deterministic surface end-to-end against the
committed ``fixtures/sample_synthesis.json`` (now MolADT-based, no SMILES).
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from literature_synthesis.extract import (
    extract_synthesis,
    load_synthesis,
    write_synthesis,
)
from literature_synthesis.licensing import (
    DEFAULT_LICENSE_WHITELIST,
    is_open_access,
    normalize_license,
)
from literature_synthesis.moladt import Molecule
from literature_synthesis.moladt_validate import is_valid_molecule
from literature_synthesis.prompts import PROMPT_TEMPLATE_VERSION, PromptInputs, prompt_hash
from literature_synthesis.runtime import (
    ENV_RUNTIME,
    OfflineRuntime,
    parse_json_object,
    select_runtime,
)
from literature_synthesis.schema import (
    LiteratureCitation,
    LiteratureIngestManifest,
    LiteratureRuntime,
    LiteratureSource,
    LiteratureSourceKind,
    LiteratureSynthesis,
)


FIXTURE_PATH = (
    Path(__file__).resolve().parent.parent / "fixtures" / "sample_synthesis.json"
)


# ---------------------------------------------------------------------------- schema


def test_fixture_round_trips_through_schema() -> None:
    synthesis = load_synthesis(FIXTURE_PATH)
    assert isinstance(synthesis, LiteratureSynthesis)
    assert synthesis.query == "industrial ammonia synthesis"
    assert len(synthesis.citations) == 1
    assert synthesis.model_provenance.runtime == LiteratureRuntime.mlx_local


def test_fixture_carries_structural_molecules_not_strings() -> None:
    synthesis = load_synthesis(FIXTURE_PATH)
    for cand in synthesis.molecule_candidates:
        assert isinstance(cand.molecule, Molecule)
        assert is_valid_molecule(cand.molecule)
        dumped = cand.model_dump()
        assert "smiles" not in dumped


def test_fixture_n2_has_two_pi_bonding_systems() -> None:
    """N2 triple bond is encoded as one σ + two perpendicular π pools."""
    synthesis = load_synthesis(FIXTURE_PATH)
    n2 = next(
        c.molecule for c in synthesis.molecule_candidates if c.name == "dinitrogen"
    )
    assert len(n2.atoms) == 2
    assert len(n2.local_bonds) == 1
    assert len(n2.systems) == 2
    assert all(s.shared_electrons == 2 for s in n2.systems)


def test_fixture_haber_bosch_conditions_are_kelvin_and_bar() -> None:
    synthesis = load_synthesis(FIXTURE_PATH)
    rxn = synthesis.reaction_candidates[0].reaction
    kinds = {c.kind for c in rxn.conditions}
    assert kinds == {"temperature", "pressure"}


def test_fixture_json_serialises_to_canonical_wire_shape() -> None:
    synthesis = load_synthesis(FIXTURE_PATH)
    blob = synthesis.model_dump(mode="json")
    assert blob["model_provenance"]["runtime"] == "mlx-local"
    assert {"molecule_candidates", "reaction_candidates", "model_provenance"} <= set(blob)


# ---------------------------------------------------------------------------- licensing


def test_license_whitelist_accepts_known_open_access() -> None:
    for raw in (
        "cc-by",
        "https://creativecommons.org/licenses/by/4.0/",
        "ARXIV-PERPETUAL",
        "cc0",
    ):
        assert is_open_access(raw), raw


def test_license_whitelist_rejects_unknown() -> None:
    for raw in (None, "", "all-rights-reserved", "elsevier-restricted"):
        assert not is_open_access(raw)


def test_normalize_license_handles_creativecommons_urls() -> None:
    assert normalize_license("https://creativecommons.org/licenses/by-sa/4.0/") == "cc-by-sa"
    assert normalize_license("https://creativecommons.org/publicdomain/zero/1.0/") == "publicdomain"


# ---------------------------------------------------------------------------- prompts


def test_prompt_template_version_is_v4_moladt() -> None:
    assert PROMPT_TEMPLATE_VERSION == "v4"


def test_prompt_hash_is_stable() -> None:
    inputs = PromptInputs(
        query="industrial ammonia synthesis",
        sector="industrial-catalysis",
        citation_titles=["A", "B"],
        excerpt_digests=["blake3:1", "blake3:2"],
    )
    assert prompt_hash(inputs) == prompt_hash(inputs)
    assert prompt_hash(inputs).startswith("blake3:")


def test_prompt_hash_changes_on_input() -> None:
    a = PromptInputs(query="q", sector="s", citation_titles=["A"], excerpt_digests=["d"])
    b = PromptInputs(query="q2", sector="s", citation_titles=["A"], excerpt_digests=["d"])
    assert prompt_hash(a) != prompt_hash(b)


# ---------------------------------------------------------------------------- runtime


def test_select_runtime_offline_uses_fixture() -> None:
    runtime = select_runtime(offline_fixture=FIXTURE_PATH, forced="offline")
    assert isinstance(runtime, OfflineRuntime)
    parsed = json.loads(runtime.generate("ignored prompt"))
    assert parsed["query"] == "industrial ammonia synthesis"


def test_select_runtime_unknown_raises() -> None:
    with pytest.raises(Exception):
        select_runtime(forced="not-a-runtime")


# ---------------------------------------------------------------------------- extract


def _make_manifest() -> LiteratureIngestManifest:
    return LiteratureIngestManifest(
        query="industrial ammonia synthesis",
        sector="industrial-catalysis",
        requested_at_unix=1_700_000_000,
        max_papers=1,
        sources=[
            LiteratureSource(
                kind=LiteratureSourceKind.arxiv,
                identifier="example.haber",
                url="https://arxiv.org/abs/example.haber",
                license_hint="cc-by",
            ),
        ],
        local_dir=None,
        license_whitelist=list(DEFAULT_LICENSE_WHITELIST),
    )


def _single_citation() -> list[LiteratureCitation]:
    return [
        LiteratureCitation(
            title="The Haber-Bosch process: a centennial review",
            authors=["Doe, J."],
            year=2018,
            doi="10.1000/example.haber-bosch",
            source_url="https://arxiv.org/abs/example.haber",
            license="cc-by",
            retrieved_at_unix=1_700_000_000,
        ),
    ]


def test_extract_offline_end_to_end() -> None:
    manifest = _make_manifest()
    citations = _single_citation()
    runtime = OfflineRuntime(fixture_path=FIXTURE_PATH)
    synthesis = extract_synthesis(
        manifest=manifest,
        citations=citations,
        excerpts=["the Haber-Bosch process combines N2 and H2"],
        runtime=runtime,
    )
    assert synthesis.summary
    assert len(synthesis.extracted_claims) == 1
    assert len(synthesis.molecule_candidates) == 3
    assert len(synthesis.reaction_candidates) == 1
    for cand in synthesis.molecule_candidates:
        assert cand.source_citation_index == 0
        assert is_valid_molecule(cand.molecule)


def test_extract_offline_is_deterministic(tmp_path: Path) -> None:
    manifest = _make_manifest()
    citations = _single_citation()
    runtime = OfflineRuntime(fixture_path=FIXTURE_PATH)
    a = extract_synthesis(
        manifest=manifest, citations=citations, excerpts=["alpha"], runtime=runtime
    )
    b = extract_synthesis(
        manifest=manifest, citations=citations, excerpts=["alpha"], runtime=runtime
    )
    write_synthesis(a, tmp_path / "a.json")
    write_synthesis(b, tmp_path / "b.json")
    assert (tmp_path / "a.json").read_bytes() == (tmp_path / "b.json").read_bytes()


def test_extract_drops_invalid_adt(tmp_path: Path) -> None:
    """A fixture with a dangling-edge molecule must be silently dropped."""
    manifest = _make_manifest()
    citations = _single_citation()
    bad = {
        "query": manifest.query,
        "sector": manifest.sector,
        "summary": "broken",
        "citations": [],
        "extracted_claims": [],
        "conflicts": [],
        "molecule_candidates": [
            {
                "name": "broken",
                "role": "target",
                "source_citation_index": 0,
                "evidence_span": "alpha",
                "molecule": {
                    "atoms": [
                        {
                            "atom_id": 0,
                            "symbol": "C",
                            "coordinate": {"x": 0.0, "y": 0.0, "z": 0.0},
                            "formal_charge": 0,
                        }
                    ],
                    "local_bonds": [[0, 1]],
                    "systems": [],
                },
            }
        ],
        "reaction_candidates": [],
        "model_provenance": {
            "runtime": "mlx-local",
            "model_id": "fixture",
            "model_version": None,
            "model_path": None,
            "temperature": 0.0,
            "prompt_hash": "blake3:test",
            "deterministic": True,
        },
    }
    bad_path = tmp_path / "bad.json"
    bad_path.write_text(json.dumps(bad))
    runtime = OfflineRuntime(fixture_path=bad_path)
    synthesis = extract_synthesis(
        manifest=manifest,
        citations=citations,
        excerpts=["alpha"],
        runtime=runtime,
        drop_invalid_adt=True,
    )
    assert synthesis.molecule_candidates == []


# ---------------------------------------------------------------------------- env


def test_env_var_runtime_default_is_mlx(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv(ENV_RUNTIME, raising=False)
    runtime = select_runtime()
    assert runtime.runtime == LiteratureRuntime.mlx_local


# ---------------------------------------------------------------------------- parse_json_object


def test_parse_json_object_plain() -> None:
    assert parse_json_object('{"k": 1}') == {"k": 1}


def test_parse_json_object_strips_markdown_fence() -> None:
    raw = '```json\n{"k": 2}\n```'
    assert parse_json_object(raw) == {"k": 2}


def test_parse_json_object_handles_reasoning_preamble() -> None:
    raw = (
        "Thinking Process:\n"
        "1. The user wants a JSON synthesis.\n"
        '\nHere is the result: {"summary": "ok", "molecule_candidates": []}\n'
        "That should satisfy the schema.\n"
    )
    assert parse_json_object(raw) == {"summary": "ok", "molecule_candidates": []}


def test_parse_json_object_handles_nested_braces() -> None:
    raw = (
        "Reasoning... here is the JSON:\n"
        '{"outer": {"inner": [1, 2, {"deep": "x}"}]}}'
        "\nAnd that is all."
    )
    assert parse_json_object(raw) == {"outer": {"inner": [1, 2, {"deep": "x}"}]}}


def test_parse_json_object_raises_with_snippet_when_no_object() -> None:
    with pytest.raises(ValueError, match="no JSON object found"):
        parse_json_object("no json here, just text")
