"""In-process PySCF SCF used by Modal containers and optional local mode.

Mirrors the classical path in ``chimiaclaw_dft.pyscf_backend`` so Modal does not
need to import the sibling package path at deploy time.
"""

from __future__ import annotations

import platform
import time
from typing import Any

from .io_util import SCHEMA_TAG

HARTREE_TO_EV = 27.211386245988

_FUNCTIONAL_ALIASES = {
    "skala-1.1": "pbe",
    "skala": "pbe",
}


def _atom_block(molecule_adt: dict[str, Any]) -> str:
    atoms = molecule_adt.get("atoms")
    if not atoms or not isinstance(atoms, dict):
        raise ValueError("molecule_adt.atoms must be a non-empty object")
    ordered = sorted(atoms.items(), key=lambda kv: int(kv[0]))
    rows: list[str] = []
    for _key, atom in ordered:
        symbol = atom["attributes"]["symbol"]
        xyz = atom["coordinate"]
        rows.append(
            f"{symbol} {xyz['x_angstrom']:.10f} "
            f"{xyz['y_angstrom']:.10f} {xyz['z_angstrom']:.10f}"
        )
    return "; ".join(rows)


def _resolve_functional(requested: str) -> tuple[str, str | None]:
    canonical = requested.strip().lower()
    if canonical in _FUNCTIONAL_ALIASES:
        return (
            _FUNCTIONAL_ALIASES[canonical],
            f"requested {requested!r}; classical Modal image falls back to PBE "
            "until Skala weights are mounted.",
        )
    return canonical, None


def run_scf(
    request: dict[str, Any],
    molecule_adt: dict[str, Any],
    cube_grid: dict[str, Any] | None = None,
    *,
    host_label: str | None = None,
) -> dict[str, Any]:
    """Run classical PySCF RKS/UKS and return a WorkerDftResult-shaped dict.

    Cube generation is intentionally **off by default on Modal** (payload size).
    Pass cube_grid only when you accept base64 cubes on the wire.
    """
    from pyscf import dft, gto  # type: ignore

    method = request.get("method", {}) if isinstance(request.get("method"), dict) else {}
    requested_xc = str(method.get("functional", "pbe"))
    pyscf_xc, fallback_note = _resolve_functional(requested_xc)
    basis = str(method.get("basis_set", "def2-tzvp"))
    total_charge = int(request.get("total_charge", 0))
    multiplicity = int(request.get("multiplicity", 1))
    spin = max(0, multiplicity - 1)

    mol = gto.M(
        atom=_atom_block(molecule_adt),
        basis=basis,
        unit="Angstrom",
        charge=total_charge,
        spin=spin,
        verbose=0,
    )

    started_wall = time.time()
    started_cpu = time.process_time()
    if multiplicity == 1:
        mf = dft.RKS(mol, xc=pyscf_xc)
    else:
        mf = dft.UKS(mol, xc=pyscf_xc)
    energy = float(mf.kernel())
    wall = time.time() - started_wall
    cpu = time.process_time() - started_cpu

    orbitals = None
    try:
        if multiplicity == 1:
            occ = mf.mo_occ
            energies = mf.mo_energy
            occupied_idx = [i for i, o in enumerate(occ) if o > 0]
            unoccupied_idx = [i for i, o in enumerate(occ) if o == 0]
            if occupied_idx and unoccupied_idx:
                homo = float(energies[occupied_idx[-1]])
                lumo = float(energies[unoccupied_idx[0]])
                gap = lumo - homo
                orbitals = {
                    "homo_hartree": homo,
                    "lumo_hartree": lumo,
                    "gap_hartree": gap,
                    "gap_ev": gap * HARTREE_TO_EV,
                }
    except Exception:  # noqa: BLE001
        orbitals = None

    dipole = None
    try:
        dipole_vec = mf.dip_moment(unit="DEBYE", verbose=0)
        dx, dy, dz = (float(v) for v in dipole_vec)
        magnitude = (dx * dx + dy * dy + dz * dz) ** 0.5
        dipole = {
            "x_debye": dx,
            "y_debye": dy,
            "z_debye": dz,
            "magnitude_debye": magnitude,
        }
    except Exception:  # noqa: BLE001
        dipole = None

    notes: list[str] = ["executed_on=modal_or_local_scf"]
    if fallback_note:
        notes.append(fallback_note)
    if cube_grid is not None:
        notes.append(
            "cube_grid requested but Modal path skips cubegen by default "
            "(use Olympus worker or enable a volume-backed cube path later)"
        )

    try:
        import pyscf  # type: ignore

        pyscf_version = pyscf.__version__
    except Exception:  # noqa: BLE001
        pyscf_version = None

    molecule = request.get("molecule", {}) if isinstance(request.get("molecule"), dict) else {}
    molecule_id = str(molecule.get("molecule_id", "unknown"))

    return {
        "schema_tag": SCHEMA_TAG,
        "request_id": str(request.get("request_id", "REQ.UNKNOWN")),
        "molecule_id": molecule_id,
        "functional": requested_xc,
        "basis_set": basis,
        "backend": "PyScf",
        "total_charge": total_charge,
        "multiplicity": multiplicity,
        "energy_hartree": energy,
        "orbitals": orbitals,
        "dipole": dipole,
        "convergence": {
            "converged": bool(getattr(mf, "converged", True)),
            "n_cycles": int(getattr(mf, "scf_cycle", getattr(mf, "iter", 0) or 0)),
            "final_gradient_norm": None,
            "scf_threshold": float(getattr(mf, "conv_tol", 1e-8) or 1e-8),
        },
        "timings": {"wall_seconds": wall, "cpu_seconds": cpu},
        "requested_properties": list(request.get("requested_properties", [])),
        "provenance": {
            "source_kind": "pyscf-classical-functional",
            "source_ref": "chimiaclaw-dft-modal",
            "host": host_label or platform.node(),
            "pyscf_version": pyscf_version,
            "skala_version": None,
            "dispersion": method.get("dispersion"),
            "notes": notes,
        },
        "orbital_cubes": [],
    }
