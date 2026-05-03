# Curated MolADT renders
This directory holds deterministic XYZ + SVG renders of every entry in
`chimiaclaw_moladt::library`, produced by the pure-Rust renderer (no external
chemistry stack). Regenerate any time with:
```sh
for mol in water ammonia methanol ethanol acetic-acid benzene toluene \
           bromobenzene phenylboronic-acid biphenyl ferrocene; do
  cargo run -p chimiaclaw-cli -- moladt-render \
    --library "$mol" \
    --xyz "demo/molecules/${mol}.xyz" \
    --svg "demo/molecules/${mol}.svg"
done
```
| Molecule           | Formula  | XYZ                                | SVG                                |
| ------------------ | -------- | ---------------------------------- | ---------------------------------- |
| water              | H2O      | [water.xyz](water.xyz)             | ![water](water.svg)                |
| ammonia            | NH3      | [ammonia.xyz](ammonia.xyz)         | ![ammonia](ammonia.svg)            |
| methanol           | CH4O     | [methanol.xyz](methanol.xyz)       | ![methanol](methanol.svg)          |
| ethanol            | C2H6O    | [ethanol.xyz](ethanol.xyz)         | ![ethanol](ethanol.svg)            |
| acetic acid        | C2H4O2   | [acetic-acid.xyz](acetic-acid.xyz) | ![acetic acid](acetic-acid.svg)    |
| benzene            | C6H6     | [benzene.xyz](benzene.xyz)         | ![benzene](benzene.svg)            |
| toluene            | C7H8     | [toluene.xyz](toluene.xyz)         | ![toluene](toluene.svg)            |
| bromobenzene       | C6H5Br   | [bromobenzene.xyz](bromobenzene.xyz) | ![bromobenzene](bromobenzene.svg) |
| phenylboronic acid | C6H7BO2  | [phenylboronic-acid.xyz](phenylboronic-acid.xyz) | ![phenylboronic acid](phenylboronic-acid.svg) |
| biphenyl           | C12H10   | [biphenyl.xyz](biphenyl.xyz)       | ![biphenyl](biphenyl.svg)          |
| ferrocene          | C10H10Fe | [ferrocene.xyz](ferrocene.xyz)     | ![ferrocene](ferrocene.svg)        |
## RDKit-tier renders
These came from the uv-managed `rdkit-smiles-to-moladt` worker through the
`chimiaclaw_moladt::worker::resolve_with_worker` boundary; the `provenance.source_kind`
field in each MolADT artifact is `rdkit-etkdgv3-mmff94` rather than
`schematic-curated`, so a downstream DFT worker can tell at a glance which tier
produced the geometry.
| Molecule           | Formula  | XYZ                                              | SVG                                              |
| ------------------ | -------- | ------------------------------------------------ | ------------------------------------------------ |
| benzaldehyde       | C7H6O    | [benzaldehyde.xyz](benzaldehyde.xyz)             | ![benzaldehyde](benzaldehyde.svg)               |
| aspirin            | C9H8O4   | [aspirin.xyz](aspirin.xyz)                       | ![aspirin](aspirin.svg)                         |
| salicylic acid     | C7H6O3   | [salicylic-acid.xyz](salicylic-acid.xyz)         | ![salicylic acid](salicylic-acid.svg)           |
| pyridine           | C5H5N    | [pyridine.xyz](pyridine.xyz)                     | ![pyridine](pyridine.svg)                       |
| methylamine        | CH5N     | [methylamine.xyz](methylamine.xyz)               | ![methylamine](methylamine.svg)                 |
| imidazole          | C3H4N2   | [imidazole.xyz](imidazole.xyz)                   | ![imidazole](imidazole.svg)                     |
| acetone            | C3H6O    | [acetone.xyz](acetone.xyz)                       | ![acetone](acetone.svg)                         |
Reproduce with:
```sh
export CHIMIACLAW_SMILES_TO_MOLADT_COMMAND="uv run --project skills/scienceclaw-port/workers/cheminformatics rdkit-smiles-to-moladt"
cargo run -p chimiaclaw-cli -- moladt-render --smiles 'O=Cc1ccccc1' --xyz demo/molecules/benzaldehyde.xyz --svg demo/molecules/benzaldehyde.svg
```
Aromatic ring members are drawn with a dashed outline (`stroke-dasharray="3,2"`).
The footer of every SVG records the molecule id, formula, and
`provenance.source_kind`, so visual inspection cannot accidentally hide which
geometry tier produced the render (curated schematic, geometry guess, or
RDKit-MMFF).
