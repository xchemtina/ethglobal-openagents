"""Entry point that turns a SMILES string into a MolADT JSON document.

The output schema mirrors the public fields of `chimiaclaw_moladt::MoleculeAdt`
in the Rust crate. Atoms are enumerated 1-based to match the BTreeMap shape
the Rust crate expects, and an aromatic ring system is emitted as a single
``BondingSystem`` per ring with ``shared_electrons = 6``.
"""
from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
import sys
from typing import Iterable, List, Sequence

# RDKit is loaded lazily so that ``--help`` still works without RDKit
# installed; the worker is, however, useless without it.
SUPPORTED_ELEMENTS = {
    1: "H",
    5: "B",
    6: "C",
    7: "N",
    8: "O",
    9: "F",
    11: "Na",
    15: "P",
    16: "S",
    17: "Cl",
    26: "Fe",
    35: "Br",
    53: "I",
}


def _import_rdkit():
    try:
        from rdkit import Chem
        from rdkit.Chem import AllChem
    except ModuleNotFoundError as error:
        sys.stderr.write(
            "rdkit is not importable. Install via uv: "
            "`uv pip install rdkit` (or run this worker through `uvx --from <path> rdkit-smiles-to-moladt`).\n"
        )
        raise SystemExit(2) from error
    return Chem, AllChem


@dataclasses.dataclass
class WorkerOptions:
    smiles: str
    embed_seed: int = 1729
    add_hydrogens: bool = True
    optimize_iterations: int = 200
    name_override: str | None = None
    molecule_id_override: str | None = None


def parse_args(argv: Sequence[str]) -> WorkerOptions:
    parser = argparse.ArgumentParser(
        prog="rdkit-smiles-to-moladt",
        description="Translate a SMILES string into a chimiaclaw-moladt JSON document.",
    )
    parser.add_argument(
        "--smiles",
        help="SMILES string. If omitted, the worker reads it from stdin.",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=1729,
        help="Random seed used by the ETKDGv3 conformer generator (default: 1729).",
    )
    parser.add_argument(
        "--no-hydrogens",
        action="store_true",
        help="Skip the AddHs pass; usually only useful for testing/debugging.",
    )
    parser.add_argument(
        "--optimize-iterations",
        type=int,
        default=200,
        help="Maximum optimization iterations passed to MMFF/UFF (default: 200).",
    )
    parser.add_argument(
        "--molecule-id",
        help="Override the molecule_id field; defaults to MOLADT.RDKIT.<inchikey-or-hash>.",
    )
    parser.add_argument(
        "--name",
        help="Override the molecule name; defaults to the SMILES.",
    )
    args = parser.parse_args(argv)
    smiles = args.smiles
    if smiles is None:
        smiles = sys.stdin.read().strip()
    if not smiles:
        parser.error("no SMILES provided on stdin or --smiles")
    return WorkerOptions(
        smiles=smiles,
        embed_seed=args.seed,
        add_hydrogens=not args.no_hydrogens,
        optimize_iterations=args.optimize_iterations,
        molecule_id_override=args.molecule_id,
        name_override=args.name,
    )


def _stable_hash(value: str) -> str:
    return hashlib.blake2b(value.encode("utf-8"), digest_size=8).hexdigest()


def _coordinates_for(conf, atom_index: int) -> dict:
    pos = conf.GetAtomPosition(atom_index)
    return {
        "x_angstrom": float(pos.x),
        "y_angstrom": float(pos.y),
        "z_angstrom": float(pos.z),
    }


def _aromatic_systems(rdmol) -> List[dict]:
    """Emit one BondingSystem per aromatic ring (set of bond indices)."""
    systems: List[dict] = []
    ring_info = rdmol.GetRingInfo()
    try:
        atom_rings = ring_info.AtomRings()
    except AttributeError:
        return systems
    for system_id, ring_atoms in enumerate(atom_rings, start=1):
        ring_atoms_list = list(ring_atoms)
        if not all(rdmol.GetAtomWithIdx(idx).GetIsAromatic() for idx in ring_atoms_list):
            continue
        member_edges: List[dict] = []
        for i, atom_idx in enumerate(ring_atoms_list):
            next_atom = ring_atoms_list[(i + 1) % len(ring_atoms_list)]
            a = atom_idx + 1
            b = next_atom + 1
            edge = {"a": min(a, b), "b": max(a, b)}
            member_edges.append(edge)
        systems.append(
            {
                "system_id": system_id,
                "shared_electrons": 6,
                "member_edges": member_edges,
                "tag": "aromatic_ring",
            }
        )
    return systems


