"""Python mirror of the cross-language MolADT family.

The **canonical source of truth** for atomic-symbol coverage, schema tags, and
element metadata is the Rust crate ``chimiaclaw-moladt``
(``crates/chimiaclaw-moladt/src/lib.rs``) at the repository root. This module
mirrors the Rust enum so the Python literature-synthesis worker can read and
write the same ``chem.molecule.adt.v1`` JSON artifacts that the Rust pipeline
signs, the DFT worker consumes, and the Haskell ``MolADT-Bayes`` repo decodes.

Drift between this Python ``AtomicSymbol`` and the Rust manifest is caught by
``tests/test_moladt_rust_parity.py``, which parses the Rust source directly
and fails the suite if either side has gained or lost an element. To extend
coverage, **add the variant in Rust first** (one place: the enum body, the
four exhaustive matches, and the ``MOLADT_ELEMENT_MANIFEST`` row), then
append the matching string value here in atomic-number order.

JSON layout follows the Rust crate's `serde` defaults so a payload signed by
``chimiaclaw-moladt::molecule_artifact`` decodes here without translation.
``Set`` becomes a list (sorted before serialisation for determinism).
``BTreeMap<AtomId, Atom>`` becomes a list of ``Atom`` objects keyed by their
``atom_id`` field -- equivalent information, with a deterministic insertion
order on disk. ``Option<T>`` becomes ``Optional[T] = None``.

There is intentionally no SMILES, no InChI, no string-shaped chemistry of any
kind here. Every chemical concept is represented structurally.
"""

from __future__ import annotations

from enum import Enum
from typing import List, Literal, Optional, Tuple, Union

from pydantic import BaseModel, ConfigDict, Field


# --------------------------------------------------------------------------- atoms


class AtomicSymbol(str, Enum):
    """Mirror of Haskell ``AtomicSymbol``.

    Order and membership match ``Chem.Molecule.AtomicSymbol`` exactly. The
    declaration is in atomic-number (Z) order so the derived ``Ord`` on the
    Haskell side sorts sensibly. Coverage: periods 1–6 in full plus Th and U.
    If the Haskell enum is extended, extend this enum in the same commit,
    otherwise persisted artifacts cannot round-trip.
    """

    # Period 1
    H = "H"
    He = "He"
    # Period 2
    Li = "Li"
    Be = "Be"
    B = "B"
    C = "C"
    N = "N"
    O = "O"
    F = "F"
    Ne = "Ne"
    # Period 3
    Na = "Na"
    Mg = "Mg"
    Al = "Al"
    Si = "Si"
    P = "P"
    S = "S"
    Cl = "Cl"
    Ar = "Ar"
    # Period 4
    K = "K"
    Ca = "Ca"
    Sc = "Sc"
    Ti = "Ti"
    V = "V"
    Cr = "Cr"
    Mn = "Mn"
    Fe = "Fe"
    Co = "Co"
    Ni = "Ni"
    Cu = "Cu"
    Zn = "Zn"
    Ga = "Ga"
    Ge = "Ge"
    As = "As"
    Se = "Se"
    Br = "Br"
    Kr = "Kr"
    # Period 5
    Rb = "Rb"
    Sr = "Sr"
    Y = "Y"
    Zr = "Zr"
    Nb = "Nb"
    Mo = "Mo"
    Tc = "Tc"
    Ru = "Ru"
    Rh = "Rh"
    Pd = "Pd"
    Ag = "Ag"
    Cd = "Cd"
    In = "In"
    Sn = "Sn"
    Sb = "Sb"
    Te = "Te"
    I = "I"
    Xe = "Xe"
    # Period 6
    Cs = "Cs"
    Ba = "Ba"
    La = "La"
    Ce = "Ce"
    Pr = "Pr"
    Nd = "Nd"
    Pm = "Pm"
    Sm = "Sm"
    Eu = "Eu"
    Gd = "Gd"
    Tb = "Tb"
    Dy = "Dy"
    Ho = "Ho"
    Er = "Er"
    Tm = "Tm"
    Yb = "Yb"
    Lu = "Lu"
    Hf = "Hf"
    Ta = "Ta"
    W = "W"
    Re = "Re"
    Os = "Os"
    Ir = "Ir"
    Pt = "Pt"
    Au = "Au"
    Hg = "Hg"
    Tl = "Tl"
    Pb = "Pb"
    Bi = "Bi"
    # Selected actinides (extend as needed)
    Th = "Th"
    U = "U"


class _Frozen(BaseModel):
    """Frozen, strict base for every ADT node."""

    model_config = ConfigDict(extra="forbid", frozen=True)


class Coordinate(_Frozen):
    """Cartesian coordinates in Angstroms. Mirrors Haskell ``Coordinate``."""

    x: float = 0.0
    y: float = 0.0
    z: float = 0.0


