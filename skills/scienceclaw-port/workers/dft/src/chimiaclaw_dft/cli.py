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
SUPPORTED_BACKENDS = ("pyscf-skala", "pyscf-classical", "stub")


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


def _read_request() -> dict[str, Any]:
    raw = sys.stdin.read()
    if not raw.strip():
        raise SystemExit("dft worker: empty stdin; expected chem.dft.request JSON")
    try:
        return json.loads(raw)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"dft worker: invalid JSON on stdin: {exc}") from exc


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


def _real_pyscf_result(request: dict[str, Any], backend: str) -> DftResult:
    """Stub for the real PySCF + Skala / classical functional path.

    The duck-side agent fills this in.  We import lazily so `--stub` works
    even when PySCF isn't installed.
    """
    try:
        import pyscf  # noqa: F401  pylint: disable=import-outside-toplevel
    except ImportError as exc:
        raise SystemExit(
            "dft worker: PySCF is not installed in this uv project; "
            "either run with --stub, or `uv sync` the project on this host."
        ) from exc

    # Placeholder until the duck-side agent wires Skala / classical PySCF.
    raise SystemExit(
        f"dft worker: backend {backend!r} not yet implemented; "
        "duck-side agent owns chimiaclaw_dft.{skala,pyscf_backend}.run."
    )


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="chimiaclaw-dft",
        description="ChimiaClaw DFT worker (chem.dft.request -> chem.dft.result).",
    )
    parser.add_argument(
        "--backend",
        choices=SUPPORTED_BACKENDS,
        default="pyscf-skala",
        help=(
            "Which backend to dispatch to.  --stub overrides this and skips "
            "all SCF code paths."
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

    started = time.time()
    request = _read_request()
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
        result = _real_pyscf_result(request, args.backend)
        # _real_pyscf_result currently raises; once implemented it will
        # populate timings.wall_seconds itself.
        result.timings.wall_seconds = time.time() - started

    json.dump(result.to_dict(), sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
