"""Structural validator for MolADT objects.

This module checks invariants that the Haskell side of the project would
enforce via its constructors. It is deliberately schema-only: no chemistry
semantics (valence, formal-charge balance, isomorphism) is verified here.
The intent is to catch the structurally-broken outputs an LLM is most likely
to emit, so they can be silently dropped before being persisted into a
``LiteratureSynthesis`` artifact.

A valid ``Molecule`` satisfies:

1. Atom ids are non-negative and unique.
2. Every σ-bond edge ``(i, j)`` is canonical (``i <= j``), references existing
   atoms, has ``i != j``, and is unique across ``local_bonds``.
3. Each ``BondingSystem.member_edges`` is itself a set of canonical edges
   over existing atoms, with no self-loops.
4. ``BondingSystem.system_id`` values are unique within a molecule.

A valid ``Reaction`` satisfies:

1. ``reactants`` and ``products`` are both non-empty.
2. Every ``StoichiometryEntry.coefficient`` is strictly positive (already
   enforced by the Pydantic model; we re-check defensively).
3. Every contained ``Molecule`` passes ``validate_molecule``.
4. Every ``Condition`` has a strictly positive scalar value.
"""

from __future__ import annotations

from typing import List

from .moladt import (
    BondingSystem,
    CatalystCondition,
    Edge,
    Molecule,
    PressureCondition,
    Reaction,
    SolventCondition,
    StoichiometryEntry,
    TemperatureCondition,
    canonical_edge,
)


class MolADTValidationError(ValueError):
    """Raised when a Molecule or Reaction fails a structural invariant."""


# --------------------------------------------------------------------------- helpers


def _check_edge(edge: Edge, atom_ids: set[int], where: str) -> None:
    i, j = edge
    if (i, j) != canonical_edge(i, j):
        raise MolADTValidationError(
            f"{where}: edge ({i}, {j}) is not canonical (require i <= j)"
        )
    if i == j:
        raise MolADTValidationError(f"{where}: self-loop on atom {i} is not allowed")
    if i not in atom_ids or j not in atom_ids:
        raise MolADTValidationError(
            f"{where}: edge ({i}, {j}) references atoms not in the molecule"
        )


def _check_unique(edges: List[Edge], where: str) -> None:
    seen: set[Edge] = set()
    for e in edges:
        if e in seen:
            raise MolADTValidationError(f"{where}: duplicate edge {e}")
        seen.add(e)


# --------------------------------------------------------------------------- molecule


def validate_molecule(mol: Molecule) -> None:
    """Raise ``MolADTValidationError`` if ``mol`` violates an invariant."""
    if not mol.atoms:
        raise MolADTValidationError("molecule has no atoms")

    atom_ids: set[int] = set()
    for atom in mol.atoms:
        if atom.atom_id in atom_ids:
            raise MolADTValidationError(
                f"duplicate atom_id {atom.atom_id} in molecule"
            )
        atom_ids.add(atom.atom_id)

    for edge in mol.local_bonds:
        _check_edge(edge, atom_ids, where="local_bonds")
    _check_unique(list(mol.local_bonds), where="local_bonds")

    system_ids: set[int] = set()
    for system in mol.systems:
        _check_bonding_system(system, atom_ids, system_ids)


def _check_bonding_system(
    system: BondingSystem, atom_ids: set[int], seen_ids: set[int]
) -> None:
    if system.system_id in seen_ids:
        raise MolADTValidationError(
            f"duplicate system_id {system.system_id} in molecule"
        )
    seen_ids.add(system.system_id)

    if not system.member_edges:
        raise MolADTValidationError(
            f"system_id={system.system_id} has no member_edges"
        )
    where = f"bonding_system(system_id={system.system_id})"
    for edge in system.member_edges:
        _check_edge(edge, atom_ids, where=where)
    _check_unique(list(system.member_edges), where=where)


def is_valid_molecule(mol: Molecule) -> bool:
    """Non-raising counterpart of :func:`validate_molecule`."""
    try:
        validate_molecule(mol)
    except MolADTValidationError:
        return False
    return True


# --------------------------------------------------------------------------- reaction


def validate_reaction(rxn: Reaction) -> None:
    """Raise ``MolADTValidationError`` if ``rxn`` violates an invariant."""
    if not rxn.reactants:
        raise MolADTValidationError("reaction has no reactants")
    if not rxn.products:
        raise MolADTValidationError("reaction has no products")

    for idx, entry in enumerate(rxn.reactants):
        _check_stoich(entry, where=f"reactants[{idx}]")
    for idx, entry in enumerate(rxn.products):
        _check_stoich(entry, where=f"products[{idx}]")

    for idx, cond in enumerate(rxn.conditions):
        _check_condition(cond, where=f"conditions[{idx}]")

    # rate may legitimately be 0.0 (unknown); we only require non-negative.
    if rxn.rate < 0.0:
        raise MolADTValidationError(f"rate must be non-negative, got {rxn.rate}")


def _check_stoich(entry: StoichiometryEntry, where: str) -> None:
    if entry.coefficient <= 0.0:
        raise MolADTValidationError(
            f"{where}: coefficient must be > 0, got {entry.coefficient}"
        )
    try:
        validate_molecule(entry.molecule)
    except MolADTValidationError as exc:
        raise MolADTValidationError(f"{where}: {exc}") from exc


def _check_condition(cond, where: str) -> None:  # noqa: ANN001 -- Condition is a Union
    if isinstance(cond, TemperatureCondition):
        if cond.kelvin <= 0.0:
            raise MolADTValidationError(
                f"{where}: temperature must be > 0 K, got {cond.kelvin}"
            )
    elif isinstance(cond, PressureCondition):
        if cond.bar <= 0.0:
            raise MolADTValidationError(
                f"{where}: pressure must be > 0 bar, got {cond.bar}"
            )
    elif isinstance(cond, (CatalystCondition, SolventCondition)):
        try:
            validate_molecule(cond.molecule)
        except MolADTValidationError as exc:
            raise MolADTValidationError(f"{where}: {exc}") from exc
    else:
        raise MolADTValidationError(f"{where}: unsupported condition kind {type(cond)}")


def is_valid_reaction(rxn: Reaction) -> bool:
    """Non-raising counterpart of :func:`validate_reaction`."""
    try:
        validate_reaction(rxn)
    except MolADTValidationError:
        return False
    return True


__all__ = [
    "MolADTValidationError",
    "validate_molecule",
    "is_valid_molecule",
    "validate_reaction",
    "is_valid_reaction",
]
