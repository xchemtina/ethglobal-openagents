"""ChimiaClaw DFT worker CLI.

Reads a `chem.dft.request` JSON document on stdin and writes a `chem.dft.result`
JSON document on stdout.

Audit posture:
- secrets are read from environment, never argv (no DFT secrets today, but the
  contract is reserved);
- `--stub` mode emits a deterministic placeholder result tagged
  `provenance.source_kind = "stub-result"` and `convergence.converged = false`,
  so the Rust adapter (`chimiaclaw-dft-skala`) refuses to sign it as a real
  artifact.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import sys
import time
from dataclasses import asdict, dataclass, field
from typing import Any


SCHEMA_TAG = "chem.dft.result"
SUPPORTED_BACKENDS = ("pyscf-classical", "pyscf-skala", "stub")


@dataclass
class Convergence:
    converged: bool
    n_cycles: int
    final_gradient_norm: float | None = None
    scf_threshold: float | None = None


@dataclass
class Orbitals:
    homo_hartree: float
    lumo_hartree: float
    gap_hartree: float
    gap_ev: float


@dataclass
class Dipole:
    x_debye: float
    y_debye: float
    z_debye: float
    magnitude_debye: float


@dataclass
class Timings:
    wall_seconds: float
    cpu_seconds: float | None = None


@dataclass
class Provenance:
    source_kind: str
    source_ref: str
    host: str | None = None
    pyscf_version: str | None = None
    skala_version: str | None = None
    dispersion: str | None = None
    notes: list[str] = field(default_factory=list)


@dataclass
class DftResult:
    schema_tag: str
    request_id: str
    molecule_id: str
    functional: str
    basis_set: str
    backend: str
    total_charge: int
    multiplicity: int
    energy_hartree: float
    orbitals: Orbitals | None
    dipole: Dipole | None
    convergence: Convergence
    timings: Timings
    requested_properties: list[str]
    provenance: Provenance

    def to_dict(self) -> dict[str, Any]:
        # asdict gives nested dicts; replace Nones-as-missing where the
        # contract uses optional fields.
        return asdict(self)


def _read_input() -> tuple[dict[str, Any], dict[str, Any] | None]:
    """Parse stdin into (request, molecule_adt).

    Accepts either the new wrapper format ({request, molecule_adt}) or, for
    backward compat with --stub callers, a flat DftRequest with molecule_adt=None.
    """
    raw = sys.stdin.read()
    if not raw.strip():
        raise SystemExit("dft worker: empty stdin; expected JSON")
    try:
        document = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"dft worker: invalid JSON on stdin: {exc}") from exc
    if isinstance(document, dict) and "request" in document and "molecule_adt" in document:
        request = document["request"]
        molecule_adt = document["molecule_adt"]
        if not isinstance(request, dict):
            raise SystemExit("dft worker: wrapper.request must be an object")
        if not isinstance(molecule_adt, dict):
            raise SystemExit("dft worker: wrapper.molecule_adt must be an object")
        return request, molecule_adt
    if isinstance(document, dict):
        return document, None
    raise SystemExit("dft worker: stdin must be a JSON object")


def _stub_result(request: dict[str, Any]) -> DftResult:
    method = request.get("method", {})
    molecule = request.get("molecule", {})
    return DftResult(
        schema_tag=SCHEMA_TAG,
        request_id=str(request.get("request_id", "REQ.UNKNOWN")),
        molecule_id=str(molecule.get("molecule_id", "unknown")),
        functional=str(method.get("functional", "stub")),
        basis_set=str(method.get("basis_set", "stub")),
        backend="StubBackend",
        total_charge=int(request.get("total_charge", 0)),
        multiplicity=int(request.get("multiplicity", 1)),
        energy_hartree=0.0,
        orbitals=None,
        dipole=None,
        convergence=Convergence(
            converged=False,
            n_cycles=0,
            final_gradient_norm=None,
            scf_threshold=None,
        ),
        timings=Timings(wall_seconds=0.0, cpu_seconds=None),
        requested_properties=list(request.get("requested_properties", [])),
        provenance=Provenance(
            source_kind="stub-result",
            source_ref="chimiaclaw-dft cli --stub",
            host=platform.node(),
            pyscf_version=None,
            skala_version=None,
            dispersion=method.get("dispersion"),
            notes=[
                "STUB MODE: no SCF was performed; "
                "the Rust signer will refuse this because convergence.converged=False.",
            ],
        ),
    )


def _real_pyscf_result(
    request: dict[str, Any],
    molecule_adt: dict[str, Any] | None,
    backend: str,
) -> DftResult:
    """Dispatch to the real backend.  Imported lazily so --stub doesn't need
    PySCF.
    """
    try:
        import pyscf  # noqa: F401  pylint: disable=import-outside-toplevel
    except ImportError as exc:
        raise SystemExit(
            "dft worker: PySCF is not installed in this uv project; "
            "either run with --stub, or `uv sync` the project on this host."
        ) from exc

    if molecule_adt is None:
        raise SystemExit(
            "dft worker: real backends need molecule_adt (atoms with coordinates); "
            "send the {request, molecule_adt} wrapper on stdin or run with --stub."
        )

    if backend in ("pyscf-classical", "pyscf-skala"):
        # pyscf-skala maps to PBE today (with a fallback note).  The duck-side
        # agent can replace this with a real Skala 1.1 backend later.
        from . import pyscf_backend  # local import to avoid circular cost

        return pyscf_backend.run(request, molecule_adt)

    raise SystemExit(f"dft worker: unknown backend {backend!r}")


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="chimiaclaw-dft",
        description="ChimiaClaw DFT worker (chem.dft.request -> chem.dft.result).",
    )
    parser.add_argument(
        "--backend",
        choices=SUPPORTED_BACKENDS,
        default="pyscf-classical",
        help=(
            "Which backend to dispatch to.  --stub overrides this and skips "
            "all SCF code paths.  pyscf-classical handles PBE/B3LYP/...; "
            "pyscf-skala falls back to PBE today and will use real Skala 1.1 "
            "once weights are wired."
        ),
    )
    parser.add_argument(
        "--stub",
        action="store_true",
        help="Skip PySCF entirely and emit a deterministic stub result.",
    )
    parser.add_argument(
        "--allow-stub-with-real",
        action="store_true",
        help=(
            "Allow --stub even when CHIMIACLAW_DFT_REAL=1.  Default is to refuse "
            "stub mode in environments that have explicitly opted into real runs."
        ),
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(list(argv) if argv is not None else sys.argv[1:])
    real_required = os.environ.get("CHIMIACLAW_DFT_REAL", "").strip() == "1"
    if args.stub and real_required and not args.allow_stub_with_real:
        print(
            "dft worker: refusing --stub because CHIMIACLAW_DFT_REAL=1; "
            "pass --allow-stub-with-real if this is intentional.",
            file=sys.stderr,
        )
        return 2

    request, molecule_adt = _read_input()
    schema = request.get("schema_tag")
    # The Rust DftRequest doesn't carry a schema_tag field today; this is
    # advisory.  If the operator embeds one, we accept it.
    if schema is not None and schema != "chem.dft.request":
        print(
            f"dft worker: unexpected schema_tag {schema!r} on stdin; "
            "expected chem.dft.request",
            file=sys.stderr,
        )
        return 2

    if args.stub or args.backend == "stub":
        result = _stub_result(request)
    else:
        result = _real_pyscf_result(request, molecule_adt, args.backend)

    json.dump(result.to_dict(), sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
