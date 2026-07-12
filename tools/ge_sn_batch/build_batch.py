#!/usr/bin/env python3
"""Regenerate demo/ge-sn-batch XYZ pack + manifest from MNT Ge sources."""

from __future__ import annotations

import json
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path

# allow running without install
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ge_to_sn import ge_to_sn, parse_xyz, write_xyz  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
MNT = Path("/Users/crischimiadao/Documents/ChimiaDAO-MNT/calculations/structures")
OUT = ROOT / "demo" / "ge-sn-batch"
XYZ_OUT = OUT / "xyz"
GE_REF = OUT / "xyz_ge_ref"

SOURCES = [
    ("NC3Sn_H", MNT / "NC3Ge_H_opt.xyz", "tricarbastannatrane hydride from NC3Ge_H_opt"),
    ("NC3Sn_Cl", MNT / "focused_atrane_precursors/NC3Ge_Cl_parent_initial.xyz", "tricarbastannatrane chloride"),
    ("C3_NC3Sn_H", MNT / "focused_atrane_precursors/c3_idealized/C3_NC3Ge_H_parent.xyz", "idealized C3 NC3Sn-H"),
    ("C3_NC3Sn_Cl", MNT / "focused_atrane_precursors/c3_idealized/C3_NC3Ge_Cl_parent.xyz", "idealized C3 NC3Sn-Cl"),
    ("beta_OH_NC3Sn_H", MNT / "focused_atrane_precursors/c3_idealized/C3_beta_3x_CH2OH_NC3Ge_H.xyz", "beta-CH2OH NC3Sn-H"),
    ("beta_SH_NC3Sn_H", MNT / "focused_atrane_precursors/c3_idealized/C3_beta_3x_CH2SH_NC3Ge_H.xyz", "beta-CH2SH NC3Sn-H"),
    ("Ad_stannatrane", MNT / "Ad_germatrane_opt.xyz", "1-adamantylstannatrane from germatrane_opt"),
    ("Ad_SnH3", MNT / "Ad_GeH3_opt.xyz", "1-adamantyl-SnH3"),
    ("Ad_SnCl3", MNT / "Ad_GeCl3_opt.xyz", "1-adamantyl-SnCl3"),
    ("Ad_SnMe3", MNT / "Ad_GeMe3_opt.xyz", "1-adamantyl-SnMe3"),
]


def main() -> int:
    XYZ_OUT.mkdir(parents=True, exist_ok=True)
    GE_REF.mkdir(parents=True, exist_ok=True)
    entries = []
    for label, src, desc in SOURCES:
        if not src.exists():
            print(f"MISSING {src}", file=sys.stderr)
            continue
        _comment, atoms = parse_xyz(src.read_text())
        sn = ge_to_sn(atoms, scale_metal_bonds=True)
        sn_path = XYZ_OUT / f"{label}.xyz"
        write_xyz(
            sn_path,
            sn,
            f"{label} | Ge→Sn scaled | from {src.name} | {desc}",
        )
        shutil.copy2(src, GE_REF / f"{src.stem}_src.xyz")
        entries.append(
            {
                "label": label,
                "xyz": str(sn_path.relative_to(ROOT)),
                "source_ge_xyz": str(src),
                "description": desc,
                "n_atoms": len(sn),
                "n_sn": sum(a[0] == "Sn" for a in sn),
                "charge": 0,
                "multiplicity": 1,
                "method": {
                    "functional": "pbe",
                    "basis_set": "def2-svp",
                    "backend": "pyscf-classical",
                },
            }
        )
        print(f"wrote {sn_path.name} ({len(sn)} atoms)")

    manifest = {
        "schema_tag": "chem.dft.ge_sn_batch.v1",
        "title": "Ge→Sn starting-point batch for Olympus PySCF",
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "scaling": {
            "factor": 1.39 / 1.20,
            "neighbor_cutoff_angstrom": 2.85,
        },
        "molecules": entries,
        "count": len(entries),
    }
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"manifest: {len(entries)} molecules → {OUT / 'manifest.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
