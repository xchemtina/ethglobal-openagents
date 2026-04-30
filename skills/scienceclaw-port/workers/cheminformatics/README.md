# chimiaclaw-rdkit-smiles-to-moladt
A uv-managed RDKit worker that turns a SMILES string into a `MoleculeAdt` JSON
document compatible with the `chimiaclaw-moladt` Rust crate.
## Contract
- Reads a single SMILES from stdin (or `--smiles <smiles>`); UTF-8.
- On success, writes a JSON document to stdout that deserializes into
  `chimiaclaw_moladt::MoleculeAdt`.
- On failure, writes a human-readable message to stderr and exits non-zero.
- Sets `provenance.source_kind` to `"rdkit-etkdgv3-mmff94"` (or
  `"rdkit-etkdgv3-uff"` when MMFF cannot parameterize the molecule).
- Adds explicit notes recording the conformer-generation seed, the optimizer
  used, and the final convergence flag so a downstream DFT worker can decide
  whether to re-optimize.
## Wiring
The Rust crate `chimiaclaw-moladt` looks for the
`CHIMIACLAW_SMILES_TO_MOLADT_COMMAND` environment variable. Point it at this
worker via `uvx`:
```sh
export CHIMIACLAW_SMILES_TO_MOLADT_COMMAND="uvx --from skills/scienceclaw-port/workers/cheminformatics rdkit-smiles-to-moladt"
cargo run -p chimiaclaw-cli -- moladt-render --smiles 'O=Cc1ccccc1' --xyz /tmp/benzaldehyde.xyz --svg /tmp/benzaldehyde.svg
```
## Status
This worker is the first ChimiaClaw uv-managed Python boundary; per repo
policy it must run under `uv` / `uvx`, not Docker or Homebrew.
The implementation favours **fresh re-implementation** against the upstream
ScienceClaw `rdkit` skill's documented behaviour (see
`skills/scienceclaw-port/porting_manifest.toml`), not literal vendoring.