def _ensure_supported(rdmol) -> None:
    for atom in rdmol.GetAtoms():
        atomic_number = atom.GetAtomicNum()
        if atomic_number not in SUPPORTED_ELEMENTS:
            raise SystemExit(
                f"unsupported element atomic_number={atomic_number} symbol={atom.GetSymbol()};\n"
                "extend chimiaclaw-moladt::AtomicSymbol before running this molecule through DFT.",
            )


def smiles_to_moladt(options: WorkerOptions) -> dict:
    Chem, AllChem = _import_rdkit()
    rdmol = Chem.MolFromSmiles(options.smiles)
    if rdmol is None:
        raise SystemExit(f"RDKit could not parse SMILES: {options.smiles!r}")
    rdmol = Chem.AddHs(rdmol) if options.add_hydrogens else rdmol
    _ensure_supported(rdmol)

    embed_params = AllChem.ETKDGv3()
    embed_params.randomSeed = options.embed_seed
    if AllChem.EmbedMolecule(rdmol, embed_params) != 0:
        raise SystemExit(
            f"RDKit ETKDGv3 embedding failed for SMILES {options.smiles!r}; "
            "try a different seed or supply explicit coordinates.",
        )
    optimizer = "rdkit-etkdgv3-mmff94"
    converged = AllChem.MMFFOptimizeMolecule(
        rdmol, maxIters=options.optimize_iterations
    )
    if converged < 0:
        # MMFF parameters not available; fall back to UFF.
        converged = AllChem.UFFOptimizeMolecule(
            rdmol, maxIters=options.optimize_iterations
        )
        optimizer = "rdkit-etkdgv3-uff"
    converged_flag = converged == 0

    canonical_smiles = Chem.MolToSmiles(rdmol)
    inchi = None
    inchikey = None
    try:
        inchi = Chem.MolToInchi(rdmol)
        if inchi:
            inchikey = Chem.InchiToInchiKey(inchi)
    except Exception:  # pragma: no cover - InChI failure is non-fatal
        inchi = None
        inchikey = None

    conf = rdmol.GetConformer()
    atoms: dict[str, dict] = {}
    for index, atom in enumerate(rdmol.GetAtoms()):
        atomic_number = atom.GetAtomicNum()
        symbol = SUPPORTED_ELEMENTS[atomic_number]
        atom_payload = {
            "atom_id": index + 1,
            "attributes": {
                "symbol": symbol,
                "atomic_number": atomic_number,
                "atomic_weight": atom.GetMass(),
            },
            "coordinate": _coordinates_for(conf, index),
            "formal_charge": atom.GetFormalCharge(),
            "shells": [],
        }
        atoms[str(index + 1)] = atom_payload

    local_bonds: List[dict] = []
    for bond in rdmol.GetBonds():
        a = bond.GetBeginAtomIdx() + 1
        b = bond.GetEndAtomIdx() + 1
        local_bonds.append({"a": min(a, b), "b": max(a, b)})
    local_bonds.sort(key=lambda edge: (edge["a"], edge["b"]))

    systems = _aromatic_systems(rdmol)

    fingerprint = inchikey or _stable_hash(canonical_smiles)
    molecule_id = options.molecule_id_override or f"MOLADT.RDKIT.{fingerprint}"
    name = options.name_override or options.smiles

    payload = {
        "molecule_id": molecule_id,
        "name": name,
        "atoms": atoms,
        "local_bonds": local_bonds,
        "systems": systems,
        "provenance": {
            "source_kind": optimizer,
            "source_ref": (
                "skills/scienceclaw-port/workers/cheminformatics::rdkit-smiles-to-moladt"
            ),
            "notes": [
                f"SMILES input: {options.smiles}",
                f"ETKDGv3 random seed: {options.embed_seed}",
                f"Hydrogens added: {options.add_hydrogens}",
                f"Optimizer: {optimizer}",
                f"Optimizer convergence flag (0 == converged): {converged}",
                "DFT worker should still re-optimize before energies are trusted.",
            ],
        },
        "projections": {
            "canonical_smiles": canonical_smiles,
            "inchi": inchi,
            "inchikey": inchikey,
        },
    }
    if not converged_flag:
        payload["provenance"]["notes"].append(
            "warning: RDKit reported the optimizer did not fully converge."
        )
    return payload


def main(argv: Iterable[str] | None = None) -> int:
    options = parse_args(list(argv) if argv is not None else sys.argv[1:])
    payload = smiles_to_moladt(options)
    json.dump(payload, sys.stdout, separators=(",", ":"))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":  # pragma: no cover - convenience entry
    raise SystemExit(main())
