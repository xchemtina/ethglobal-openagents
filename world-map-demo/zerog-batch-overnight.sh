#!/usr/bin/env bash
# Overnight batch upload to 0G Galileo Storage.
#
# Uploads every molecule XYZ and DFT cube file from the demo gallery,
# logging each root hash and transaction. Designed to run unattended.
#
# Required env:
#   ZEROG_PRIVATE_KEY  - funded Galileo testnet key (env, not argv)
#   PATH must include 0g-storage-client and cargo
#
# Optional:
#   DELAY_SECONDS      - pause between uploads (default: 30)
#   OUT_DIR            - where to write receipts (default: demo/zerog-batch-out)
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DELAY="${DELAY_SECONDS:-30}"
OUT_DIR="${OUT_DIR:-${REPO_ROOT}/demo/zerog-batch-out}"
LOG="${OUT_DIR}/batch.log"
RPC_URL="https://evmrpc-testnet.0g.ai"
INDEXER_URL="https://indexer-storage-testnet-turbo.0g.ai"

: "${ZEROG_PRIVATE_KEY:?must set ZEROG_PRIVATE_KEY}"

mkdir -p "${OUT_DIR}"

echo "=== 0G batch upload started at $(date -u) ===" | tee -a "${LOG}"
echo "Delay between uploads: ${DELAY}s" | tee -a "${LOG}"

upload_count=0
fail_count=0

upload_file() {
    local file="$1"
    local label="$2"
    local basename
    basename="$(basename "${file}")"

    echo "[$(date -u)] Uploading ${label}: ${basename}..." | tee -a "${LOG}"

    output=$(0g-storage-client upload \
        --url "${RPC_URL}" \
        --key "${ZEROG_PRIVATE_KEY}" \
        --indexer "${INDEXER_URL}" \
        --file "${file}" 2>&1)
    exit_code=$?

    # Extract root hash from output
    root=$(echo "${output}" | grep -oE '0x[0-9a-fA-F]{64}' | head -1)

    if [ ${exit_code} -eq 0 ] && [ -n "${root}" ]; then
        upload_count=$((upload_count + 1))
        echo "  OK  root=${root}" | tee -a "${LOG}"
        # Save receipt
        echo "{\"file\":\"${basename}\",\"label\":\"${label}\",\"root\":\"${root}\",\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" > "${OUT_DIR}/${basename}.receipt.json"
    else
        fail_count=$((fail_count + 1))
        echo "  FAIL exit=${exit_code}" | tee -a "${LOG}"
        echo "${output}" >> "${LOG}"
    fi
}

# Phase 1: Molecule XYZ gallery (18 files, ~200-900 bytes each)
echo "" | tee -a "${LOG}"
echo "--- Phase 1: Molecule XYZ gallery ---" | tee -a "${LOG}"
for xyz in "${REPO_ROOT}"/demo/molecules/*.xyz; do
    upload_file "${xyz}" "molecule-xyz"
    sleep "${DELAY}"
done

# Phase 2: DFT orbital density cubes (18 files, larger)
echo "" | tee -a "${LOG}"
echo "--- Phase 2: DFT orbital cubes ---" | tee -a "${LOG}"
for cube in "${REPO_ROOT}"/demo/dft/cubes/*.cube; do
    upload_file "${cube}" "dft-cube"
    sleep "${DELAY}"
done

echo "" | tee -a "${LOG}"
echo "=== Batch complete at $(date -u) ===" | tee -a "${LOG}"
echo "Uploaded: ${upload_count}  Failed: ${fail_count}" | tee -a "${LOG}"
echo "Receipts in: ${OUT_DIR}" | tee -a "${LOG}"
