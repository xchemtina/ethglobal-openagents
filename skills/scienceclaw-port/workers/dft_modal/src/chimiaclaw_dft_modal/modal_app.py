"""Modal app: elastic GPU SCF for ChimiaClaw.

Deploy (after ``modal token set`` / ``modal setup``)::

    cd skills/scienceclaw-port/workers/dft_modal
    uv sync --extra modal --extra pyscf
    uv run modal deploy -m chimiaclaw_dft_modal.modal_app

Remote invoke is done by ``chimiaclaw-dft-modal --mode modal`` which looks up
the deployed function by app/name. This module must stay importable without
``modal`` installed so unit tests and stub mode work offline.
"""

from __future__ import annotations

import os
from typing import Any

# Modal is an optional dependency; keep import soft for offline tests.
try:
    import modal
except ImportError:  # pragma: no cover
    modal = None  # type: ignore


APP_NAME = os.environ.get("CHIMIACLAW_MODAL_APP", "chimiaclaw-dft")
FUNCTION_NAME = os.environ.get("CHIMIACLAW_MODAL_FUNCTION", "run_dft_job")
GPU = os.environ.get("CHIMIACLAW_MODAL_GPU", "H100")
TIMEOUT = int(os.environ.get("CHIMIACLAW_DFT_MAX_WALL_SECONDS", "600"))
MEMORY_MIB = int(os.environ.get("CHIMIACLAW_MODAL_MEMORY_MIB", "32768"))


def _build_app() -> Any:
    if modal is None:
        raise RuntimeError(
            "modal package not installed; `uv sync --extra modal` in dft_modal"
        )

    app = modal.App(APP_NAME)

    image = (
        modal.Image.debian_slim(python_version="3.12")
        .pip_install(
            "numpy>=1.26",
            "scipy>=1.12",
            "h5py>=3.10",
            "pyscf>=2.7",
        )
        .env({"OMP_NUM_THREADS": "8"})
    )

    # Prefer adding local source when available (Modal ≥0.64). Fall back to
    # embedding a minimal copy path via pip install of this project if needed.
    try:
        image = image.add_local_python_source("chimiaclaw_dft_modal")
    except Exception:  # noqa: BLE001
        # Older modal / non-package layout: image still has pyscf; function
        # will import scf from the deployed package if operator mounts it.
        pass

    @app.function(
        image=image,
        gpu=GPU,
        timeout=TIMEOUT,
        memory=MEMORY_MIB,
        retries=0,
    )
    def run_dft_job(payload: dict[str, Any]) -> dict[str, Any]:
        """Remote SCF. ``payload`` is the full worker wrapper dict."""
        from chimiaclaw_dft_modal.guards import DftGuardError, enforce_guards
        from chimiaclaw_dft_modal.scf import run_scf

        request = payload.get("request") if isinstance(payload, dict) else None
        molecule_adt = payload.get("molecule_adt") if isinstance(payload, dict) else None
        cube_grid = payload.get("cube_grid") if isinstance(payload, dict) else None
        if not isinstance(request, dict):
            raise ValueError("payload.request must be an object")
        # Re-enforce guards inside the container (defense in depth).
        try:
            enforce_guards(request, molecule_adt, mode="modal")
        except DftGuardError as exc:
            raise RuntimeError(str(exc)) from exc
        if not isinstance(molecule_adt, dict):
            raise ValueError("payload.molecule_adt required on Modal")
        return run_scf(
            request,
            molecule_adt,
            cube_grid if isinstance(cube_grid, dict) else None,
            host_label=f"modal:{GPU}",
        )

    @app.function(
        image=image,
        gpu=GPU,
        timeout=TIMEOUT,
        memory=MEMORY_MIB,
    )
    def run_dft_batch(payloads: list[dict[str, Any]]) -> list[dict[str, Any]]:
        """Map many jobs across containers (10–50 H100 swarm entrypoint)."""
        # Nested map: each item is one molecule job.
        return list(run_dft_job.map(payloads))

    @app.local_entrypoint()
    def main(path: str = "") -> None:
        """``modal run -m chimiaclaw_dft_modal.modal_app --path job.json``"""
        import json
        from pathlib import Path

        if not path:
            print("usage: modal run -m chimiaclaw_dft_modal.modal_app --path job.json")
            return
        payload = json.loads(Path(path).read_text(encoding="utf-8"))
        result = run_dft_job.remote(payload)
        print(json.dumps(result, indent=2))

    # Expose for deploy introspection.
    app.run_dft_job = run_dft_job  # type: ignore[attr-defined]
    app.run_dft_batch = run_dft_batch  # type: ignore[attr-defined]
    return app


# Construct only when modal is present so `import modal_app` in tests is safe
# only if modal installed — cli imports invoke_remote lazily.
app = _build_app() if modal is not None else None


def invoke_remote(payload: dict[str, Any]) -> dict[str, Any]:
    """Call the deployed Modal function (or run ephemerally if undeployed)."""
    if modal is None:
        raise RuntimeError("modal package not installed")

    # Prefer deployed lookup so CHIMIACLAW_DFT_COMMAND stays a cheap local process.
    try:
        fn = modal.Function.from_name(APP_NAME, FUNCTION_NAME)
        return fn.remote(payload)
    except Exception as lookup_exc:  # noqa: BLE001
        # Fall back to ephemeral app.run() for first-time smoke.
        built = _build_app()
        with built.run():
            try:
                return built.run_dft_job.remote(payload)  # type: ignore[attr-defined]
            except Exception as run_exc:  # noqa: BLE001
                raise RuntimeError(
                    f"Modal invoke failed (deployed lookup: {lookup_exc}; "
                    f"ephemeral: {run_exc}). Run `modal setup` and "
                    f"`uv run modal deploy -m chimiaclaw_dft_modal.modal_app`."
                ) from run_exc
