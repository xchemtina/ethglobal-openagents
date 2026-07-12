# ChimiaClaw summary

## Current submission surface

ChimiaClaw is being presented as a focused three-agent scientific pipeline, not a broad speculative lab swarm. The dashboard at `demo/world-map.html` prefers a locally generated `demo/world-model.live.json` when present, falls back to `demo/world-model.json`, and shows only the current submission story: Literature extracts cited reaction candidates, Retrosynthesis turns curated molecules into signed route proposals, and DFT runs real PySCF calculations with signed result artifacts and orbital-cube evidence.

## What is real now

- The Rust artifact substrate signs payload-bound DAG nodes with Ed25519 signatures and Blake3 content hashes.
- `apps/retroquoter` and `RouteQuoteSkill` produce deterministic signed retrosynthesis route and procurement quote artifacts.
- The PySCF DFT path produces signed `chem.dft.result` artifacts parented to `chem.dft.request` and `chem.molecule.adt`, with cube files committed by SHA-256 rather than inlined where requested. Six HOMO/LUMO cube pairs are sampled into a central WebGPU orbital gallery with visible bond skeletons, and a separate overnight five-molecule germanium run adds scalar energy/gap/dipole evidence without cubes.
- The restored dashboard model contains three agent lanes, three handoffs, three agent-run cards, a focused evidence set including the overnight scalar DFT card, and one central six-molecule WebGPU HOMO/LUMO orbital gallery.
- ENS root identity for `chimiaclaw.eth` is live on Sepolia and verified through signed `identity.ens.publication`, `identity.ens.resolution`, and `identity.ens.verification` artifacts.
- One 0G Galileo Turbo anchor proves the storage path for a ferrocene MolADT source artifact.
- `demo/live-dashboard-watch.py` scans `demo/overnight-full-out/` and writes an auto-refreshing local projection with counts and artifact IDs for full-pipeline DFT, Uniswap quote-only settlement, 0G anchors, and ENS artifacts.
- The dashboard verifier can recursively find nested artifacts and validates both the focused three-agent fixture and live projection against real artifact evidence.

## What is intentionally not claimed

- The Literature lane is the next signed run, not a completed extraction claim. It should emit `science.literature.synthesis` with citations, extracted reaction equations, confidence, and candidate molecules before downstream agents consume it.
- Per-agent ENS capability records for Literature, Retro, and DFT are not yet published; only the root ENS identity is live and verified.
- The 0G anchor proves one upload path, not that every literature artifact, route artifact, or cube file is already stored on 0G.
- KeeperHub, AXL, Uniswap, MSSP, World Avatar, autonomous wetlab custody, and DAO execution remain future or auxiliary surfaces unless a signed artifact proves a specific run.
- The broad lab-swarm dashboard was preserved as a backup, but it is not the primary submission capture surface.

## How to validate the current surface

```sh
python3 -m py_compile demo/live-dashboard-watch.py
python3 demo/live-dashboard-watch.py --once
python3 -m json.tool demo/world-model.json >/tmp/chimiaclaw-world-model.json
python3 -m json.tool demo/world-model.live.json >/tmp/chimiaclaw-world-model.live.json
python3 - <<'PY2'
from pathlib import Path
html = Path('demo/world-map.html').read_text()
start = html.index('<script>') + len('<script>')
end = html.index('</script>', start)
Path('/tmp/chimiaclaw-world-map-script.js').write_text(html[start:end])
PY2
node --check /tmp/chimiaclaw-world-map-script.js
cargo run -p chimiaclaw-cli -- world-model verify --world-model demo/world-model.json --artifact-dir demo
cargo run -p chimiaclaw-cli -- world-model verify --world-model demo/world-model.live.json --artifact-dir demo
```

## Immediate next build

1. Run the first citation-bound Literature extraction and sign `science.literature.synthesis`.
2. Feed extracted/curated reaction candidates into RetroQuoter so `chem.retrosynth.route_proposal` parents the literature artifact.
3. Send only molecules without signed results to DFT, then sign new `chem.dft.result` outputs; request and hash cube outputs only when orbital visualization or storage evidence is needed.
4. Publish per-agent ENS capability records or subnames after the operator confirms the final text records.
5. Upload selected literature, route, result, cube, and derived orbital-gallery artifacts to 0G so the evidence panel can show more than one anchor.
6. Record final dashboard footage from the three-agent surface, including the six-molecule WebGPU orbital gallery and, if available, the live source pill plus overnight pipeline cards, not the broad lab-swarm backup.
