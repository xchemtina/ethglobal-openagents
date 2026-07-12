"""Offline tests for Modal DFT spend/atom guards."""

from __future__ import annotations

import pytest

from chimiaclaw_dft_modal.guards import (
    DftGuardError,
    DftGuards,
    count_atoms,
    enforce_guards,
    estimate_job_usd,
)


def _water_adt() -> dict:
    return {
        "atoms": {
            "0": {
                "attributes": {"symbol": "O"},
                "coordinate": {
                    "x_angstrom": 0.0,
                    "y_angstrom": 0.0,
                    "z_angstrom": 0.0,
                },
            },
            "1": {
                "attributes": {"symbol": "H"},
                "coordinate": {
                    "x_angstrom": 0.96,
                    "y_angstrom": 0.0,
                    "z_angstrom": 0.0,
                },
            },
            "2": {
                "attributes": {"symbol": "H"},
                "coordinate": {
                    "x_angstrom": -0.24,
                    "y_angstrom": 0.93,
                    "z_angstrom": 0.0,
                },
            },
        }
    }


def _request(**kwargs) -> dict:
    base = {
        "request_id": "REQ.TEST.WATER",
        "total_charge": 0,
        "multiplicity": 1,
        "molecule": {"molecule_id": "water"},
        "method": {"functional": "pbe", "basis_set": "def2-tzvp"},
        "requested_properties": ["total_energy"],
    }
    base.update(kwargs)
    return base


def _loose_guards(**overrides) -> DftGuards:
    base = DftGuards(
        max_atoms=40,
        max_electrons=200,
        max_wall_seconds=600,
        max_estimated_usd=2.50,
        gpu_hourly_usd=4.00,
        require_operator_flag=False,
        allow_open_shell=False,
        z_table={"H": 1, "O": 8, "C": 6, "Xx": 0},
    )
    if not overrides:
        return base
    from dataclasses import replace

    return replace(base, **overrides)


def test_count_atoms_water():
    assert count_atoms(_water_adt()) == 3


def test_accepts_small_water():
    summary = enforce_guards(
        _request(),
        _water_adt(),
        guards=_loose_guards(),
        mode="local",
    )
    assert summary["n_atoms"] == 3
    assert summary["n_electrons"] == 10


def test_rejects_atom_cap(monkeypatch):
    with pytest.raises(DftGuardError, match="atom cap"):
        enforce_guards(
            _request(),
            _water_adt(),
            guards=_loose_guards(max_atoms=2),
            mode="local",
        )


def test_rejects_open_shell_by_default():
    with pytest.raises(DftGuardError, match="open-shell"):
        enforce_guards(
            _request(multiplicity=2),
            _water_adt(),
            guards=_loose_guards(),
            mode="local",
        )


def test_rejects_spend_cap():
    # 600s * $10/h = $1.666... still under 2.50; use short budget.
    with pytest.raises(DftGuardError, match="spend cap"):
        enforce_guards(
            _request(),
            _water_adt(),
            guards=_loose_guards(
                max_wall_seconds=3600,
                gpu_hourly_usd=10.0,
                max_estimated_usd=0.50,
            ),
            mode="modal",
        )


def test_operator_flag_required(monkeypatch):
    monkeypatch.delenv("CHIMIACLAW_DFT_LIVE_OPERATOR", raising=False)
    with pytest.raises(DftGuardError, match="operator gate"):
        enforce_guards(
            _request(),
            _water_adt(),
            guards=_loose_guards(require_operator_flag=True),
            mode="modal",
        )


def test_operator_flag_allows(monkeypatch):
    monkeypatch.setenv("CHIMIACLAW_DFT_LIVE_OPERATOR", "1")
    summary = enforce_guards(
        _request(),
        _water_adt(),
        guards=_loose_guards(require_operator_flag=True),
        mode="modal",
    )
    assert summary["mode"] == "modal"


def test_estimate_job_usd():
    assert estimate_job_usd(3600, 4.0) == 4.0
    assert abs(estimate_job_usd(600, 4.0) - (600 / 3600) * 4.0) < 1e-9
