#!/usr/bin/env bash
# Run Ge→Sn single-point batch on duck@olympus.local via ChimiaClaw DFT worker.
#
# Usage (from OpenAgents repo root):
#   ./demo/ge-sn-batch/run_olympus.sh              # dry-run plan
#   ./demo/ge-sn-batch/run_olympus.sh --execute    # real SCF
#   ./demo/ge-sn-batch/run_olympus.sh --label NC3Sn_H --execute

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BATCH="$ROOT/demo/ge-sn-batch"
MANIFEST="$BATCH/manifest.json"
OUT_DIR="$BATCH/results"
EXECUTE=0
ONLY_LABEL=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --execute) EXECUTE=1; shift ;;
    --label) ONLY_LABEL="$2"; shift 2 ;;
    -h|--help) sed -n '1,12p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

WORKER_REMOTE="${CHIMIACLAW_DFT_COMMAND:-ssh duck@olympus.local /Users/duck/.local/bin/uv run --project /Users/duck/Documents/ChimiaDAO-QM/DFT/skills/scienceclaw-port/workers/dft chimiaclaw-dft --backend pyscf-classical}"

echo "manifest: $MANIFEST"
echo "worker:   $WORKER_REMOTE"
echo "execute:  $EXECUTE"
echo "label:    ${ONLY_LABEL:-*(all)*}"

if [[ ! -f "$MANIFEST" ]]; then
  echo "missing manifest" >&2
  exit 1
fi

python3 - "$MANIFEST" "$ONLY_LABEL" <<'PY'
import json, sys
from pathlib import Path
m = json.loads(Path(sys.argv[1]).read_text())
only = sys.argv[2]
n = 0
for mol in m["molecules"]:
    if only and mol["label"] != only:
        continue
    n += 1
    print(f"{mol['label']:20s}  atoms={mol['n_atoms']:3d}  {mol['xyz']}")
print(f"total selected: {n}")
PY

if [[ "$EXECUTE" -ne 1 ]]; then
  echo
  echo "Dry-run only. Pass --execute after reviewing spend/time."
  exit 0
fi

mkdir -p "$OUT_DIR"
export CHIMIACLAW_DFT_COMMAND="$WORKER_REMOTE"
export GE_SN_ROOT="$ROOT"
export GE_SN_BATCH="$BATCH"
export GE_SN_ONLY="$ONLY_LABEL"

python3 <<'PY'
import json, os, shlex, subprocess, sys
from pathlib import Path

root = Path(os.environ["GE_SN_ROOT"])
batch = Path(os.environ["GE_SN_BATCH"])
only = os.environ.get("GE_SN_ONLY") or ""
manifest = json.loads((batch / "manifest.json").read_text())
out_dir = batch / "results"
out_dir.mkdir(exist_ok=True)
worker = os.environ["CHIMIACLAW_DFT_COMMAND"]


def atoms_from_xyz(path: Path):
    lines = path.read_text().strip().splitlines()
    n = int(lines[0].split()[0])
    atoms = {}
    for i, ln in enumerate(lines[2 : 2 + n]):
        parts = ln.split()
        sym, x, y, z = parts[0], float(parts[1]), float(parts[2]), float(parts[3])
        atoms[str(i)] = {
            "attributes": {"symbol": sym},
            "coordinate": {
                "x_angstrom": x,
                "y_angstrom": y,
                "z_angstrom": z,
            },
        }
    return atoms, n


failed = []
for mol in manifest["molecules"]:
    if only and mol["label"] != only:
        continue
    label = mol["label"]
    xyz = (root / mol["xyz"]).resolve()
    if not xyz.exists():
        print(f"MISSING xyz {xyz}", file=sys.stderr)
        failed.append(label)
        continue
    atoms, n = atoms_from_xyz(xyz)
    method = mol.get("method") or {}
    payload = {
        "request": {
            "request_id": f"REQ.GE_SN.{label}",
            "total_charge": mol.get("charge", 0),
            "multiplicity": mol.get("multiplicity", 1),
            "molecule": {
                "molecule_id": f"MOLADT.GE_SN.{label}",
                "molecule_name": label,
            },
            "method": {
                "functional": method.get("functional", "pbe"),
                "basis_set": method.get("basis_set", "def2-svp"),
                "backend": "PyScf",
            },
            "requested_properties": ["total_energy", "homo_lumo_gap", "dipole"],
        },
        "molecule_adt": {"atoms": atoms},
    }
    job_path = out_dir / f"{label}.job.json"
    result_path = out_dir / f"{label}.dft_result.json"
    job_path.write_text(json.dumps(payload, indent=2))
    print(f"==> {label} ({n} atoms)", flush=True)
    # worker is a shell string (often ssh … uv run …)
    shell = f"{worker} < {shlex.quote(str(job_path))} > {shlex.quote(str(result_path))} 2> {shlex.quote(str(out_dir / (label + '.err')))}"
    rc = subprocess.call(shell, shell=True)
    if rc != 0:
        print(f"FAILED {label} rc={rc}", file=sys.stderr)
        err = (out_dir / f"{label}.err").read_text()[-2000:]
        print(err, file=sys.stderr)
        failed.append(label)
        continue
    try:
        body = json.loads(result_path.read_text())
        orbs = body.get("orbitals") or {}
        print(
            f"    E={body.get('energy_hartree')}  "
            f"conv={(body.get('convergence') or {}).get('converged')}  "
            f"gap={orbs.get('gap_ev')}"
        )
    except Exception as e:
        print(f"    result unreadable: {e}", file=sys.stderr)
        failed.append(label)

print("done — results in", out_dir)
if failed:
    print("failed:", ", ".join(failed), file=sys.stderr)
    sys.exit(1)
PY
