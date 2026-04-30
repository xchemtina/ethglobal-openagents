# chimiaclaw-askcos-retro
A uv-managed worker that calls a user-managed ASKCOS template-relevance
endpoint and emits a `chem.retrosynth.template_suggestions` JSON document
ready to be signed by ChimiaClaw.
## Contract
- Reads a target SMILES from stdin (or `--smiles <smiles>`); UTF-8.
- Reads the endpoint URL from `--endpoint <url>` or the
  `CHIMIACLAW_ASKCOS_ENDPOINT` environment variable; refuses to invoke a live
  endpoint without explicit configuration.
- POSTs to `<endpoint>/api/v2/template-relevance/` (the upstream ASKCOS
  TorchServe contract) with template sets `reaxys`, `pistachio`,
  `pistachio_ringbreaker`, `bkms_metabolic`, `reaxys_biocatalysis` unless
  overridden by `--template-set`.
- Emits a JSON document with one `proposals[]` entry per template set,
  each carrying ranked precursor SMILES with score/template metadata.
- Sets `provenance.source_kind = "askcos-template-relevance"` and records the
  endpoint, the request seed, and the template-set list in `provenance.notes`.
## Wiring
```sh
export CHIMIACLAW_ASKCOS_ENDPOINT="http://duck.olympus.local:9410"
uvx --from skills/scienceclaw-port/workers/retrosynth askcos-retro --smiles "O=Cc1ccccc1"
```
## Disk cache
The worker carries a content-hashed disk cache so repeat invocations against
the same `(endpoint, target_smiles, sorted_template_sets, top_k)` never
retouch the network. The cache key is a 16-byte Blake2b digest; entries are
sharded into `<key[0:2]>/<key>.json` files under the configured cache dir.
- Default cache dir: `~/.cache/chimiaclaw/askcos`.
- Override with `--cache-dir <path>` or `CHIMIACLAW_ASKCOS_CACHE_DIR`.
- `--no-cache` skips both the read and the write for one invocation.
- `--cache-only` refuses to call the endpoint and exits non-zero on a miss
  (useful for offline / sponsor-down replay).
Every emitted JSON document now carries a top-level `cache` field of shape
`{ hit: bool, key: str, path: str }`; the Rust adapter mirrors this as
`AskcosCacheRecord` and signs it as part of the
`chem.retrosynth.template_suggestions` artifact.
Verified end-to-end on a stub ASKCOS endpoint:
- first call → `cache.hit = false`, two HTTP calls to the endpoint, two
  proposals returned;
- second call with identical args → `cache.hit = true`, identical proposals,
  zero additional HTTP calls;
- `--cache-only` against a fresh SMILES → exit code 2 with the missing-cache
  path on stderr.
## Status
Per repo policy this worker must run under uv/uvx; it is **not** a Docker
container. The previous ScienceClaw scraper fallback is intentionally not
ported because it fabricates demo-like routes that should never enter the
signed ChimiaClaw retrosynthesis graph.
