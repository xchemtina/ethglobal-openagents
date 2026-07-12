# ChimiaClaw DFT on Modal

Elastic GPU backend for live DFT. Same **stdin → stdout** contract as
[`../dft`](../dft) so Rust `CHIMIACLAW_DFT_COMMAND` / `chimiaclaw-dft-skala`
does not change.

```
paid dft.live_*  →  chimiaclaw-cli  →  chimiaclaw-dft-modal
                                           ├─ stub  (offline)
                                           ├─ local (PySCF on this host)
                                           └─ modal (H100 / A10G / …)
                                      → chem.dft.result JSON
                                      → Rust signs artifact
```

**Signing never happens on Modal.** The container returns science JSON only.

## Modes

| Mode | GPU spend | Use |
|------|-----------|-----|
| `stub` | none | CI, cashier wiring, guard dry-run |
| `local` | none (laptop/Olympus CPU) | debug SCF without Modal |
| `modal` | yes | production elastic compute |

## Hard guards (before any GPU reserve)

| Env | Default | Meaning |
|-----|---------|---------|
| `CHIMIACLAW_DFT_MAX_ATOMS` | `40` | atom count cap |
| `CHIMIACLAW_DFT_MAX_ELECTRONS` | `200` | electron budget |
| `CHIMIACLAW_DFT_MAX_WALL_SECONDS` | `600` | Modal function timeout |
| `CHIMIACLAW_DFT_GPU_HOURLY_USD` | `4.00` | price used for estimate |
| `CHIMIACLAW_DFT_MAX_ESTIMATED_USD` | `2.50` | matches `dft.live_small` SKU |
| `CHIMIACLAW_DFT_LIVE_OPERATOR` | unset | **must be `1` for modal/local live** |
| `CHIMIACLAW_DFT_ALLOW_OPEN_SHELL` | `0` | UKS / multiplicity ≠ 1 |
| `CHIMIACLAW_DFT_REQUIRE_OPERATOR` | `1` | enforce operator flag |
| `CHIMIACLAW_MODAL_GPU` | `H100` | Modal GPU class |
| `CHIMIACLAW_MODAL_APP` | `chimiaclaw-dft` | Modal app name |

Rejected jobs exit **code 3** with stderr `dft-modal: rejected by guards: …`.

## Install

```bash
cd skills/scienceclaw-port/workers/dft_modal
uv sync --extra dev
uv run pytest

# When ready for real Modal:
uv sync --extra modal --extra pyscf
modal setup   # link Modal account / token
uv run modal deploy -m chimiaclaw_dft_modal.modal_app
```

## Wire as ChimiaClaw DFT command

```bash
export CHIMIACLAW_DFT_COMMAND="uv run --project skills/scienceclaw-port/workers/dft_modal chimiaclaw-dft-modal --mode stub"
# later:
export CHIMIACLAW_DFT_LIVE_OPERATOR=1
export CHIMIACLAW_DFT_MODAL_MODE=modal
export CHIMIACLAW_DFT_COMMAND="uv run --project skills/scienceclaw-port/workers/dft_modal chimiaclaw-dft-modal --mode modal"
```

## Smoke (stub, offline)

```bash
cat > /tmp/dft-job.json <<'JSON'
{
  "request": {
    "request_id": "REQ.MODAL.SMOKE.WATER",
    "total_charge": 0,
    "multiplicity": 1,
    "molecule": {"molecule_id": "water", "canonical_smiles": "O"},
    "method": {"functional": "pbe", "basis_set": "def2-svp"},
    "requested_properties": ["total_energy"]
  },
  "molecule_adt": {
    "atoms": {
      "0": {"attributes": {"symbol": "O"}, "coordinate": {"x_angstrom": 0.0, "y_angstrom": 0.0, "z_angstrom": 0.1173}},
      "1": {"attributes": {"symbol": "H"}, "coordinate": {"x_angstrom": 0.0, "y_angstrom": 0.7572, "z_angstrom": -0.4692}},
      "2": {"attributes": {"symbol": "H"}, "coordinate": {"x_angstrom": 0.0, "y_angstrom": -0.7572, "z_angstrom": -0.4692}}
    }
  }
}
JSON

uv run --project . chimiaclaw-dft-modal --mode stub < /tmp/dft-job.json | jq '.convergence,.provenance'
```

Expect `converged: false` and `source_kind: stub-result` — Rust will **refuse to sign** as a real SCF. That is intentional.

## Swarm (10–50 H100s)

Do **not** request 50 GPUs for one molecule. Fan out:

```python
# After deploy: modal.Function.from_name("chimiaclaw-dft", "run_dft_batch")
# payloads = [wrapper_dict_for_each_molecule, ...]
# results = run_dft_batch.remote(payloads)
```

Each item is one guarded SCF; Modal schedules containers across the fleet.

## Public API honesty

- Gateway `dft.cached_result` — paid cache, **no Modal**.
- Gateway `dft.live_small` — stays **501 / operator-gated** until
  `CHIMIACLAW_DFT_LIVE_OPERATOR=1`, Modal is deployed, and spend caps are reviewed.
- Never claim live H100 settlement without those three.

See also `docs/MODAL_DFT.md`.
