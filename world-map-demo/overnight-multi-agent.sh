#!/usr/bin/env bash
# Overnight multi-agent loop: three ChimiaClaw agents cycle through
# ENS text-record publications (Sepolia) and 0G Storage uploads (Galileo).
#
# Required env:
#   ZEROG_PRIVATE_KEY      - Galileo testnet key with gas
#   ENS_WRITE_PRIVATE_KEY  - Sepolia controller key for chimiaclaw.eth
#   ENS_WRITE_RPC_URL      - Sepolia RPC (e.g. https://sepolia.infura.io/v3/...)
#   ENS_RPC_URL            - Sepolia read RPC (can equal write URL)
#
# Optional:
#   DELAY_SECONDS          - pause between operations (default: 60)
#   MAX_CYCLES             - how many full 3-agent cycles to run (default: 10)
#   OUT_DIR                - output directory (default: demo/overnight-out)
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DELAY="${DELAY_SECONDS:-60}"
MAX_CYCLES="${MAX_CYCLES:-10}"
OUT_DIR="${OUT_DIR:-${REPO_ROOT}/demo/overnight-out}"
LOG="${OUT_DIR}/overnight.log"

: "${ZEROG_PRIVATE_KEY:?must set ZEROG_PRIVATE_KEY}"
: "${ENS_WRITE_PRIVATE_KEY:?must set ENS_WRITE_PRIVATE_KEY}"
: "${ENS_WRITE_RPC_URL:?must set ENS_WRITE_RPC_URL}"
: "${ENS_RPC_URL:?must set ENS_RPC_URL}"

export CHIMIACLAW_ENS_PUBLISH_COMMAND="uv run --project ${REPO_ROOT}/skills/scienceclaw-port/workers/identity-ens ens-publish-text-records"
export ZEROG_UPLOAD_COMMAND="uv run --project ${REPO_ROOT}/skills/scienceclaw-port/workers/storage-0g zerog-upload"

ZEROG_RPC="https://evmrpc-testnet.0g.ai"
ZEROG_INDEXER="https://indexer-storage-testnet-turbo.0g.ai"

mkdir -p "${OUT_DIR}"

log() { echo "[$(date -u +%H:%M:%S)] $*" | tee -a "${LOG}"; }

log "=== Overnight multi-agent loop started ==="
log "Cycles: ${MAX_CYCLES}, Delay: ${DELAY}s"

# Build CLI once
log "Building chimiaclaw-cli with live-sponsors..."
cargo build --quiet -p chimiaclaw-cli --features live-sponsors --manifest-path "${REPO_ROOT}/Cargo.toml" 2>>"${LOG}"
CLI="${REPO_ROOT}/target/debug/chimiaclaw-cli"

# Collect files per agent
DFT_FILES=("${REPO_ROOT}"/demo/dft/cubes/*.cube)
RETRO_FILES=("${REPO_ROOT}"/demo/molecules/*.xyz)
LIT_FILES=("${REPO_ROOT}"/demo/molecules/*.svg)

dft_idx=0
retro_idx=0
lit_idx=0

zerog_upload_count=0
ens_publish_count=0
fail_count=0

upload_to_zerog() {
    local file="$1"
    local agent_label="$2"
    local basename
    basename="$(basename "${file}")"

    log "  [0G] ${agent_label} uploading ${basename}..."
    output=$(0g-storage-client upload \
        --url "${ZEROG_RPC}" \
        --key "${ZEROG_PRIVATE_KEY}" \
        --indexer "${ZEROG_INDEXER}" \
        --file "${file}" 2>&1)
    root=$(echo "${output}" | grep -oE '0x[0-9a-fA-F]{64}' | sort -u | head -1)
    if [ -n "${root}" ]; then
        zerog_upload_count=$((zerog_upload_count + 1))
        log "  [0G] OK root=${root}"
        echo "{\"agent\":\"${agent_label}\",\"file\":\"${basename}\",\"root\":\"${root}\",\"ts\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" \
            > "${OUT_DIR}/${agent_label}_${basename}.receipt.json"
    else
        fail_count=$((fail_count + 1))
        log "  [0G] FAIL"
    fi
}

publish_ens() {
    local agent="$1"
    local capabilities="$2"
    local head_cid="$3"

    log "  [ENS] ${agent} publishing to chimiaclaw.eth..."
    "${CLI}" live ens-publish \
        --agent "${agent}" \
        --ens "chimiaclaw.eth" \
        --record "chimiaclaw.profile.cid=zg://profile/${agent}" \
        --record "chimiaclaw.capabilities=${capabilities}" \
        --record "chimiaclaw.settlement.endpoint=artifact-ledger:simulated-release" \
        --record "chimiaclaw.axl.peer_id=axl-${agent%%.service.*}-peer" \
        --record "chimiaclaw.head_artifact.cid=${head_cid}" \
        --out-dir "${OUT_DIR}" \
        >>"${LOG}" 2>&1

    if [ $? -eq 0 ]; then
        ens_publish_count=$((ens_publish_count + 1))
        log "  [ENS] OK"
    else
        fail_count=$((fail_count + 1))
        log "  [ENS] FAIL (check log)"
    fi
}

for cycle in $(seq 1 "${MAX_CYCLES}"); do
    log ""
    log "--- Cycle ${cycle}/${MAX_CYCLES} ---"

    # === Agent 1: DFT ===
    log "Agent: dft.service.chimiaclaw.eth"
    if [ ${dft_idx} -lt ${#DFT_FILES[@]} ]; then
        upload_to_zerog "${DFT_FILES[$dft_idx]}" "dft"
        dft_idx=$((dft_idx + 1))
    fi
    publish_ens "dft.service.chimiaclaw.eth" \
        "cap.dft.single_point,cap.dft.geometry_opt" \
        "zg://dft-head/cycle-${cycle}"
    sleep "${DELAY}"

    # === Agent 2: Retrosynthesis ===
    log "Agent: retro.service.chimiaclaw.eth"
    if [ ${retro_idx} -lt ${#RETRO_FILES[@]} ]; then
        upload_to_zerog "${RETRO_FILES[$retro_idx]}" "retro"
        retro_idx=$((retro_idx + 1))
    fi
    publish_ens "retro.service.chimiaclaw.eth" \
        "cap.retrosynth.template_relevance,cap.retrosynth.route_quote" \
        "zg://retro-head/cycle-${cycle}"
    sleep "${DELAY}"

    # === Agent 3: Literature ===
    log "Agent: literature.service.chimiaclaw.eth"
    if [ ${lit_idx} -lt ${#LIT_FILES[@]} ]; then
        upload_to_zerog "${LIT_FILES[$lit_idx]}" "literature"
        lit_idx=$((lit_idx + 1))
    fi
    publish_ens "literature.service.chimiaclaw.eth" \
        "cap.literature.synthesis,cap.literature.claim_extraction" \
        "zg://lit-head/cycle-${cycle}"
    sleep "${DELAY}"
done

log ""
log "=== Overnight complete ==="
log "0G uploads: ${zerog_upload_count}"
log "ENS publications: ${ens_publish_count}"
log "Failures: ${fail_count}"
log "Receipts: ${OUT_DIR}"
