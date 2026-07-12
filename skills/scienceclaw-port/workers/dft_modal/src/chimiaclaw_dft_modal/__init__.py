"""Modal-backed ChimiaClaw DFT worker.

Same JSON contract as ``skills/scienceclaw-port/workers/dft``:

- stdin: ``{request, molecule_adt, cube_grid?}`` or flat request
- stdout: ``chem.dft.result`` JSON
- non-zero exit + stderr on failure / guard rejection

Signing still happens in Rust (``chimiaclaw-dft-skala``). This package never
holds DAO keys.
"""

__version__ = "0.1.0"
