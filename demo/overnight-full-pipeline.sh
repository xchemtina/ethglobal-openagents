#!/usr/bin/env bash
# Full pipeline: propylene glycol diesters (smallest first) + Ge molecules
# DFT → Uniswap settlement → 0G anchor → ENS publish
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DELAY="${DELAY_SECONDS:-10}"
OUT_DIR="${OUT_DIR:-${REPO_ROOT}/demo/overnight-full-out}"
LOG="${OUT_DIR}/overnight-full.log"
LIVE_MODEL="${LIVE_MODEL:-${REPO_ROOT}/demo/world-model.live.json}"
LIVE_DASHBOARD_WATCHER="${LIVE_DASHBOARD_WATCHER:-${REPO_ROOT}/demo/live-dashboard-watch.py}"

: "${UNISWAP_API_KEY:?must set UNISWAP_API_KEY}"
: "${ENS_RPC_URL:?must set ENS_RPC_URL}"
: "${ENS_WRITE_RPC_URL:?must set ENS_WRITE_RPC_URL}"
: "${ENS_WRITE_PRIVATE_KEY:?must set ENS_WRITE_PRIVATE_KEY}"
: "${ZEROG_PRIVATE_KEY:?must set ZEROG_PRIVATE_KEY}"
: "${CHIMIACLAW_SMILES_TO_MOLADT_COMMAND:?must set CHIMIACLAW_SMILES_TO_MOLADT_COMMAND}"
: "${CHIMIACLAW_DFT_COMMAND:?must set CHIMIACLAW_DFT_COMMAND}"

export CHIMIACLAW_ENS_PUBLISH_COMMAND="uv run --project ${REPO_ROOT}/skills/scienceclaw-port/workers/identity-ens ens-publish-text-records"
export ZEROG_UPLOAD_COMMAND="uv run --project ${REPO_ROOT}/skills/scienceclaw-port/workers/storage-0g zerog-upload"

mkdir -p "${OUT_DIR}/molecules" "${OUT_DIR}/dft" "${OUT_DIR}/uniswap" "${OUT_DIR}/zerog" "${OUT_DIR}/ens"

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*" | tee -a "${LOG}"; }

refresh_dashboard() {
    if [ -f "${LIVE_DASHBOARD_WATCHER}" ]; then
        python3 "${LIVE_DASHBOARD_WATCHER}" \
            --base-world-model "${REPO_ROOT}/demo/world-model.json" \
            --pipeline-dir "${OUT_DIR}" \
            --output "${LIVE_MODEL}" \
            --once \
            >>"${LOG}" 2>&1 || log "    live dashboard refresh skipped"
    fi
}

log "=== Full pipeline started ==="
refresh_dashboard

cargo build --quiet -p chimiaclaw-cli --features live-sponsors --manifest-path "${REPO_ROOT}/Cargo.toml" 2>>"${LOG}"
CLI="${REPO_ROOT}/target/debug/chimiaclaw-cli"

# Molecules: PG diesters C6/C8/C10/C12 at B3LYP, then 10 ChemRxiv organometallics
SMILES=(
    # PG diesters (smallest first)
    "CCCCCC(=O)OC(C)COC(=O)CCCCC"
    "CCCCCCCC(=O)OC(C)COC(=O)CCCCCCC"
    "CCCCCCCCCC(=O)OC(C)COC(=O)CCCCCCCCC"
    "CCCCCCCCCCCC(=O)OC(C)COC(=O)CCCCCCCCCCC"
    # Group-14 metallylenes (ChemRxiv: Tiekink/Hamlin 2025, Kleinhaus/Gessner 2023)
    "[Si](C)C"
    "[Ge](C)C"
    "[Sn](C)C"
    # Amino-substituted metallylenes (H2 activation study, Kleinhaus/Gessner)
    "[Si](N)N"
    "[Ge](N)N"
    "[Sn](N)N"
    # ALD precursors (Samii/Pedersen 2021 triazenides - simplified models)
    "[GeH2](NC)NC"
    "[SnH2](NC)NC"
    # Tetramethyl group-14 (molecular conductors, Inkpen 2025)
    "C([SiH3])(C)C"
    "C([GeH3])(C)C"
)
NAMES=(
    "propylene-glycol-dihexanoate"
    "propylene-glycol-dioctanoate"
    "propylene-glycol-didecanoate"
    "propylene-glycol-dilaurate"
    "dimethylsilylene"
    "dimethylgermylene"
    "dimethylstannylene"
    "diaminosilylene"
    "diaminogermylene"
    "diaminostannylene"
    "germanium-diamidocarbenoid"
    "tin-diamidocarbenoid"
    "isobutylsilane"
    "isobutylgermane"
)

dft_count=0
dft_ok=0

