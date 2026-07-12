# Modal + ChimiaClaw DFT

## Goal

Run live / batch quantum chemistry on elastic GPUs (including H100 fleets) without
putting the DAO cashier, signing keys, or website on Modal.

## Architecture

```text
chimiadao.io / agents  →  api-gateway (x402)
                              ↓ paid live job (future)
                         chimiaclaw-cli live dft-execute
                              ↓ CHIMIACLAW_DFT_COMMAND
                         chimiaclaw-dft-modal
                              ↓ --mode modal
                         Modal app "chimiaclaw-dft"  (H100 × N jobs)
                              ↓ DftResult JSON
                         Rust signs chem.dft.result
```

| Layer | Host | Notes |
|-------|------|--------|
| Brand + catalog | Vercel / chimiadao.io | free discovery |
| Cashier | `services/api-gateway` | free / stub / live x402 |
| Artifact truth | local / 0G | signed DAG |
| Heavy SCF | **Modal** | this doc |
| Lab trust edge | Olympus | optional physical node |

## Package

[`skills/scienceclaw-port/workers/dft_modal`](../skills/scienceclaw-port/workers/dft_modal)

Same JSON contract as the Olympus worker:

- **in:** `{ "request", "molecule_adt", "cube_grid?" }`
- **out:** `chem.dft.result` worker JSON (`orbital_cubes` optional)
- **fail:** non-zero exit; guards use exit code **3**

## Account link checklist

1. Create / log into Modal account for the DAO (team workspace preferred).
2. On an operator machine: `pipx install modal` or `uv sync --extra modal` in `dft_modal`.
3. `modal setup` (token) — never commit tokens; store in operator secret store / CI OIDC later.
4. `uv run modal deploy -m chimiaclaw_dft_modal.modal_app`
5. Smoke one remote water job with `CHIMIACLAW_DFT_LIVE_OPERATOR=1`.
6. Point `CHIMIACLAW_DFT_COMMAND` at `chimiaclaw-dft-modal --mode modal`.
7. Only then consider flipping gateway `dft.live_small` off of 501.

## Spend model

Default estimate:

```text
estimated_usd = (MAX_WALL_SECONDS / 3600) * GPU_HOURLY_USD
```

Must be ≤ `CHIMIACLAW_DFT_MAX_ESTIMATED_USD` (default **$2.50** = catalog `dft.live_small`).

Tune `CHIMIACLAW_DFT_GPU_HOURLY_USD` to match Modal’s actual list price for the
selected GPU class. Prefer **A10G / L4** for small organics; reserve **H100** for
Skala / larger systems / batch pressure.

## Batch / 10–50 H100s

Use **map** over molecules (`run_dft_batch`), not multi-GPU single SCF:

- 50 independent jobs → up to 50 containers
- Each job still passes atom / electron / open-shell / spend guards
- Artifact DAG gets one signed result per molecule

## What stays off Modal

- `CHIMIA_X402_PAY_TO` private keys / facilitator secrets  
- Artifact signing seeds  
- Free-run of uncapped libraries  
- Claiming live USDC settlement for DFT until x402 live mode is real  

## Status

| Piece | State |
|-------|--------|
| Worker package + guards + stub CLI | **landed** |
| Offline unit tests | **landed** |
| Modal deploy + remote SCF | operator (needs account token) |
| Gateway `POST /v1/dft/live` | still 501 until operator enablement |
| Skala weights on Modal volume | future |
| Cube volumes on Modal | future (Olympus path still best for cube gallery) |
