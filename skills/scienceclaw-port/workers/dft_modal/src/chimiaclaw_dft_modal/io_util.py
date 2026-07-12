"""Shared stdin/stdout helpers matching the Olympus DFT worker contract."""

from __future__ import annotations

import json
import platform
import sys
from typing import Any


SCHEMA_TAG = "chem.dft.result"


def read_worker_input(raw: str | None = None) -> tuple[dict[str, Any], dict[str, Any] | None, dict[str, Any] | None]:
    """Parse stdin (or raw string) into (request, molecule_adt, cube_grid)."""
    text = sys.stdin.read() if raw is None else raw
    if not text.strip():
        raise SystemExit("dft-modal: empty stdin; expected JSON")
    try:
        document = json.loads(text)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"dft-modal: invalid JSON on stdin: {exc}") from exc
    if not isinstance(document, dict):
        raise SystemExit("dft-modal: stdin must be a JSON object")
    if "request" in document and "molecule_adt" in document:
        request = document["request"]
        molecule_adt = document["molecule_adt"]
        cube_grid = document.get("cube_grid")
        if not isinstance(request, dict):
            raise SystemExit("dft-modal: wrapper.request must be an object")
        if not isinstance(molecule_adt, dict):
            raise SystemExit("dft-modal: wrapper.molecule_adt must be an object")
        if cube_grid is not None and not isinstance(cube_grid, dict):
            raise SystemExit("dft-modal: wrapper.cube_grid must be an object or absent")
        return request, molecule_adt, cube_grid
    return document, None, None


def wrap_payload(
    request: dict[str, Any],
    molecule_adt: dict[str, Any] | None,
    cube_grid: dict[str, Any] | None,
) -> dict[str, Any]:
    if molecule_adt is None:
        return {"request": request}
    out: dict[str, Any] = {"request": request, "molecule_adt": molecule_adt}
    if cube_grid is not None:
        out["cube_grid"] = cube_grid
    return out


def stub_result(request: dict[str, Any], *, note: str) -> dict[str, Any]:
    method = request.get("method", {}) if isinstance(request.get("method"), dict) else {}
    molecule = request.get("molecule", {}) if isinstance(request.get("molecule"), dict) else {}
    return {
        "schema_tag": SCHEMA_TAG,
        "request_id": str(request.get("request_id", "REQ.UNKNOWN")),
        "molecule_id": str(molecule.get("molecule_id", "unknown")),
        "functional": str(method.get("functional", "stub")),
        "basis_set": str(method.get("basis_set", "stub")),
        "backend": "StubBackend",
        "total_charge": int(request.get("total_charge", 0)),
        "multiplicity": int(request.get("multiplicity", 1)),
        "energy_hartree": 0.0,
        "orbitals": None,
        "dipole": None,
        "convergence": {
            "converged": False,
            "n_cycles": 0,
            "final_gradient_norm": None,
            "scf_threshold": None,
        },
        "timings": {"wall_seconds": 0.0, "cpu_seconds": None},
        "requested_properties": list(request.get("requested_properties", [])),
        "provenance": {
            "source_kind": "stub-result",
            "source_ref": "chimiaclaw-dft-modal --stub",
            "host": platform.node(),
            "pyscf_version": None,
            "skala_version": None,
            "dispersion": method.get("dispersion"),
            "notes": [
                "STUB MODE: no SCF was performed; Rust signer refuses "
                "convergence.converged=False.",
                note,
            ],
        },
        "orbital_cubes": [],
    }


def write_result(result: dict[str, Any]) -> None:
    json.dump(result, sys.stdout, indent=2)
    sys.stdout.write("\n")
