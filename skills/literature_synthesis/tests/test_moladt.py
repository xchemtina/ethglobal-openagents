"""Tests for the MolADT mirror and its structural validator.

Each rejection path the validator implements is exercised by name. Two known-
good molecules (methane and benzene) anchor the positive cases, with benzene
specifically chosen to exercise a Dietz π bonding system.
"""

from __future__ import annotations

import json

import pytest

from literature_synthesis.moladt import (
    Atom,
    AtomicSymbol,
    BondingSystem,
    CatalystCondition,
    Coordinate,
    Molecule,
    PressureCondition,
    Reaction,
    SolventCondition,
    StoichiometryEntry,
    TemperatureCondition,
    canonical_edge,
    canonicalise_molecule,
)
from literature_synthesis.moladt_validate import (
    MolADTValidationError,
    is_valid_molecule,
    is_valid_reaction,
    validate_molecule,
    validate_reaction,
)


# --------------------------------------------------------------------------- fixtures


def _atom(aid: int, sym: AtomicSymbol, *, x: float = 0.0, y: float = 0.0, z: float = 0.0) -> Atom:
    return Atom(atom_id=aid, symbol=sym, coordinate=Coordinate(x=x, y=y, z=z))


def methane() -> Molecule:
    """CH4 with the carbon at the origin and tetrahedral hydrogens."""
    return Molecule(
        atoms=[
            _atom(0, AtomicSymbol.C),
            _atom(1, AtomicSymbol.H, x=1.09),
            _atom(2, AtomicSymbol.H, x=-1.09),
            _atom(3, AtomicSymbol.H, y=1.09),
            _atom(4, AtomicSymbol.H, y=-1.09),
        ],
        local_bonds=[(0, 1), (0, 2), (0, 3), (0, 4)],
    )


def benzene() -> Molecule:
    """C6H6: six C-C σ + six C-H σ + one π bonding system over the ring."""
    carbons = [_atom(i, AtomicSymbol.C) for i in range(6)]
    hydrogens = [_atom(i + 6, AtomicSymbol.H) for i in range(6)]
    cc_edges = [canonical_edge(i, (i + 1) % 6) for i in range(6)]
    ch_edges = [canonical_edge(i, i + 6) for i in range(6)]
    pi_pool = BondingSystem(
        system_id=0,
        shared_electrons=6,
        member_edges=cc_edges,
        tag="pi_ring",
    )
    return Molecule(
        atoms=carbons + hydrogens,
        local_bonds=cc_edges + ch_edges,
        systems=[pi_pool],
    )


# --------------------------------------------------------------------------- positive


def test_methane_validates() -> None:
    validate_molecule(methane())
    assert is_valid_molecule(methane())


def test_benzene_with_pi_system_validates() -> None:
    mol = benzene()
    validate_molecule(mol)
    # Sanity: π pool has 6 electrons over 6 ring edges.
    assert mol.systems[0].shared_electrons == 6
    assert len(mol.systems[0].member_edges) == 6


def test_canonical_edge_is_idempotent() -> None:
    assert canonical_edge(2, 5) == (2, 5)
    assert canonical_edge(5, 2) == (2, 5)


def test_canonicalise_molecule_sorts_atoms_and_edges() -> None:
    """Same chemistry, different field order → identical JSON after canonicalise."""
    a = Molecule(
        atoms=[_atom(1, AtomicSymbol.H), _atom(0, AtomicSymbol.H)],
        local_bonds=[(1, 0)],  # non-canonical
    )
    b = Molecule(
        atoms=[_atom(0, AtomicSymbol.H), _atom(1, AtomicSymbol.H)],
        local_bonds=[(0, 1)],
    )
    assert canonicalise_molecule(a).model_dump() == canonicalise_molecule(b).model_dump()


# --------------------------------------------------------------------------- molecule rejections


def test_empty_atom_list_is_rejected() -> None:
    with pytest.raises(MolADTValidationError, match="no atoms"):
        validate_molecule(Molecule(atoms=[]))


def test_duplicate_atom_id_is_rejected() -> None:
    mol = Molecule(
        atoms=[_atom(0, AtomicSymbol.C), _atom(0, AtomicSymbol.H)],
        local_bonds=[],
    )
    with pytest.raises(MolADTValidationError, match="duplicate atom_id"):
        validate_molecule(mol)


def test_dangling_edge_is_rejected() -> None:
    mol = Molecule(
        atoms=[_atom(0, AtomicSymbol.C), _atom(1, AtomicSymbol.H)],
        local_bonds=[(0, 2)],  # atom 2 does not exist
    )
    with pytest.raises(MolADTValidationError, match="not in the molecule"):
        validate_molecule(mol)