log ""
log "--- Phase 1: DFT (${#SMILES[@]} molecules, smallest first) ---"
for idx in "${!SMILES[@]}"; do
    smiles="${SMILES[$idx]}"
    name="${NAMES[$idx]}"
    dft_count=$((dft_count + 1))
    log "  [${dft_count}/${#SMILES[@]}] ${name}"

    mol_dir="${OUT_DIR}/molecules/${name}"
    dft_dir="${OUT_DIR}/dft/${name}"
    mkdir -p "${mol_dir}" "${dft_dir}"

    if "${CLI}" moladt-dft-demo \
        --smiles "${smiles}" \
        --functional b3lyp \
        --basis def2-svp \
        --out-dir "${mol_dir}" \
        >>"${LOG}" 2>&1; then
        log "    MolADT: OK"
    else
        log "    MolADT: FAIL"
        continue
    fi

    mol_art=$(ls "${mol_dir}"/chem_molecule_adt.*.json 2>/dev/null | head -1)
    req_art=$(ls "${mol_dir}"/chem_dft_request.*.json 2>/dev/null | head -1)

    # Skip DFT if a result artifact already exists for this molecule
    existing_result=$(ls "${dft_dir}"/chem_dft_result.*.json 2>/dev/null | head -1)
    if [ -n "${existing_result}" ]; then
        dft_ok=$((dft_ok + 1))
        log "    DFT: CACHED ($(basename "${existing_result}"))"
    elif [ -n "${mol_art}" ] && [ -n "${req_art}" ]; then
        if "${CLI}" live dft-execute \
            --request-artifact-json "${req_art}" \
            --molecule-artifact-json "${mol_art}" \
            --out-dir "${dft_dir}" \
            >>"${LOG}" 2>&1; then
            dft_ok=$((dft_ok + 1))
            log "    DFT: OK"
        else
            log "    DFT: FAIL"
        fi
    fi
    refresh_dashboard
    sleep "${DELAY}"
done
log "  DFT: ${dft_ok}/${dft_count} succeeded"

# Phase 2: Uniswap quotes
uniswap_count=0
USDC="0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
USDT="0xdAC17F958D2ee523a2206206994597C13D831ec7"
SWAPPER="0x0000000000000000000000000000000000000001"

log ""
log "--- Phase 2: Uniswap settlement quotes ---"
for amount_label in '3100000:retro-$3.10' '8250000:dft-$8.25' '115500000:batch-14x-$115.50'; do
    amount="${amount_label%%:*}"
    label="${amount_label#*:}"
    log "  [UNISWAP] ${label} (${amount} USDC→USDT)"
    if "${CLI}" live uniswap-quote \
        --token-in "${USDC}" --token-out "${USDT}" \
        --amount "${amount}" --swapper "${SWAPPER}" \
        --chain-id 1 \
        --out-dir "${OUT_DIR}/uniswap" \
        >>"${LOG}" 2>&1; then
        uniswap_count=$((uniswap_count + 1))
        log "    OK"
    else
        log "    FAIL"
    fi
    refresh_dashboard
    sleep 2
done

# Phase 3: 0G anchoring of DFT results
zerog_count=0
log ""
log "--- Phase 3: 0G Storage anchoring ---"
for dft_result in "${OUT_DIR}"/dft/*/chem_dft_result.*.json; do
    [ -f "${dft_result}" ] || continue
    bn="$(basename "${dft_result}")"
    log "  [0G] ${bn}"
    if "${CLI}" live zerog-anchor \
        --source-artifact-json "${dft_result}" \
        --payload-file "${dft_result}" \
        --agent storage.zerog.operator.chimiaclaw.eth \
        > "${OUT_DIR}/zerog/${bn%.json}.anchor.json" 2>>"${LOG}"; then
        zerog_count=$((zerog_count + 1))
        log "    OK"
    else
        log "    FAIL"
    fi
    refresh_dashboard
    sleep 2
done

# Phase 4: ENS publication
ens_count=0
log ""
log "--- Phase 4: ENS publication (Sepolia) ---"
for agent_spec in \
    "dft.service.chimiaclaw.eth:cap.dft.single_point,cap.dft.geometry_opt" \
    "retro.service.chimiaclaw.eth:cap.retrosynth.route_quote" \
    "literature.service.chimiaclaw.eth:cap.literature.synthesis"; do
    agent="${agent_spec%%:*}"
    caps="${agent_spec#*:}"
    log "  [ENS] ${agent}"
    if "${CLI}" live ens-publish \
        --agent "${agent}" \
        --ens chimiaclaw.eth \
        --record "chimiaclaw.capabilities=${caps}" \
        --record "chimiaclaw.settlement.endpoint=uniswap-trade-api:CLASSIC:V2+V3+V4" \
        --out-dir "${OUT_DIR}/ens" \
        >>"${LOG}" 2>&1; then
        ens_count=$((ens_count + 1))
        log "    OK"
    else
        log "    FAIL"
    fi
    refresh_dashboard
    sleep 5
done

# Phase 5: Science market snapshot
log ""
log "--- Phase 5: Science market demo ---"
"${CLI}" science-market-demo > "${OUT_DIR}/science-market-demo.json" 2>>"${LOG}"
refresh_dashboard

log ""
log "=== Full pipeline complete ==="
log "DFT: ${dft_ok}/${dft_count}"
log "Uniswap: ${uniswap_count}"
log "0G: ${zerog_count}"
log "ENS: ${ens_count}"
log "Output: ${OUT_DIR}"
refresh_dashboard
