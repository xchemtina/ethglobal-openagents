#!/usr/bin/env bash
# ENS publish -> resolve -> verify round-trip for ChimiaClaw.
#
# Required env (operator-supplied):
#   ENS_WRITE_RPC_URL     - testnet RPC with write capability (Sepolia/Holesky)
#   ENS_WRITE_PRIVATE_KEY - controller private key (NEVER pass on argv)
#   ENS_RPC_URL           - read-only RPC the resolver uses (can equal write URL)
#   CHIMIACLAW_AGENT      - agent id (e.g. dft.service.chimiadao.eth)
#   CHIMIACLAW_ENS        - ENS name to publish on
#
# Optional:
#   OUT_DIR               - where to drop the three signed artifacts
set -uo pipefail

: "${CHIMIACLAW_AGENT:?must set CHIMIACLAW_AGENT (e.g. dft.service.chimiadao.eth)}"
: "${CHIMIACLAW_ENS:?must set CHIMIACLAW_ENS (e.g. dft.service.chimiadao.eth)}"
: "${ENS_WRITE_RPC_URL:?must set ENS_WRITE_RPC_URL}"
: "${ENS_WRITE_PRIVATE_KEY:?must set ENS_WRITE_PRIVATE_KEY (env, not argv)}"
: "${ENS_RPC_URL:?must set ENS_RPC_URL for the read-back step}"

OUT_DIR="${OUT_DIR:-demo/ens-out}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

export CHIMIACLAW_ENS_PUBLISH_COMMAND="uv run --project ${REPO_ROOT}/skills/scienceclaw-port/workers/identity-ens ens-publish-text-records"

mkdir -p "${OUT_DIR}"

cargo run --quiet -p chimiaclaw-cli --features live-sponsors -- \
  live ens-publish \
  --agent "${CHIMIACLAW_AGENT}" \
  --ens "${CHIMIACLAW_ENS}" \
  --record "chimiaclaw.profile.cid=${CHIMIACLAW_PROFILE_CID:-zg://demo-profile-root}" \
  --record "chimiaclaw.capabilities=${CHIMIACLAW_CAPABILITIES:-cap.dft.single_point}" \
  --record "chimiaclaw.settlement.endpoint=${CHIMIACLAW_SETTLEMENT_ENDPOINT:-artifact-ledger:simulated-release}" \
  --record "chimiaclaw.axl.peer_id=${CHIMIACLAW_AXL_PEER_ID:-axl-dft-demo-peer}" \
  --record "chimiaclaw.head_artifact.cid=${CHIMIACLAW_HEAD_CID:-zg://demo-head-root}" \
  --out-dir "${OUT_DIR}"

echo
echo "Three signed artifacts (publication / resolution / verification) written to ${OUT_DIR}."
echo "Inspect them with:"
echo "  ls -1 ${OUT_DIR}"