def test_non_canonical_edge_is_rejected() -> None:
    mol = Molecule(
        atoms=[_atom(0, AtomicSymbol.C), _atom(1, AtomicSymbol.H)],
        local_bonds=[(1, 0)],  # j < i
    )
    with pytest.raises(MolADTValidationError, match="not canonical"):
        validate_molecule(mol)


def test_self_loop_is_rejected() -> None:
    mol = Molecule(
        atoms=[_atom(0, AtomicSymbol.C)],
        local_bonds=[(0, 0)],
    )
    with pytest.raises(MolADTValidationError, match="self-loop"):
        validate_molecule(mol)


def test_duplicate_edge_is_rejected() -> None:
    mol = Molecule(
        atoms=[_atom(0, AtomicSymbol.C), _atom(1, AtomicSymbol.H)],
        local_bonds=[(0, 1), (0, 1)],
    )
    with pytest.raises(MolADTValidationError, match="duplicate edge"):
        validate_molecule(mol)


def test_negative_shared_electrons_is_rejected_at_construction() -> None:
    """Pydantic catches this before the validator sees it."""
    with pytest.raises(Exception):  # noqa: B017 -- pydantic raises ValidationError
        BondingSystem(
            system_id=0, shared_electrons=-1, member_edges=[(0, 1)]
        )


def test_bonding_system_with_dangling_edge_is_rejected() -> None:
    mol = Molecule(
        atoms=[_atom(0, AtomicSymbol.C)],
        local_bonds=[],
        systems=[
            BondingSystem(
                system_id=0, shared_electrons=2, member_edges=[(0, 1)]
            )
        ],
    )
    with pytest.raises(MolADTValidationError, match="not in the molecule"):
        validate_molecule(mol)


def test_duplicate_system_id_is_rejected() -> None:
    mol = Molecule(
        atoms=[_atom(0, AtomicSymbol.C), _atom(1, AtomicSymbol.C)],
        local_bonds=[(0, 1)],
        systems=[
            BondingSystem(system_id=0, shared_electrons=2, member_edges=[(0, 1)]),
            BondingSystem(system_id=0, shared_electrons=2, member_edges=[(0, 1)]),
        ],
    )
    with pytest.raises(MolADTValidationError, match="duplicate system_id"):
        validate_molecule(mol)


# --------------------------------------------------------------------------- reaction


def _stoich(coeff: float, mol: Molecule) -> StoichiometryEntry:
    return StoichiometryEntry(coefficient=coeff, molecule=mol)


def test_methane_combustion_reaction_validates() -> None:
    """CH4 + 2 O2 → CO2 + 2 H2O (structures left minimal)."""
    o2 = Molecule(
        atoms=[_atom(0, AtomicSymbol.O), _atom(1, AtomicSymbol.O)],
        local_bonds=[(0, 1)],
    )
    co2 = Molecule(
        atoms=[
            _atom(0, AtomicSymbol.C),
            _atom(1, AtomicSymbol.O),
            _atom(2, AtomicSymbol.O),
        ],
        local_bonds=[(0, 1), (0, 2)],
    )
    h2o = Molecule(
        atoms=[
            _atom(0, AtomicSymbol.O),
            _atom(1, AtomicSymbol.H),
            _atom(2, AtomicSymbol.H),
        ],
        local_bonds=[(0, 1), (0, 2)],
    )
    rxn = Reaction(
        reactants=[_stoich(1.0, methane()), _stoich(2.0, o2)],
        products=[_stoich(1.0, co2), _stoich(2.0, h2o)],
        conditions=[TemperatureCondition(kelvin=1200.0)],
        rate=0.0,
    )
    validate_reaction(rxn)
    assert is_valid_reaction(rxn)


def test_zero_coefficient_is_rejected_at_construction() -> None:
    with pytest.raises(Exception):  # noqa: B017
        StoichiometryEntry(coefficient=0.0, molecule=methane())


def test_negative_temperature_is_rejected_at_construction() -> None:
    with pytest.raises(Exception):  # noqa: B017
        TemperatureCondition(kelvin=-1.0)


def test_negative_pressure_is_rejected_at_construction() -> None:
    with pytest.raises(Exception):  # noqa: B017
        PressureCondition(bar=-0.5)


def test_reaction_without_reactants_is_rejected() -> None:
    co2 = Molecule(
        atoms=[
            _atom(0, AtomicSymbol.C),
            _atom(1, AtomicSymbol.O),
            _atom(2, AtomicSymbol.O),
        ],
        local_bonds=[(0, 1), (0, 2)],
    )
    rxn = Reaction(reactants=[], products=[_stoich(1.0, co2)])
    with pytest.raises(MolADTValidationError, match="no reactants"):
        validate_reaction(rxn)


