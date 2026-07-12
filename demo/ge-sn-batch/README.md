# Ge → Sn DFT starting-point batch

Ten Sn-containing starting geometries derived from trusted Ge XYZs in
`ChimiaDAO-MNT/calculations/structures/`, for single-point PySCF on
`duck@olympus.local`.

## What this is

| Item | Value |
|------|--------|
| Count | **10** molecules |
| Transform | `Ge` → `Sn` + metal–ligand scale by `r(Sn)/r(Ge) = 1.39/1.20` |
| Default method | PBE / def2-svp (matches overnight Ge scalar) |
| Geometry quality | **Starting points only** — re-opt before publishing energies as chemistry claims |

## Molecules

See [`manifest.json`](manifest.json). Includes:

- `NC3Sn_H`, `NC3Sn_Cl` — tricarbastannatrane parents (Kavoosi-class cage)
- β-OH / β-SH functionalized cages
- Adamantyl stannatrane + `Ad_SnH3/Cl3/Me3`

Ge references copied under `xyz_ge_ref/`.

## Rebuild XYZs

```bash
# after editing tools/ge_sn_batch/ge_to_sn.py or sources
python3 tools/ge_sn_batch/ge_to_sn.py \
  /Users/crischimiadao/Documents/ChimiaDAO-MNT/calculations/structures/NC3Ge_H_opt.xyz \
  -o demo/ge-sn-batch/xyz/NC3Sn_H.xyz
```

Or regenerate the full pack with the inventory builder (see session notes /
`tools/ge_sn_batch/`).

## Run on Olympus

```bash
# plan only
./demo/ge-sn-batch/run_olympus.sh

# one molecule
./demo/ge-sn-batch/run_olympus.sh --label NC3Sn_H --execute

# full batch (operator; tens of minutes for adamantyl species)
./demo/ge-sn-batch/run_olympus.sh --execute
```

Results land in `demo/ge-sn-batch/results/` as **raw worker JSON** first.
Seal as ChimiaClaw artifacts with `live dft-execute` once MolADT parents exist.

## View in ChimeraX

```bash
# via MCP (tools/chimerax_mcp) or CLI:
/Applications/ChimeraX-1.11.1.app/Contents/bin/ChimeraX \
  --cmd "open demo/ge-sn-batch/xyz/NC3Sn_H.xyz; open demo/ge-sn-batch/xyz_ge_ref/NC3Ge_H_src.xyz; tile"
```

## Honesty

- Not crystal structures from Kavoosi.
- Not yet signed `chem.dft.result` until executed + sealed.
- Si-blocked TBS precursor remains blocked separately; these jobs are Sn/Ge only.
