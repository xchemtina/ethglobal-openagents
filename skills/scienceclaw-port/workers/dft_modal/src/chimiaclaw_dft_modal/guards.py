"""Hard caps for live / Modal DFT before any GPU is reserved.

These limits protect the DAO treasury and Modal spend. Override via env for
operator-controlled batch jobs; defaults target the public ``dft.live_small``
SKU (small organics, not free-run libraries).
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Any


class DftGuardError(Exception):
    """Raised when a job is rejected before compute starts."""


@dataclass(frozen=True)
class DftGuards:
    max_atoms: int
    max_electrons: int
    max_wall_seconds: int
    max_estimated_usd: float
    gpu_hourly_usd: float
    require_operator_flag: bool
    allow_open_shell: bool
    # Rough element Z table for electron counting (H..Xe + common heavier).
    # Unknown symbols count as 0 and force rejection unless listed here.
    z_table: dict[str, int]


# Minimal Z table for common organic / main-group demo molecules.
_DEFAULT_Z: dict[str, int] = {
    "H": 1,
    "He": 2,
    "Li": 3,
    "Be": 4,
    "B": 5,
    "C": 6,
    "N": 7,
    "O": 8,
    "F": 9,
    "Ne": 10,
    "Na": 11,
    "Mg": 12,
    "Al": 13,
    "Si": 14,
    "P": 15,
    "S": 16,
    "Cl": 17,
    "Ar": 18,
    "K": 19,
    "Ca": 20,
    "Sc": 21,
    "Ti": 22,
    "V": 23,
    "Cr": 24,
    "Mn": 25,
    "Fe": 26,
    "Co": 27,
    "Ni": 28,
    "Cu": 29,
    "Zn": 30,
    "Ga": 31,
    "Ge": 32,
    "As": 33,
    "Se": 34,
    "Br": 35,
    "Kr": 36,
    "Rb": 37,
    "Sr": 38,
    "Y": 39,
    "Zr": 40,
    "Nb": 41,
    "Mo": 42,
    "Tc": 43,
    "Ru": 44,
    "Rh": 45,
    "Pd": 46,
    "Ag": 47,
    "Cd": 48,
    "In": 49,
    "Sn": 50,
    "Sb": 51,
    "Te": 52,
    "I": 53,
    "Xe": 54,
    "Cs": 55,
    "Ba": 56,
    "La": 57,
    "Hf": 72,
    "Ta": 73,
    "W": 74,
    "Re": 75,
    "Os": 76,
    "Ir": 77,
    "Pt": 78,
    "Au": 79,
    "Hg": 80,
    "Tl": 81,
    "Pb": 82,
    "Bi": 83,
    "Th": 90,
    "U": 92,
}


def load_guards_from_env() -> DftGuards:
    """Public SKU defaults: ~40 atoms, 10 min, ~$2.50 ceiling."""
    return DftGuards(
        max_atoms=int(os.environ.get("CHIMIACLAW_DFT_MAX_ATOMS", "40")),
        max_electrons=int(os.environ.get("CHIMIACLAW_DFT_MAX_ELECTRONS", "200")),
        max_wall_seconds=int(os.environ.get("CHIMIACLAW_DFT_MAX_WALL_SECONDS", "600")),
        max_estimated_usd=float(os.environ.get("CHIMIACLAW_DFT_MAX_ESTIMATED_USD", "2.50")),
        # Modal H100 list price is operator-dependent; default is conservative.
        gpu_hourly_usd=float(os.environ.get("CHIMIACLAW_DFT_GPU_HOURLY_USD", "4.00")),
        require_operator_flag=os.environ.get(
            "CHIMIACLAW_DFT_REQUIRE_OPERATOR", "1"
        ).strip()
        not in ("0", "false", "False"),
        allow_open_shell=os.environ.get("CHIMIACLAW_DFT_ALLOW_OPEN_SHELL", "0").strip()
        in ("1", "true", "True"),
        z_table=dict(_DEFAULT_Z),
    )


def count_atoms(molecule_adt: dict[str, Any] | None) -> int:
    if not molecule_adt:
        return 0
    atoms = molecule_adt.get("atoms")
    if isinstance(atoms, dict):
        return len(atoms)
    if isinstance(atoms, list):
        return len(atoms)
    return 0


def iter_symbols(molecule_adt: dict[str, Any] | None) -> list[str]:
    if not molecule_adt:
        return []
    atoms = molecule_adt.get("atoms")
    symbols: list[str] = []
    if isinstance(atoms, dict):
        ordered = sorted(atoms.items(), key=lambda kv: int(kv[0]))
        for _k, atom in ordered:
            if not isinstance(atom, dict):
                continue
            attrs = atom.get("attributes") or {}
            sym = attrs.get("symbol") if isinstance(attrs, dict) else None
            if isinstance(sym, str):
                symbols.append(sym)
    elif isinstance(atoms, list):
        for atom in atoms:
            if not isinstance(atom, dict):
                continue
            attrs = atom.get("attributes") or atom
            sym = attrs.get("symbol") if isinstance(attrs, dict) else None
            if isinstance(sym, str):
                symbols.append(sym)
    return symbols


def count_electrons(
    molecule_adt: dict[str, Any] | None,
    total_charge: int,
    z_table: dict[str, int],
) -> tuple[int, list[str]]:
    """Return (n_electrons, unknown_symbols)."""
    unknown: list[str] = []
    z_sum = 0
    for sym in iter_symbols(molecule_adt):
        z = z_table.get(sym) or z_table.get(sym.capitalize())
        if z is None:
            unknown.append(sym)
            continue
        z_sum += z
    return z_sum - total_charge, unknown


def estimate_job_usd(wall_seconds: int, gpu_hourly_usd: float) -> float:
    return (wall_seconds / 3600.0) * gpu_hourly_usd


def enforce_guards(
    request: dict[str, Any],
    molecule_adt: dict[str, Any] | None,
    guards: DftGuards | None = None,
    *,
    mode: str = "modal",
) -> dict[str, Any]:
    """Validate job before Modal reservation. Returns a summary dict on success.

    Raises DftGuardError with a machine-readable message on rejection.
    """
    g = guards or load_guards_from_env()
    if mode not in ("stub", "local") and g.require_operator_flag:
        flag = os.environ.get("CHIMIACLAW_DFT_LIVE_OPERATOR", "").strip()
        if flag != "1":
            raise DftGuardError(
                "operator gate: set CHIMIACLAW_DFT_LIVE_OPERATOR=1 to reserve "
                "Modal/GPU compute (public free-run is disabled)"
            )

    n_atoms = count_atoms(molecule_adt)
    if n_atoms <= 0:
        raise DftGuardError(
            "molecule_adt missing or has zero atoms; live DFT needs coordinates"
        )
    if n_atoms > g.max_atoms:
        raise DftGuardError(
            f"atom cap exceeded: {n_atoms} > max_atoms={g.max_atoms} "
            f"(override CHIMIACLAW_DFT_MAX_ATOMS for operator batches only)"
        )

    total_charge = int(request.get("total_charge", 0))
    multiplicity = int(request.get("multiplicity", 1))
    if multiplicity != 1 and not g.allow_open_shell:
        raise DftGuardError(
            f"open-shell (multiplicity={multiplicity}) disabled for live_small; "
            "set CHIMIACLAW_DFT_ALLOW_OPEN_SHELL=1 for UKS operator jobs"
        )

    n_elec, unknown = count_electrons(molecule_adt, total_charge, g.z_table)
    if unknown:
        raise DftGuardError(
            f"unknown/unsupported elements for electron budget: {sorted(set(unknown))}"
        )
    if n_elec <= 0:
        raise DftGuardError(f"invalid electron count: {n_elec}")
    if n_elec > g.max_electrons:
        raise DftGuardError(
            f"electron cap exceeded: {n_elec} > max_electrons={g.max_electrons}"
        )

    est = estimate_job_usd(g.max_wall_seconds, g.gpu_hourly_usd)
    if est > g.max_estimated_usd + 1e-9:
        raise DftGuardError(
            f"spend cap: estimated ${est:.2f} for wall={g.max_wall_seconds}s "
            f"@ ${g.gpu_hourly_usd:.2f}/h exceeds max_estimated_usd="
            f"${g.max_estimated_usd:.2f}"
        )

    return {
        "n_atoms": n_atoms,
        "n_electrons": n_elec,
        "multiplicity": multiplicity,
        "max_wall_seconds": g.max_wall_seconds,
        "estimated_usd_ceiling": round(est, 4),
        "gpu_hourly_usd": g.gpu_hourly_usd,
        "mode": mode,
    }
