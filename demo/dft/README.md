# ChimiaClaw signed DFT result gallery

Real PBE/def2-tzvp SCF calculations on `duck@olympus.local` through PySCF
2.13.0, signed as `chem.dft.result` artifacts by `chimiaclaw-dft-skala`.

Every result here is reproducible with:

```sh
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  moladt-dft-demo --library <name> --functional pbe --basis def2-tzvp \
  --out-dir demo/dft/

export CHIMIACLAW_DFT_COMMAND="ssh duck@olympus.local /Users/duck/.local/bin/uv run --project /Users/duck/Documents/ChimiaDAO-QM/DFT/skills/scienceclaw-port/workers/dft chimiaclaw-dft --backend pyscf-classical"

cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live dft-execute \
  --request-artifact-json demo/dft/chem_dft_request.<id>.json \
  --molecule-artifact-json demo/dft/chem_molecule_adt.<id>.json \
  --out-dir demo/dft/
```

## Gallery (PBE/def2-tzvp on `Olympus.local`)

```
MOLADT.WATER.001         E=  -76.376421Ha   gap=6.964eV   |μ|=2.028D   wall=0.16s
MOLADT.METHANOL.001      E= -115.626291Ha   gap=6.025eV   |μ|=1.706D   wall=0.76s
MOLADT.BENZENE.001       E= -232.018795Ha   gap=5.129eV   |μ|=0.000D   wall=9.34s
```

Dipole magnitudes match chemistry intuition: water and methanol are polar
(literature: ~1.85 D and ~1.69 D respectively; PBE/def2-tzvp slightly
overestimates water as expected); benzene's six-fold symmetry forces
|μ| → 0 (numerically 0.000 D, recovered correctly).

## Files

- `chem_molecule_adt.<id>.json` — signed `chem.molecule.adt` (curated MolADT
  geometry, atoms with x/y/z in Ångström).
- `chem_dft_request.<id>.json` — signed `chem.dft.request` (functional, basis,
  charge, multiplicity, dispersion). Parented to the matching molecule
  artifact.
- `chem_dft_result.<id>.json` — signed `chem.dft.result` (energy, frontier
  orbitals, convergence, timings, provenance). Parented to the matching
  request artifact.

| molecule  | molecule artifact          | request artifact           | result artifact            |
| --------- | -------------------------- | -------------------------- | -------------------------- |
| water     | `art_a1260505a4c2c867`     | `art_c297438e1d252604`     | `art_c5094c8d71bda378`     |
| methanol  | `art_3270cc598fcb7c73`     | `art_8c77e87b34c6c65a`     | `art_7a93e7a0a013f675`     |
| benzene   | `art_fff6d384b83ab849`     | `art_30b545be1442d883`     | `art_b189177794da6f32`     |

## Verification

Every artifact JSON in this directory verifies independently:

```sh
cargo run -p chimiaclaw-cli -- artifact inspect \
  --store-dir <a fresh dir into which you've copied the JSONs>
```

The Rust signer refuses to sign any `chem.dft.result` whose
`convergence.converged` is `false` or whose `schema_tag` is wrong, so the
artifact graph cannot contain a fake SCF.

## Provenance posture

`provenance.source_kind` is one of:

- `pyscf-classical-functional` — real PySCF SCF with a stock functional (PBE,
  B3LYP, ...). All three artifacts in this gallery are this kind.
- `pyscf-skala-1.1` — real Skala 1.1 deep-learned XC functional. Not yet
  shipped; awaits weights install on duck. Until then,
  `--backend pyscf-skala` falls back to PBE with an explicit fallback notice
  in `provenance.notes` (see DECISIONS D15).
- `stub-result` — `--stub` mode placeholder; never signed by the Rust adapter.

## Caveats

- HOMO/LUMO gaps from PBE are systematically too small (well-known DFT
  problem). Trust the chemistry pattern, not the absolute eV numbers.
- All MolADT geometries here are `schematic-curated`, not relaxed. A
  production DFT pipeline should run a geometry optimization or take RDKit
  ETKDGv3+MMFF94 input via `CHIMIACLAW_SMILES_TO_MOLADT_COMMAND` first.
- `convergence.n_cycles` currently reports `0` because PySCF doesn't expose
  the cycle count on the `mf` attribute we read; the SCF really did converge
  (verified by `mf.converged = True`). Cycle accounting is a known TODO.
