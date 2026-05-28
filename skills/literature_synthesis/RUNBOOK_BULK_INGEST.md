# Bulk-ingest runbook
Operator runbook for parsing the full ~1004-paper local corpus through Docling.

## Measured throughput (mid-2026)
- Apple Silicon, 2 workers
- ~10 s/page average across organogermanium / main-group chemistry papers
- 5-paper smoke run: 5:02 wall-clock, 4 OK + 1 corrupt-PDF failure (handled cleanly)

Extrapolated:
- 4 workers, ~10 average pages/paper, 1004 papers → roughly 7 hours wall-clock
- 2 workers → roughly 14 hours

## One-shot full run
```bash path=null start=null
cd ~/OpenAgents/skills/literature_synthesis
nohup .venv/bin/literature-synthesis bulk-ingest \
  --root ~/Documents/Thesis_DPhil/OverallPapers \
  --root ~/Documents/ChimiaDAO-MNT/literature \
  --root ~/Documents/ChimiaDAO-Papers \
  --root ~/Documents/Papers \
  --root ~/Documents/ChimiaDAO-Vicar/Miglyol840/literature/pdfs \
  --root ~/Documents/ChimiaDAO-Vicar/ProductSheets \
  --cache-dir ~/.chimia_kb/docling \
  --manifest ~/.chimia_kb/manifest.json \
  --workers 4 \
  > ~/.chimia_kb/bulk-ingest.log 2>&1 &
```

## Monitoring
```bash path=null start=null
# follow progress
tail -f ~/.chimia_kb/bulk-ingest.log

# count what's in the cache so far
find ~/.chimia_kb/docling -name 'document.md' | wc -l

# list per-paper failures (only present after run finishes; for live failures, look at error.json under the cache)
jq '.failures[] | {primary_path, error}' ~/.chimia_kb/manifest.json | head -20
find ~/.chimia_kb/docling -name 'error.json' -exec jq '{path: .primary_path, error}' {} +
```

## Resume after a crash / Ctrl-C
The cache is keyed by Blake3 content hash. Re-running the exact same command
skips every paper whose `document.md` + `meta.json` already exist. Failed
papers are *not* retried by default (an `error.json` sidecar marks them).
Force a retry of a single paper:

```bash path=null start=null
rm -rf ~/.chimia_kb/docling/<first-two-hex>/<full-hash>/
```

Or force-retry every failed paper with `--force` (also reparses successes —
expensive). Better: write a small loop that deletes only `error.json`-tagged
dirs and reruns.

## Validation against the OpenAgents Rust verifier
Once the run finishes, the per-paper signed `science.literature.synthesis`
extraction step (next phase) consumes `~/.chimia_kb/manifest.json` plus the
per-paper `~/.chimia_kb/docling/<hash>/document.md` files. The Rust crate
checks every claim's `evidence_span` is a literal substring of that markdown.

## Sanity numbers to expect
- ~1004 unique papers (1202 source paths after symlink/copy dedup)
- Cache size: ~3-6 GB (roughly 1.5-2× source PDFs because of rich JSON +
  markdown + figures metadata).
- Failure rate observed: ~1 in 5 on small samples, mostly corrupt or
  encrypted PDFs. Expect ~5-15% on the full corpus; capture them via
  `failures.json` and decide per-paper whether to chase Mistral OCR / Marker.