def test_reaction_with_invalid_molecule_is_rejected() -> None:
    bad_mol = Molecule(
        atoms=[_atom(0, AtomicSymbol.C)],
        local_bonds=[(0, 1)],  # dangling
    )
    rxn = Reaction(
        reactants=[_stoich(1.0, methane())],
        products=[_stoich(1.0, bad_mol)],
    )
    with pytest.raises(MolADTValidationError, match=r"products\[0\]"):
        validate_reaction(rxn)


# --------------------------------------------------------------------------- json round-trip


def test_molecule_json_round_trips_byte_identically_after_canonicalise() -> None:
    a = canonicalise_molecule(benzene())
    b = canonicalise_molecule(
        Molecule.model_validate_json(a.model_dump_json())
    )
    assert a.model_dump_json() == b.model_dump_json()


def test_condition_tagged_union_round_trips() -> None:
    t = TemperatureCondition(kelvin=773.15)
    payload = json.loads(t.model_dump_json())
    assert payload == {"kind": "temperature", "kelvin": 773.15}
    assert TemperatureCondition.model_validate(payload).kelvin == 773.15

    p = PressureCondition(bar=200.0)
    payload = json.loads(p.model_dump_json())
    assert payload == {"kind": "pressure", "bar": 200.0}


# --------------------------------------------------------------------------- extended symbols


def test_extended_atomic_symbols_are_available() -> None:
    """Spot-check that the wider periodic-table coverage is wired in."""
    for raw in ("Cu", "Si", "Ge", "Sn", "Ru", "Ti", "Pt", "Au", "La", "U"):
        # Construction via the enum value succeeds.
        sym = AtomicSymbol(raw)
        assert sym.value == raw
        # And the symbol is usable inside an Atom that passes validation.
        mol = Molecule(
            atoms=[
                _atom(0, sym),
                _atom(1, AtomicSymbol.O),
            ],
            local_bonds=[(0, 1)],
        )
        validate_molecule(mol)


def test_old_smiles_only_elements_reject() -> None:
    """Astatine and other non-supported symbols are rejected at construction."""
    for raw in ("At", "Po", "Fr", "Ra", "Es"):
        with pytest.raises(Exception):  # noqa: B017 -- pydantic ValueError on enum miss
            AtomicSymbol(raw)


# --------------------------------------------------------------------------- catalyst/solvent conditions


def _ru_centre() -> Molecule:
    """Trivial 'Ru-only' placeholder molecule (single atom). Sufficient for
    validator tests; real Ru catalysts have full coordination spheres."""
    return Molecule(atoms=[_atom(0, AtomicSymbol.Ru)])


def test_catalyst_condition_with_valid_molecule_passes() -> None:
    cond = CatalystCondition(molecule=_ru_centre())
    rxn = Reaction(
        reactants=[_stoich(1.0, methane())],
        products=[_stoich(1.0, methane())],
        conditions=[cond],
    )
    validate_reaction(rxn)
    payload = json.loads(cond.model_dump_json())
    assert payload["kind"] == "catalyst"
    assert payload["molecule"]["atoms"][0]["symbol"] == "Ru"


def test_solvent_condition_with_valid_molecule_passes() -> None:
    # THF C4H8O: minimal structure -- just the 5-atom ring skeleton.
    thf = Molecule(
        atoms=[
            _atom(0, AtomicSymbol.O),
            _atom(1, AtomicSymbol.C),
            _atom(2, AtomicSymbol.C),
            _atom(3, AtomicSymbol.C),
            _atom(4, AtomicSymbol.C),
        ],
        local_bonds=[(0, 1), (1, 2), (2, 3), (3, 4), (0, 4)],
    )
    cond = SolventCondition(molecule=thf)
    rxn = Reaction(
        reactants=[_stoich(1.0, methane())],
        products=[_stoich(1.0, methane())],
        conditions=[cond],
    )
    validate_reaction(rxn)


def test_catalyst_condition_with_invalid_molecule_is_rejected() -> None:
    bad = Molecule(
        atoms=[_atom(0, AtomicSymbol.Ru)],
        local_bonds=[(0, 1)],  # dangling
    )
    rxn = Reaction(
        reactants=[_stoich(1.0, methane())],
        products=[_stoich(1.0, methane())],
        conditions=[CatalystCondition(molecule=bad)],
    )
    with pytest.raises(MolADTValidationError, match=r"conditions\[0\]"):
        validate_reaction(rxn)
