#!/usr/bin/env python3
"""One-shot pull of Olympus Ge→Sn results + rewrite RESULTS.md.

Does **not** wait for jobs. Safe to run anytime:

  python3 tools/ge_sn_batch/collect_results.py
  python3 tools/ge_sn_batch/collect_results.py --no-scp   # local only
"""

from __future__ import annotations

import argparse
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BATCH = ROOT / "demo" / "ge-sn-batch"
RESULTS = BATCH / "results"
PLANNED = [
    "NC3Sn_H",
    "NC3Sn_Cl",
    "C3_NC3Sn_H",
    "C3_NC3Sn_Cl",
    "Ad_SnH3",
    "Ad_SnCl3",
    "Ad_SnMe3",
    "beta_OH_NC3Sn_H",
    "beta_SH_NC3Sn_H",
    "Ad_stannatrane",
]


def scp_pull() -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    cmd = [
        "scp",
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=15",
        "duck@olympus.local:/tmp/ge-sn-batch/*.dft_result.json",
        str(RESULTS) + "/",
    ]
    subprocess.run(cmd, check=False)


def load_rows() -> list[dict]:
    rows: list[dict] = []
    for label in PLANNED:
        path = RESULTS / f"{label}.dft_result.json"
        if not path.exists() or path.stat().st_size < 50:
            continue
        try:
            body = json.loads(path.read_text())
        except json.JSONDecodeError:
            continue
        conv = body.get("convergence") or {}
        orbs = body.get("orbitals") or {}
        dip = body.get("dipole") or {}
        rows.append(
            {
                "label": label,
                "energy_hartree": body.get("energy_hartree"),
                "converged": conv.get("converged"),
                "n_cycles": conv.get("n_cycles"),
                "gap_ev": orbs.get("gap_ev"),
                "dipole_debye": dip.get("magnitude_debye"),
                "wall_seconds": (body.get("timings") or {}).get("wall_seconds"),
                "functional": body.get("functional"),
                "basis_set": body.get("basis_set"),
                "host": (body.get("provenance") or {}).get("host"),
            }
        )
    return rows


def write_summary(rows: list[dict]) -> None:
    done = [r for r in rows if r.get("converged")]
    failed = [r for r in rows if r.get("converged") is False]
    have = {r["label"] for r in rows}
    missing = [p for p in PLANNED if p not in have]
    summary = {
        "schema_tag": "chem.dft.ge_sn_batch.results.v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "method": "PBE/def2-svp single-point on Ge→Sn scaled starting geometries",
        "host": "Olympus.local",
        "completed_converged": len(done),
        "failed_unconverged": [r["label"] for r in failed],
        "pending": missing,
        "total_planned": len(PLANNED),
        "results": rows,
        "notes": [
            "Raw worker JSON — not sealed ChimiaClaw chem.dft.result artifacts.",
            "beta_* jobs may fail SCF on crude scaled geometries; re-opt first.",
        ],
    }
    (BATCH / "results_summary.json").write_text(json.dumps(summary, indent=2) + "\n")

    md = [
        "# Ge→Sn batch results (Olympus)",
        "",
        f"Updated: `{summary['generated_at_utc']}`",
        "",
        f"**{len(done)} / {len(PLANNED)} converged** · "
        f"{len(failed)} unconverged · {len(missing)} missing",
        "",
        "| Molecule | status | E (Ha) | gap (eV) | wall (s) |",
        "|----------|--------|-------:|---------:|---------:|",
    ]
    by = {r["label"]: r for r in rows}
    for label in PLANNED:
        r = by.get(label)
        if not r:
            md.append(f"| {label} | pending | — | — | — |")
            continue
        st = "converged" if r.get("converged") else "unconverged"
        e, g, w = r.get("energy_hartree"), r.get("gap_ev"), r.get("wall_seconds")
        md.append(
            f"| {label} | {st} | {e:.6f} | "
            f"{g if g is not None else '—'} | "
            f"{w if w is not None else '—'} |"
        )
    md += [
        "",
        "## Notes",
        "",
        "- Collect anytime: `python3 tools/ge_sn_batch/collect_results.py`",
        "- Do not wait on this script; it never blocks on SCF.",
        "",
    ]
    (BATCH / "RESULTS.md").write_text("\n".join(md))
    print(json.dumps({k: summary[k] for k in (
        "completed_converged", "failed_unconverged", "pending"
    )}, indent=2))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-scp", action="store_true")
    args = ap.parse_args()
    if not args.no_scp:
        scp_pull()
    rows = load_rows()
    write_summary(rows)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
