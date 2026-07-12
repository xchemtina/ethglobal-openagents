#!/usr/bin/env bash
# ENS publish -> resolve -> verify round-trip for the Literature lane subname.
#
# Phase 1 contract: `literature.chimiaclaw.eth` carries four text records
# that prove the lane is live and identify which extraction harness produced
# the most recent signed `science.literature.synthesis` artifact.
#
# Required env (operator-supplied):
#   ENS_WRITE_RPC_URL          - testnet RPC with write capability (Sepolia)
#   ENS_WRITE_PRIVATE_KEY      - controller private key (NEVER pass on argv)
#   ENS_RPC_URL                - read-only RPC for the resolver step
#
# Optional env (defaults baked in for the Phase-1 carbenoid pipeline):
#   CHIMIACLAW_LITERATURE_AGENT       (default: literature.service.chimiaclaw.eth)
#   CHIMIACLAW_LITERATURE_ENS         (default: literature.chimiaclaw.eth)
#   CHIMIACLAW_LITERATURE_PROFILE_CID (default: zg://demo-profile-root/literature)
#   CHIMIACLAW_LITERATURE_HEAD_CID    (default: zg://demo-head-root/literature)
#   CHIMIACLAW_LITERATURE_RUNTIME     (default: mlx-local:gemma-4-e4b-it-4bit)
#   CHIMIACLAW_LITERATURE_SKILL       (default: science.literature.synthesis.v1)
#   OUT_DIR                           (default: demo/ens-literature-out)
set -uo pipefail

: "${ENS_WRITE_RPC_URL:?must set ENS_WRITE_RPC_URL}"
: "${ENS_WRITE_PRIVATE_KEY:?must set ENS_WRITE_PRIVATE_KEY (env, not argv)}"
: "${ENS_RPC_URL:?must set ENS_RPC_URL for the read-back step}"

CHIMIACLAW_LITERATURE_AGENT="${CHIMIACLAW_LITERATURE_AGENT:-literature.service.chimiaclaw.eth}"
CHIMIACLAW_LITERATURE_ENS="${CHIMIACLAW_LITERATURE_ENS:-literature.chimiaclaw.eth}"
CHIMIACLAW_LITERATURE_PROFILE_CID="${CHIMIACLAW_LITERATURE_PROFILE_CID:-zg://demo-profile-root/literature}"
CHIMIACLAW_LITERATURE_HEAD_CID="${CHIMIACLAW_LITERATURE_HEAD_CID:-zg://demo-head-root/literature}"
CHIMIACLAW_LITERATURE_RUNTIME="${CHIMIACLAW_LITERATURE_RUNTIME:-mlx-local:gemma-4-e4b-it-4bit}"
CHIMIACLAW_LITERATURE_SKILL="${CHIMIACLAW_LITERATURE_SKILL:-science.literature.synthesis.v1}"

OUT_DIR="${OUT_DIR:-demo/ens-literature-out}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

export CHIMIACLAW_ENS_PUBLISH_COMMAND="uv run --project ${REPO_ROOT}/skills/scienceclaw-port/workers/identity-ens ens-publish-text-records"

mkdir -p "${OUT_DIR}"

cargo run --quiet -p chimiaclaw-cli --features live-sponsors -- \
  live ens-publish \
  --agent  "${CHIMIACLAW_LITERATURE_AGENT}" \
  --ens    "${CHIMIACLAW_LITERATURE_ENS}" \
  --record "chimiaclaw.profile.cid=${CHIMIACLAW_LITERATURE_PROFILE_CID}" \
  --record "chimiaclaw.head_artifact.cid=${CHIMIACLAW_LITERATURE_HEAD_CID}" \
  --record "chimiaclaw.skill=${CHIMIACLAW_LITERATURE_SKILL}" \
  --record "chimiaclaw.runtime=${CHIMIACLAW_LITERATURE_RUNTIME}" \
  --out-dir "${OUT_DIR}"

echo
echo "Literature lane ENS roundtrip artifacts written to ${OUT_DIR}."
echo "Required text records on ${CHIMIACLAW_LITERATURE_ENS}:"
echo "  chimiaclaw.profile.cid       = ${CHIMIACLAW_LITERATURE_PROFILE_CID}"
echo "  chimiaclaw.head_artifact.cid = ${CHIMIACLAW_LITERATURE_HEAD_CID}"
echo "  chimiaclaw.skill             = ${CHIMIACLAW_LITERATURE_SKILL}"
echo "  chimiaclaw.runtime           = ${CHIMIACLAW_LITERATURE_RUNTIME}"
