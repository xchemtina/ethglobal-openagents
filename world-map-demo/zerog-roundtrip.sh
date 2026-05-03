#!/usr/bin/env bash
# 0G upload round-trip for ChimiaClaw.
#
# Required env (operator-supplied for real uploads):
#   ZEROG_PRIVATE_KEY  - testnet key with gas on Galileo (env, not argv)
#
# Optional:
#   ZEROG_STUB=1       - skip the real network call and emit a deterministic
#                        Blake2b-rooted stub receipt. Useful for CI and demos
#                        that need a signed storage.zerog.upload artifact
#                        without spending testnet funds.
#   ZEROG_BINARY       - path to 0g-storage-client (defaults to PATH lookup)
#   ZEROG_RPC_URL, ZEROG_INDEXER_URL - override the defaults
#   OUT_DIR            - where to drop signed artifacts (default: demo/zerog-out)
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${OUT_DIR:-${REPO_ROOT}/demo/zerog-out}"
mkdir -p "${OUT_DIR}"

export ZEROG_UPLOAD_COMMAND="uv run --project ${REPO_ROOT}/skills/scienceclaw-port/workers/storage-0g zerog-upload"
export ZEROG_PRIVATE_KEY="${ZEROG_PRIVATE_KEY:-0xnot-a-real-key-stub-mode-only}"

# Build a deterministic source artifact (the demo ferrocene MolADT) and write
# its XYZ projection out so the wrapper has a real file to upload.
"${REPO_ROOT}"/target/debug/chimiaclaw-cli moladt-dft-demo > "${OUT_DIR}/dft-demo.json" 2>/dev/null || \
  (cd "${REPO_ROOT}" && cargo run --quiet -p chimiaclaw-cli -- moladt-dft-demo > "${OUT_DIR}/dft-demo.json")
python3 -c "
import json, sys
d = json.load(open('${OUT_DIR}/dft-demo.json'))
open('${OUT_DIR}/source-artifact.json', 'w').write(json.dumps(d['molecule_artifact']))
open('${OUT_DIR}/source-payload.xyz', 'w').write(d['projections']['xyz'])
print('source artifact id:', d['molecule_artifact']['id'])
"

cd "${REPO_ROOT}"
cargo run --quiet -p chimiaclaw-cli --features live-sponsors -- \
  live zerog-anchor \
  --source-artifact-json "${OUT_DIR}/source-artifact.json" \
  --payload-file "${OUT_DIR}/source-payload.xyz" \
  --agent storage.zerog.operator.chimiaclaw.eth \
  > "${OUT_DIR}/anchor.json"

echo
echo "Signed storage.zerog.upload artifact written to ${OUT_DIR}/anchor.json"
python3 -c "
import json
d = json.load(open('${OUT_DIR}/anchor.json'))
print('  id          :', d['id'])
print('  schema_tags :', list(d['schema_tags']))
print('  output_cid  :', d['output_cid'])
print('  parents     :', d['parent_artifact_ids'])
"
