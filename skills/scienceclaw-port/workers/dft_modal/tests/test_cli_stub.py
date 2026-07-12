"""CLI stub mode: no Modal account, no PySCF."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"


def _water_payload() -> dict:
    return {
        "request": {
            "request_id": "REQ.TEST.WATER",
            "total_charge": 0,
            "multiplicity": 1,
            "molecule": {
                "molecule_id": "water",
                "molecule_name": "water",
                "canonical_smiles": "O",
            },
            "method": {"functional": "pbe", "basis_set": "def2-tzvp"},
            "requested_properties": ["total_energy"],
        },
        "molecule_adt": {
            "atoms": {
                "0": {
                    "attributes": {"symbol": "O"},
                    "coordinate": {
                        "x_angstrom": 0.0,
                        "y_angstrom": 0.0,
                        "z_angstrom": 0.1173,
                    },
                },
                "1": {
                    "attributes": {"symbol": "H"},
                    "coordinate": {
                        "x_angstrom": 0.0,
                        "y_angstrom": 0.7572,
                        "z_angstrom": -0.4692,
                    },
                },
                "2": {
                    "attributes": {"symbol": "H"},
                    "coordinate": {
                        "x_angstrom": 0.0,
                        "y_angstrom": -0.7572,
                        "z_angstrom": -0.4692,
                    },
                },
            }
        },
    }


def test_stub_cli_stdout():
    env = {
        **dict(**{k: v for k, v in __import__("os").environ.items()}),
        "PYTHONPATH": str(SRC),
    }
    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "chimiaclaw_dft_modal.cli",
            "--mode",
            "stub",
        ],
        input=json.dumps(_water_payload()),
        text=True,
        capture_output=True,
        cwd=str(ROOT),
        env=env,
        check=False,
    )
    assert proc.returncode == 0, proc.stderr
    body = json.loads(proc.stdout)
    assert body["schema_tag"] == "chem.dft.result"
    assert body["convergence"]["converged"] is False
    assert body["provenance"]["source_kind"] == "stub-result"
    assert "guard dry-run" in " ".join(body["provenance"]["notes"])


def test_modal_mode_rejected_without_operator(monkeypatch=None):
    import os

    env = {
        **os.environ,
        "PYTHONPATH": str(SRC),
    }
    env.pop("CHIMIACLAW_DFT_LIVE_OPERATOR", None)
    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "chimiaclaw_dft_modal.cli",
            "--mode",
            "modal",
        ],
        input=json.dumps(_water_payload()),
        text=True,
        capture_output=True,
        cwd=str(ROOT),
        env=env,
        check=False,
    )
    assert proc.returncode == 3
    assert "operator gate" in proc.stderr
