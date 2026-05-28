# literature-synthesis
ChimiaClaw Literature lane worker.

Open-access ingestion + deterministic citation-grounded extraction → JSON
consumable by the Rust `chimiaclaw-literature` constructors.

## Subcommands
- `literature-synthesis ingest --query Q --sector S --max-papers N --out DIR`
- `literature-synthesis extract --manifest manifest.json --excerpts excerpts.json --out synthesis.json`
- `literature-synthesis show --synthesis synthesis.json`
- `literature-synthesis run-fixture --fixture fixtures/sample_synthesis.json --out synthesis.json`

## Runtimes
Selected by `CHIMIACLAW_LITERATURE_RUNTIME` (default `mlx-local`):
- `mlx-local` — MLX with `~/mlx-models/gemma-4-e4b-it-4bit`
- `local-ollama` — local Ollama HTTP API
- `openai` — OpenAI Chat Completions
- `openrouter` — OpenRouter `/v1/chat/completions`
- `offline` — read raw model output from a fixture file

## Install
```
uv venv
uv pip install -e .[full]
```
The optional `ingest` extra path-depends on `~/Documents/ChimiaDAO-PaperIngestion`.
