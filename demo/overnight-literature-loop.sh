#!/usr/bin/env bash
# Closed-loop literature lane: ingest → extract → sign → handoff → verify.
#
# This script runs the full Phase-1 chain end-to-end for one query:
#   1. Sign a `science.literature.synthesis` artifact (offline fixture by
#      default; pass --synthesis-json to consume real Python-worker output).
#   2. Hand off any extracted MoleculeCandidates as `chem.molecule.adt`
#      artifacts ready for the existing Retrosynthesis lane.
#   3. Refresh the live dashboard projection.
#   4. Run `world-model verify` against the live model and fail loudly on
#      any unresolved artifact reference.
#
# Required env (none, by default the offline fixture path is used).
#
# Optional env / flags:
#   QUERY                       Free-text query for logging only
#                               (default: "main-group hypovalent carbenoids")
#   SYNTHESIS_JSON              Path to a LiteratureSynthesis JSON (default:
#                               skills/literature_synthesis/fixtures/sample_synthesis.json)
#   MANIFEST_JSON               Optional LiteratureIngestManifest JSON to
#                               parent the synthesis on (default: empty).
#   OUT_DIR                     Output dir (default: demo/overnight-full-out)
#   ALLOW_WORKER                If "1" do NOT pass --no-worker to handoff so
#                               unsupported SMILES will hit
#                               CHIMIACLAW_SMILES_TO_MOLADT_COMMAND if set.
#   SKIP_WORLD_MODEL_VERIFY     If "1" skip the world-model verify step.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
QUERY="${QUERY:-main-group hypovalent carbenoids}"
SYNTHESIS_JSON="${SYNTHESIS_JSON:-${REPO_ROOT}/skills/literature_synthesis/fixtures/sample_synthesis.json}"
MANIFEST_JSON="${MANIFEST_JSON:-}"
OUT_DIR="${OUT_DIR:-${REPO_ROOT}/demo/overnight-full-out}"
LIVE_MODEL="${LIVE_MODEL:-${REPO_ROOT}/demo/world-model.live.json}"
LIVE_DASHBOARD_WATCHER="${LIVE_DASHBOARD_WATCHER:-${REPO_ROOT}/demo/live-dashboard-watch.py}"
ALLOW_WORKER="${ALLOW_WORKER:-0}"
SKIP_WORLD_MODEL_VERIFY="${SKIP_WORLD_MODEL_VERIFY:-0}"

LITERATURE_DIR="${OUT_DIR}/literature"
MOLECULES_DIR="${LITERATURE_DIR}/handoff-molecules"
LOG="${LITERATURE_DIR}/overnight-literature.log"
mkdir -p "${LITERATURE_DIR}" "${MOLECULES_DIR}"

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

log "=== overnight-literature-loop started ==="
log "query: ${QUERY}"
log "synthesis_json: ${SYNTHESIS_JSON}"
log "manifest_json: ${MANIFEST_JSON:-<none>}"

if ! cargo build --quiet -p chimiaclaw-cli --manifest-path "${REPO_ROOT}/Cargo.toml" 2>>"${LOG}"; then
    log "    FAIL: could not build chimiaclaw-cli"
    exit 1
fi

CLI="${REPO_ROOT}/target/debug/chimiaclaw-cli"

# Phase 1: sign the synthesis (and optionally the ingest manifest).
log ""
log "--- Phase 1: sign science.literature.synthesis ---"
demo_args=(
    "science-literature-demo"
    "--synthesis-json" "${SYNTHESIS_JSON}"
    "--out-dir" "${LITERATURE_DIR}"
)
if [ -n "${MANIFEST_JSON}" ]; then
    demo_args+=("--manifest-json" "${MANIFEST_JSON}")
fi
"${CLI}" "${demo_args[@]}" >>"${LOG}" 2>&1 \
    && log "    OK" \
    || { log "    FAIL"; exit 1; }

synth_art=$(ls "${LITERATURE_DIR}"/science_literature_synthesis.art_*.json 2>/dev/null | head -1)
if [ -z "${synth_art}" ]; then
    log "    no synthesis artifact written; aborting"
    exit 1
fi
log "    synthesis artifact: $(basename "${synth_art}")"
refresh_dashboard

# Phase 2: handoff every MoleculeCandidate to a signed chem.molecule.adt artifact.
log ""
log "--- Phase 2: literature-handoff -> chem.molecule.adt ---"
handoff_args=(
    "live" "literature-handoff"
    "--synthesis-artifact-json" "${synth_art}"
    "--out-dir" "${MOLECULES_DIR}"
)
if [ "${ALLOW_WORKER}" != "1" ]; then
    handoff_args+=("--no-worker")
fi
"${CLI}" "${handoff_args[@]}" >>"${LOG}" 2>&1 \
    && log "    OK" \
    || log "    FAIL (continuing; some SMILES may have been skipped)"
refresh_dashboard

# Phase 3: world-model verify.
if [ "${SKIP_WORLD_MODEL_VERIFY}" != "1" ]; then
    log ""
    log "--- Phase 3: world-model verify (live projection) ---"
    if "${CLI}" world-model verify \
        --world-model "${LIVE_MODEL}" \
        --artifact-dir "${OUT_DIR}" \
        >>"${LOG}" 2>&1; then
        log "    OK"
    else
        log "    FAIL: at least one reference or model invariant did not verify"
        exit 1
    fi
fi

log ""
log "=== overnight-literature-loop complete ==="
log "Output: ${LITERATURE_DIR}"
log "Handoff molecules: ${MOLECULES_DIR}"
