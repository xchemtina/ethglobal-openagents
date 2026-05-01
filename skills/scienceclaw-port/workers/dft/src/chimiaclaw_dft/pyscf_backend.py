"""PySCF classical-functional backend.

Runs a real SCF using a stock PySCF functional (PBE, B3LYP, ...).  This is
the path we ship by default — Skala 1.1 is a separate optional backend that
the duck-side agent wires up later.

Input:
    {
      "request": <DftRequest>,
      "molecule_adt": <MoleculeAdt>,    # atoms with x/y/z coordinates
    }

Output:
    a fully populated DftResult dict matching chimiaclaw_dft_skala::DftResult.
"""

from __future__ import annotations

import base64
import hashlib
import platform
import tempfile
import time
from pathlib import Path
from typing import Any

from .cli import (
    SCHEMA_TAG,
    Convergence,
    DftResult,
    Dipole,
    OrbitalCube,
    Orbitals,
    Provenance,
    Timings,
)


# Hartree -> eV
HARTREE_TO_EV = 27.211386245988
# atomic unit of dipole -> Debye
AU_TO_DEBYE = 2.541746229


_FUNCTIONAL_ALIASES = {
    "skala-1.1": "pbe",  # fallback so a Skala request still produces SOMETHING
                          # tagged as classical-fallback (see provenance below).
    "skala": "pbe",
}


def _atom_block(molecule_adt: dict[str, Any]) -> str:
    """Build a PySCF gto.M `atom=` block from a MoleculeAdt JSON payload.

    The Rust `MoleculeAdt::atoms` is a BTreeMap<u32, Atom> that serializes to
    a JSON object keyed by stringified atom_id.  Each Atom carries
    `attributes.symbol` and `coordinate.{x,y,z}_angstrom`.
    """
    atoms = molecule_adt.get("atoms")
    if not atoms:
        raise ValueError("molecule_adt has no atoms")
    if not isinstance(atoms, dict):
        raise ValueError(f"unexpected atoms shape: {type(atoms).__name__}")
    ordered = sorted(atoms.items(), key=lambda kv: int(kv[0]))
    rows = []
    for _key, atom in ordered:
        symbol = atom["attributes"]["symbol"]
        xyz = atom["coordinate"]
        rows.append(
            f"{symbol} {xyz['x_angstrom']:.10f} {xyz['y_angstrom']:.10f} {xyz['z_angstrom']:.10f}"
        )
    return "; ".join(rows)


def _resolve_functional(requested: str) -> tuple[str, str | None]:
    """Return the (pyscf_xc_string, fallback_notice) for a requested functional.

    If the operator asked for `skala-1.1` but the classical backend is the only
    one available, we fall back to PBE and tag the result with a notice so the
    artifact reviewer can see what happened.
    """
    canonical = requested.strip().lower()
    if canonical in _FUNCTIONAL_ALIASES:
        return (
            _FUNCTIONAL_ALIASES[canonical],
            f"requested {requested!r} but classical backend only ships PBE/B3LYP/etc.; "
            "fell back to PBE.  Re-run with backend=pyscf-skala once weights are available.",
        )
    return canonical, None


def _generate_cubes(
    mol: Any,
    mf: Any,
    grid: dict[str, Any],
    molecule_id: str,
) -> list[OrbitalCube]:
    """Generate HOMO / LUMO / total-density cube files via pyscf.tools.cubegen.

    Each cube is hashed with SHA-256 and returned along with its base64-
    encoded bytes so the Rust adapter can materialize the file locally and
    sign the hash into the chem.dft.result artifact.
    """
    from pyscf.tools import cubegen

    resolution = int(grid.get("resolution", 60))
    margin = float(grid.get("margin_angstrom", 3.0))
    include_homo = bool(grid.get("include_homo", True))
    include_lumo = bool(grid.get("include_lumo", True))
    include_total = bool(grid.get("include_total_density", True))

    cubes: list[OrbitalCube] = []
    occ = mf.mo_occ
    energies = mf.mo_energy
    coeff = mf.mo_coeff
    occupied = [i for i, o in enumerate(occ) if o > 0]
    unoccupied = [i for i, o in enumerate(occ) if o == 0]
    homo_idx = occupied[-1] if occupied else None
    lumo_idx = unoccupied[0] if unoccupied else None

    with tempfile.TemporaryDirectory(prefix="chimiaclaw-dft-cubes-") as tmp_dir:
        tmp_path = Path(tmp_dir)
        tasks: list[tuple[str, callable]] = []

        def _orbital(label: str, idx: int):
            path = tmp_path / f"{molecule_id}_{label}.cube"

            def _gen():
                cubegen.orbital(
                    mol,
                    str(path),
                    coeff[:, idx],
                    nx=resolution,
                    ny=resolution,
                    nz=resolution,
                    margin=margin,
                )
                return path

            return label, _gen

        if include_homo and homo_idx is not None:
            tasks.append(_orbital("HOMO", homo_idx))
        if include_lumo and lumo_idx is not None:
            tasks.append(_orbital("LUMO", lumo_idx))
        if include_total:
            density_path = tmp_path / f"{molecule_id}_TOTAL_DENSITY.cube"

            def _gen_density():
                cubegen.density(
                    mol,
                    str(density_path),
                    mf.make_rdm1(),
                    nx=resolution,
                    ny=resolution,
                    nz=resolution,
                    margin=margin,
                )
                return density_path

            tasks.append(("TOTAL_DENSITY", _gen_density))

        for label, gen in tasks:
            try:
                path = gen()
            except Exception as exc:  # pylint: disable=broad-except
                # Cubegen can fail on edge cases (basis quirks, etc.).  Skip
                # this cube rather than aborting the whole result.
                print(
                    f"dft worker: cubegen failed for {label}: {exc}",
                    file=__import__("sys").stderr,
                )
                continue
            data = path.read_bytes()
            sha256 = hashlib.sha256(data).hexdigest()
            cubes.append(
                OrbitalCube(
                    label=label,
                    sha256=sha256,
                    bytes=len(data),
                    grid_resolution=resolution,
                    worker_path=str(path),
                    bytes_base64=base64.standard_b64encode(data).decode("ascii"),
                )
            )
        # Suppress lint about unused variable.
        _ = energies
    return cubes


