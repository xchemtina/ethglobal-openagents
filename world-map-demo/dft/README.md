# ChimiaClaw signed DFT result gallery

Real PBE/def2-tzvp SCF calculations on `duck@olympus.local` through PySCF
2.13.0, signed as `chem.dft.result` artifacts by `chimiaclaw-dft-skala`.
Each result commits to HOMO, LUMO, and total electron density cubes via
SHA-256 (see `cubes/`).

## Reproduce

```sh
# 1. Curated MolADT  (use --library for water/methanol/benzene/...)
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  moladt-dft-demo --library water --functional pbe --basis def2-tzvp \
  --out-dir demo/dft/

# 1'. Or: arbitrary SMILES via the RDKit ETKDGv3 + MMFF94 worker
export CHIMIACLAW_SMILES_TO_MOLADT_COMMAND="ssh duck@olympus.local /Users/duck/.local/bin/uv run --project /Users/duck/Documents/ChimiaDAO-QM/DFT/skills/scienceclaw-port/workers/cheminformatics rdkit-smiles-to-moladt"
cargo run -p chimiaclaw-cli --features live-sponsors -- \
  moladt-dft-demo --smiles "OCC(O)C" --functional pbe --out-dir demo/dft/

# 2. SCF + cubes through duck's PySCF
export CHIMIACLAW_DFT_COMMAND="ssh duck@olympus.local /Users/duck/.local/bin/uv run --project /Users/duck/Documents/ChimiaDAO-QM/DFT/skills/scienceclaw-port/workers/dft chimiaclaw-dft --backend pyscf-classical"

cargo run -p chimiaclaw-cli --features live-sponsors -- \
  live dft-execute \
  --request-artifact-json demo/dft/chem_dft_request.<id>.json \
  --molecule-artifact-json demo/dft/chem_molecule_adt.<id>.json \
  --out-dir demo/dft/ \
  --cube-out-dir demo/dft/cubes \
  --cube-resolution 50
```

## Gallery (PBE/def2-tzvp, cube grid 50³, on `Olympus.local`)

```
water              E=  -76.376421Ha   gap=6.964eV   |μ|=2.028D   wall=  0.2s
methanol           E= -115.626291Ha   gap=6.025eV   |μ|=1.706D   wall=  0.7s
benzene            E= -232.018795Ha   gap=5.129eV   |μ|=0.000D   wall=  8.1s
propylene glycol   E= -269.351584Ha   gap=6.226eV   |μ|=2.000D   wall=  7.2s
caprylic acid (C8) E= -464.544963Ha   gap=5.257eV   |μ|=1.958D   wall=305.8s
capric acid (C10)  E= -543.084109Ha   gap=5.389eV   |μ|=1.510D   wall=523.1s
```

Dipoles match chemistry intuition: water and methanol close to literature
(~1.85 D and ~1.69 D respectively; PBE slightly over-estimates water as
expected); benzene's six-fold symmetry recovers |μ|=0 exactly; the polyol
and carboxylic acids carry meaningful dipoles.

## Molecule sources

The first three molecules use the curated `chimiaclaw-moladt::library`
(schematic geometry, hand-built). The C3/C8/C10 trio is resolved via the
RDKit ETKDGv3 + MMFF94 worker on duck — molecule IDs use RDKit-style
InChIKey-derived stems (`MOLADT.RDKIT.<inchikey>`).

| molecule          | molecule artifact          | request artifact           | result artifact            |
| ----------------- | -------------------------- | -------------------------- | -------------------------- |
| water             | `art_a1260505a4c2c867`     | `art_c297438e1d252604`     | `art_3d5c1283b1a8f79f`     |
| methanol          | `art_3270cc598fcb7c73`     | `art_8c77e87b34c6c65a`     | `art_563825a02d8ea8a3`     |
| benzene           | `art_fff6d384b83ab849`     | `art_30b545be1442d883`     | `art_87a648cd3b5f6490`     |
| propylene glycol  | `art_5e22a0afddbffb43`     | `art_9df13e5b1e1e4d68`     | `art_c1d9cf319fc537e2`     |
| caprylic acid     | `art_dcd858dfce5e043e`     | `art_81b8612b8cdf6693`     | `art_b4002fedd3e69f20`     |
| capric acid       | `art_f8aaf0f296548dd4`     | `art_357f3171b5e9bc2e`     | `art_5d1b8812735b2611`     |

## Orbital density cubes (`cubes/`)

Each result generates three Gaussian-style `.cube` files via
`pyscf.tools.cubegen` at the requested grid resolution (default 60, this
gallery used 50 to keep file sizes under 2 MB each):

- **HOMO** — highest-occupied molecular orbital, `cubegen.orbital(mol, ..., mo_coeff[:, homo])`
- **LUMO** — lowest-unoccupied molecular orbital
- **TOTAL_DENSITY** — total electron density `ρ(r)`, from `cubegen.density(mol, ..., make_rdm1())`

The signed `chem.dft.result.orbital_densities[]` block carries one entry per
cube with `{label, sha256, bytes, grid_resolution, local_path}` — the cube
bytes themselves are NOT inlined into the artifact JSON. Tampering with a
cube on disk invalidates the SHA-256 commitment in the signed artifact.

```
demo/dft/cubes/
├── MOLADT.WATER.001_HOMO_<sha256-prefix>.cube
├── MOLADT.WATER.001_LUMO_<sha256-prefix>.cube
├── MOLADT.WATER.001_TOTAL_DENSITY_<sha256-prefix>.cube
└── ... (×6 molecules = 18 cubes, ~28 MB total)
```

These are standard Gaussian cube files and can be opened in VMD, PyMOL,
Avogadro, ChimeraX, etc. for visualization.

## Verification

Every artifact JSON in this directory verifies independently:

```sh
cargo run -p chimiaclaw-cli -- artifact inspect \
  --store-dir <a fresh dir into which you've copied the JSONs>
```

The Rust signer refuses to sign any `chem.dft.result` whose
`convergence.converged` is `false` or whose `schema_tag` is wrong, so the
artifact graph cannot contain a fake SCF. The CLI also re-hashes every cube
locally and rejects any worker-reported SHA-256 mismatch before the result
is signed.

## Provenance posture

`provenance.source_kind` is one of:

- `pyscf-classical-functional` — real PySCF SCF with a stock functional
  (PBE, B3LYP, ...). All artifacts in this gallery are this kind.
- `pyscf-skala-1.1` — real Skala 1.1 deep-learned XC functional. Not yet
  shipped; awaits weights install on duck. Until then,
  `--backend pyscf-skala` falls back to PBE with an explicit fallback notice
  in `provenance.notes` (see DECISIONS D15).
- `stub-result` — `--stub` mode placeholder; never signed by the Rust
  adapter.

## Caveats

- HOMO/LUMO gaps from PBE are systematically too small (well-known DFT
  problem). Trust the chemistry pattern, not the absolute eV numbers.
- Schematic-curated geometries (water/methanol/benzene) are not relaxed; the
  RDKit-resolved geometries (propylene glycol, caprylic, capric) come from
  ETKDGv3 + MMFF94 and are reasonable starting points but not DFT-optimized.
  A production pipeline would run a geometry optimization first.
- `convergence.n_cycles` reports `0` because the modern PySCF API doesn't
  surface the cycle count on the attribute we read; the SCF really did
  converge (verified by `mf.converged = True`).
