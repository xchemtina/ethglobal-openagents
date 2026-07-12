# ChimiaClaw DFT worker

uv-managed Python worker that reads a `chem.dft.request` JSON document on stdin
and writes a `chem.dft.result` JSON document on stdout.

> **No Docker, no Homebrew, no pip-only flows.** Everything is uv-managed.

## Contract

- **stdin**: a JSON document matching `chimiaclaw_moladt::DftRequest`.
- **stdout**: a JSON document matching `chimiaclaw_dft_skala::DftResult`, with
  `schema_tag = "chem.dft.result"`.
- **non-zero exit + stderr message** on failure.

The Rust adapter (`chimiaclaw-dft-skala`) signs the result as a
`chem.dft.result` artifact parented to the `chem.dft.request` artifact and
refuses to sign if `convergence.converged` is false.

## Install (on duck@olympus.local)

```sh
cd ~/Documents/ChimiaDAO-QM/DFT
uv python install 3.12
uv venv
uv sync --project ./skills/scienceclaw-port/workers/dft
# Real Skala 1.1 install path is up to the operator; once installed:
uv pip install -e ./skills/scienceclaw-port/workers/dft[dispersion]
```

## Stub mode (CI / offline / scaffolding)

```sh
uv run --project ./skills/scienceclaw-port/workers/dft chimiaclaw-dft \
  --stub < /tmp/chem-dft-request.json > /tmp/chem-dft-result.json
```

`--stub` skips PySCF entirely, parses the request, and emits a deterministic
placeholder result with `provenance.source_kind = "stub-result"` and
`convergence.converged = false` so signed artifacts can never silently
impersonate a real SCF.

## Real mode (PySCF + Skala 1.1)

```sh
export PYSCF_VERSION="$(uv run python -c 'import pyscf; print(pyscf.__version__)')"
export SKALA_VERSION="1.1"
uv run --project ./skills/scienceclaw-port/workers/dft chimiaclaw-dft \
  --backend pyscf-skala \
  < /tmp/chem-dft-request.json > /tmp/chem-dft-result.json
```

The duck-side agent owns the actual Skala 1.1 install (model weights,
inference path, PySCF integration) — see `chimiaclaw_dft/skala.py` for the
hook.

## End-to-end smoke (Rust + worker)

```sh
export CHIMIACLAW_DFT_COMMAND="ssh duck@olympus.local 'uv run --project ~/Documents/ChimiaDAO-QM/DFT/skills/scienceclaw-port/workers/dft chimiaclaw-dft --backend pyscf-skala'"
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live dft-execute \
  --request-artifact-json /tmp/chem-dft-request-artifact.json \
  --out-dir demo/dft/
```

## Elastic GPUs (Modal)

For H100 / swarm throughput, use the sibling package
[`../dft_modal`](../dft_modal) as `CHIMIACLAW_DFT_COMMAND` instead of
ssh-to-Olympus. Same stdin/stdout contract; see `docs/MODAL_DFT.md`.

## Schema reference

`chem.dft.result` (canonical):

```json
{
  "schema_tag": "chem.dft.result",
  "request_id": "REQ.DFT.SKALA.FERROCENE.001",
  "molecule_id": "ferrocene",
  "functional": "skala-1.1",
  "basis_set": "def2-tzvp",
  "backend": "PyScf",
  "total_charge": 0,
  "multiplicity": 1,
  "energy_hartree": -1648.123456,
  "orbitals": {
    "homo_hartree": -0.2145,
    "lumo_hartree": -0.0251,
    "gap_hartree": 0.1894,
    "gap_ev": 5.154
  },
  "dipole": { "x_debye": 0, "y_debye": 0, "z_debye": 0, "magnitude_debye": 0 },
  "convergence": {
    "converged": true,
    "n_cycles": 18,
    "final_gradient_norm": 1.2e-7,
    "scf_threshold": 1.0e-8
  },
  "timings": { "wall_seconds": 41.7, "cpu_seconds": 165.2 },
  "requested_properties": ["total_energy", "homo_lumo_gap", "dipole"],
  "provenance": {
    "source_kind": "pyscf-skala-1.1",
    "source_ref": "duck@olympus.local:pyscf-2.11.0+skala-1.1",
    "host": "duck@olympus.local",
    "pyscf_version": "2.11.0",
    "skala_version": "1.1",
    "dispersion": "dftd3",
    "notes": []
  }
}
```

`provenance.source_kind` MUST be one of:

- `"pyscf-skala-1.1"` — canonical Skala 1.1 path.
- `"pyscf-classical-functional"` — any non-Skala PySCF SCF (e.g. PBE, B3LYP).
- `"stub-result"` — `--stub` mode.  The Rust signer also rejects this when
  `convergence.converged` is false.

## Audit posture

- private keys / API keys are read from environment, never argv;
- no Docker, no Homebrew, no pip;
- `--stub` mode is explicitly labelled and produces deterministic output;
- the Rust signer rejects `convergence.converged = false` and any
  schema_tag mismatch, so the artifact graph never contains a fake result.
