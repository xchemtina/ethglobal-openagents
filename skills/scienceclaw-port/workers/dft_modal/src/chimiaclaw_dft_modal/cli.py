"""CLI entry for ``CHIMIACLAW_DFT_COMMAND``.

Modes
-----
stub
    Deterministic non-converged result (Rust will not sign as real).
local
    In-process PySCF on this machine (requires ``--extra pyscf``).
modal
    Reserve Modal GPU via deployed ``chimiaclaw-dft`` app (requires Modal account).

Guards always run before ``local`` / ``modal`` compute.
"""

from __future__ import annotations

import argparse
import os
import sys
from typing import Any

from .guards import DftGuardError, enforce_guards, load_guards_from_env
from .io_util import read_worker_input, stub_result, wrap_payload, write_result


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="chimiaclaw-dft-modal",
        description=(
            "ChimiaClaw DFT via Modal/local/stub. "
            "Same stdin→stdout contract as chimiaclaw-dft."
        ),
    )
    parser.add_argument(
        "--mode",
        choices=("stub", "local", "modal"),
        default=os.environ.get("CHIMIACLAW_DFT_MODAL_MODE", "stub"),
        help="Execution backend (env CHIMIACLAW_DFT_MODAL_MODE).",
    )
    parser.add_argument(
        "--stub",
        action="store_true",
        help="Alias for --mode stub.",
    )
    parser.add_argument(
        "--skip-guards",
        action="store_true",
        help="Dangerous: skip atom/time/spend guards (operator break-glass only).",
    )
    return parser.parse_args(argv)


def _run_local(
    request: dict[str, Any],
    molecule_adt: dict[str, Any] | None,
    cube_grid: dict[str, Any] | None,
) -> dict[str, Any]:
    if molecule_adt is None:
        raise SystemExit("dft-modal local mode requires molecule_adt on stdin")
    try:
        from .scf import run_scf
    except ImportError as exc:  # pragma: no cover
        raise SystemExit(
            "dft-modal: local SCF import failed; install with "
            "`uv sync --extra pyscf`"
        ) from exc
    try:
        return run_scf(request, molecule_adt, cube_grid, host_label="local")
    except ImportError as exc:
        raise SystemExit(
            "dft-modal: PySCF missing; `uv sync --extra pyscf` or use --mode stub"
        ) from exc


def _run_modal(
    request: dict[str, Any],
    molecule_adt: dict[str, Any] | None,
    cube_grid: dict[str, Any] | None,
) -> dict[str, Any]:
    if molecule_adt is None:
        raise SystemExit("dft-modal modal mode requires molecule_adt on stdin")
    payload = wrap_payload(request, molecule_adt, cube_grid)
    try:
        from .modal_app import invoke_remote
    except ImportError as exc:
        raise SystemExit(
            "dft-modal: modal package missing; `uv sync --extra modal` "
            "and run `modal setup`"
        ) from exc
    return invoke_remote(payload)


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(list(argv) if argv is not None else sys.argv[1:])
    mode = "stub" if args.stub else args.mode

    request, molecule_adt, cube_grid = read_worker_input()

    if mode != "stub" and not args.skip_guards:
        try:
            summary = enforce_guards(request, molecule_adt, mode=mode)
            print(
                f"dft-modal: guards ok {summary}",
                file=sys.stderr,
            )
        except DftGuardError as exc:
            print(f"dft-modal: rejected by guards: {exc}", file=sys.stderr)
            return 3

    if mode == "stub":
        # Stub still reports what guards *would* do (no operator flag required).
        try:
            from dataclasses import replace

            dry = replace(load_guards_from_env(), require_operator_flag=False)
            summary = enforce_guards(
                request, molecule_adt, guards=dry, mode="stub"
            )
            note = f"guard dry-run would accept: {summary}"
        except DftGuardError as exc:
            note = f"guard dry-run would reject: {exc}"
        result = stub_result(request, note=note)
        write_result(result)
        return 0

    if mode == "local":
        result = _run_local(request, molecule_adt, cube_grid)
        write_result(result)
        return 0

    if mode == "modal":
        result = _run_modal(request, molecule_adt, cube_grid)
        write_result(result)
        return 0

    print(f"dft-modal: unknown mode {mode!r}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
