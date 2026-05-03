#!/usr/bin/env bash
set -euo pipefail
cd /vercel/share/v0-project
echo "[v0] Restoring SciCrucible_v1/package.json from git..."
git checkout HEAD -- SciCrucible_v1/package.json
echo "[v0] Restored. Contents:"
cat SciCrucible_v1/package.json