def run(
    request: dict[str, Any],
    molecule_adt: dict[str, Any],
    cube_grid: dict[str, Any] | None = None,
) -> DftResult:
    from pyscf import dft, gto  # imported lazily so --stub doesn't need it

    method = request.get("method", {})
    requested_xc = str(method.get("functional", "pbe"))
    pyscf_xc, fallback_note = _resolve_functional(requested_xc)
    basis = str(method.get("basis_set", "def2-tzvp"))
    total_charge = int(request.get("total_charge", 0))
    multiplicity = int(request.get("multiplicity", 1))
    spin = max(0, multiplicity - 1)  # PySCF spin = 2S = multiplicity - 1

    atom_block = _atom_block(molecule_adt)
    mol = gto.M(
        atom=atom_block,
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

    # Frontier orbitals (closed-shell case).
    orbitals: Orbitals | None = None
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
                orbitals = Orbitals(
                    homo_hartree=homo,
                    lumo_hartree=lumo,
                    gap_hartree=gap,
                    gap_ev=gap * HARTREE_TO_EV,
                )
    except Exception:  # pylint: disable=broad-except
        orbitals = None

    # Dipole moment (mf.dip_moment returns a numpy array in the requested unit).
    dipole: Dipole | None = None
    try:
        dipole_vec = mf.dip_moment(unit="DEBYE", verbose=0)
        dx, dy, dz = (float(v) for v in dipole_vec)
        magnitude = (dx * dx + dy * dy + dz * dz) ** 0.5
        dipole = Dipole(
            x_debye=dx,
            y_debye=dy,
            z_debye=dz,
            magnitude_debye=magnitude,
        )
    except Exception:  # pylint: disable=broad-except
        dipole = None

    converged = bool(getattr(mf, "converged", True))
    n_cycles = int(getattr(mf, "scf_cycle", getattr(mf, "iter", 0) or 0))
    scf_threshold = float(getattr(mf, "conv_tol", 1e-8) or 1e-8)

    notes: list[str] = []
    if fallback_note is not None:
        notes.append(fallback_note)

    source_kind = (
        "pyscf-skala-1.1"
        if requested_xc.lower().startswith("skala") and fallback_note is None
        else "pyscf-classical-functional"
    )

    try:
        import pyscf  # noqa: F401  pylint: disable=import-outside-toplevel
        pyscf_version = pyscf.__version__
    except Exception:  # pylint: disable=broad-except
        pyscf_version = None

    molecule = request.get("molecule", {})
    molecule_id = str(molecule.get("molecule_id", "unknown"))
    cubes: list[OrbitalCube] = []
    if cube_grid is not None and converged:
        try:
            cubes = _generate_cubes(mol, mf, cube_grid, molecule_id)
        except Exception as exc:  # pylint: disable=broad-except
            notes.append(f"orbital cube generation failed: {exc}")

    return DftResult(
        schema_tag=SCHEMA_TAG,
        request_id=str(request.get("request_id", "REQ.UNKNOWN")),
        molecule_id=molecule_id,
        functional=requested_xc,
        basis_set=basis,
        backend="PyScf",
        total_charge=total_charge,
        multiplicity=multiplicity,
        energy_hartree=energy,
        orbitals=orbitals,
        dipole=dipole,
        convergence=Convergence(
            converged=converged,
            n_cycles=n_cycles,
            final_gradient_norm=None,
            scf_threshold=scf_threshold,
        ),
        timings=Timings(wall_seconds=wall, cpu_seconds=cpu),
        requested_properties=list(request.get("requested_properties", [])),
        provenance=Provenance(
            source_kind=source_kind,
            source_ref=f"pyscf={pyscf_version} xc={pyscf_xc} basis={basis}",
            host=platform.node(),
            pyscf_version=pyscf_version,
            skala_version=None,
            dispersion=method.get("dispersion"),
            notes=notes,
        ),
        orbital_cubes=cubes,
    )