class Atom(_Frozen):
    """A single atom in a molecule.

    The Haskell ``Atom`` record also carries ``attributes`` (atomic number,
    atomic weight) and ``shells`` (electronic configuration), but both are
    fully determined by ``symbol`` and are looked up from ``Constants`` on
    decode. We therefore omit them from the wire format to keep artifacts
    compact and to remove a class of decoding inconsistencies.
    """

    atom_id: int = Field(ge=0)
    symbol: AtomicSymbol
    coordinate: Coordinate = Coordinate()
    formal_charge: int = 0


# --------------------------------------------------------------------------- bonding


# An undirected edge between two atoms, canonicalised to (i, j) with i <= j.
# Encoded on the wire as a two-element list to match the Haskell ``Edge``
# constructor's positional fields after the ``mkEdge`` canonicalisation.
Edge = Tuple[int, int]


class BondingSystem(_Frozen):
    """One Dietz bonding system: ``shared_electrons`` over ``member_edges``.

    Mirrors Haskell ``BondingSystem`` from ``Chem.Dietz``. ``member_atoms`` is
    intentionally not stored on the wire because it is a deterministic
    function of ``member_edges`` (Haskell ``mkBondingSystem`` derives it).
    """

    system_id: int = Field(ge=0)
    shared_electrons: int = Field(ge=0)
    member_edges: List[Edge]
    tag: Optional[str] = None


class Molecule(_Frozen):
    """A molecule: atoms, σ-bond skeleton, Dietz bonding systems."""

    atoms: List[Atom]
    local_bonds: List[Edge] = []
    systems: List[BondingSystem] = []


# --------------------------------------------------------------------------- reactions


class TemperatureCondition(_Frozen):
    """Mirrors Haskell ``TempCondition``. Stored in Kelvin to remove
    ambiguity (the Haskell record's ``temperature`` field is unitless)."""

    kind: Literal["temperature"] = "temperature"
    kelvin: float = Field(gt=0.0)


class PressureCondition(_Frozen):
    """Mirrors Haskell ``PressureCondition``. Stored in bar."""

    kind: Literal["pressure"] = "pressure"
    bar: float = Field(gt=0.0)


class CatalystCondition(_Frozen):
    """A catalyst expressed as a structural ``Molecule``.

    The catalyst is not consumed stoichiometrically, so it lives in
    ``conditions`` rather than ``reactants``/``products``.
    """

    kind: Literal["catalyst"] = "catalyst"
    molecule: Molecule


class SolventCondition(_Frozen):
    """A solvent expressed as a structural ``Molecule``.

    Like catalysts, solvents are not consumed by the reaction; they qualify
    the conditions under which it runs.
    """

    kind: Literal["solvent"] = "solvent"
    molecule: Molecule


# Pydantic will discriminate on the literal ``kind`` field when parsing.
Condition = Union[
    TemperatureCondition,
    PressureCondition,
    CatalystCondition,
    SolventCondition,
]


class StoichiometryEntry(_Frozen):
    """One reactant or product term: a positive coefficient + a Molecule.

    Mirrors the Haskell ``(Double, Molecule)`` tuple in
    ``Reaction.reactants`` / ``Reaction.products``.
    """

    coefficient: float = Field(gt=0.0)
    molecule: Molecule


class Reaction(_Frozen):
    """A balanced chemical transformation.

    Strictly mirrors the Haskell ``Reaction`` record. Information that the
    Haskell ADT cannot represent (catalyst identity, solvent identity,
    reaction mechanism, stereochemistry of named centres, etc.) is
    deliberately omitted rather than smuggled in as a string.
    """

    reactants: List[StoichiometryEntry]
    products: List[StoichiometryEntry]
    conditions: List[Condition] = []
    rate: float = 0.0


# --------------------------------------------------------------------------- canonicalisation


def canonical_edge(i: int, j: int) -> Edge:
    """Return the canonical (i, j) ordering used by Haskell ``mkEdge``."""
    return (i, j) if i <= j else (j, i)


def canonicalise_molecule(mol: Molecule) -> Molecule:
    """Return ``mol`` with edges canonicalised and atoms sorted by id.

    This is the only legal way to compare two molecules for byte-equality on
    disk. It does NOT perform graph isomorphism — two structurally identical
    molecules with different ``atom_id`` numbering remain distinct artifacts.
    """
    atoms = sorted(mol.atoms, key=lambda a: a.atom_id)
    local_bonds = sorted({canonical_edge(i, j) for (i, j) in mol.local_bonds})
    systems = sorted(
        (
            BondingSystem(
                system_id=s.system_id,
                shared_electrons=s.shared_electrons,
                member_edges=sorted(
                    {canonical_edge(i, j) for (i, j) in s.member_edges}
                ),
                tag=s.tag,
            )
            for s in mol.systems
        ),
        key=lambda s: s.system_id,
    )
    return Molecule(atoms=atoms, local_bonds=local_bonds, systems=systems)


__all__ = [
    "AtomicSymbol",
    "Coordinate",
    "Atom",
    "Edge",
    "BondingSystem",
    "Molecule",
    "TemperatureCondition",
    "PressureCondition",
    "CatalystCondition",
    "SolventCondition",
    "Condition",
    "StoichiometryEntry",
    "Reaction",
    "canonical_edge",
    "canonicalise_molecule",
]
