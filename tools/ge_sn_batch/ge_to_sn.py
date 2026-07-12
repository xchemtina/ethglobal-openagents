#!/usr/bin/env python3
"""Ge → Sn XYZ conversion for atrane / organometallic starting points.

Does **not** claim crystal accuracy. Produces a chemically intended starting
geometry for PySCF single-points on Olympus:

1. Replace every ``Ge`` with ``Sn``.
2. Optionally scale each metal–ligand vector by r(Sn)/r(Ge) (Cordero 2008).

Usage:
  python tools/ge_sn_batch/ge_to_sn.py input.xyz -o out.xyz
  python tools/ge_sn_batch/ge_to_sn.py input.xyz --no-scale -o out.xyz
"""

from __future__ import annotations

import argparse
from pathlib import Path

# Cordero et al. covalent radii (Å)
COVALENT_RADIUS = {
    "Ge": 1.20,
    "Sn": 1.39,
    "Si": 1.11,
    "C": 0.76,
    "N": 0.71,
    "O": 0.66,
    "H": 0.31,
    "S": 1.05,
    "Cl": 1.02,
}


def parse_xyz(text: str) -> tuple[str, list[tuple[str, float, float, float]]]:
    # Keep blank lines: many MNT XYZs use "N\\n\\n atom..." (empty comment).
    raw = [ln.rstrip() for ln in text.splitlines()]
    lines = [ln for ln in raw if ln is not None]
    if not lines:
        raise ValueError("empty xyz")
    try:
        n = int(lines[0].split()[0])
    except ValueError as exc:
        raise ValueError("xyz must start with atom count") from exc
    comment = lines[1] if len(lines) > 1 else ""
    # If "comment" looks like an atom line, the blank comment was stripped upstream
    # in older files — recover by treating it as first atom.
    atoms: list[tuple[str, float, float, float]] = []
    start = 2
    parts1 = comment.split()
    if len(parts1) >= 4:
        try:
            float(parts1[1])
            float(parts1[2])
            float(parts1[3])
            # comment is actually atom 1
            atoms.append(
                (parts1[0], float(parts1[1]), float(parts1[2]), float(parts1[3]))
            )
            comment = ""
            start = 2
        except ValueError:
            pass
    for ln in lines[start:]:
        if not ln.strip():
            continue
        parts = ln.split()
        if len(parts) < 4:
            continue
        try:
            atoms.append(
                (parts[0], float(parts[1]), float(parts[2]), float(parts[3]))
            )
        except ValueError:
            continue
        if len(atoms) >= n:
            break
    if not atoms:
        raise ValueError("no atoms parsed")
    return comment, atoms


def write_xyz(
    path: Path,
    atoms: list[tuple[str, float, float, float]],
    comment: str,
) -> None:
    lines = [str(len(atoms)), comment]
    for sym, x, y, z in atoms:
        lines.append(f"{sym:2s}  {x:14.8f}  {y:14.8f}  {z:14.8f}")
    path.write_text("\n".join(lines) + "\n")


def ge_to_sn(
    atoms: list[tuple[str, float, float, float]],
    *,
    scale_metal_bonds: bool = True,
    neighbor_cutoff: float = 2.85,
    factor: float | None = None,
) -> list[tuple[str, float, float, float]]:
    if not any(sym == "Ge" for sym, *_ in atoms):
        raise ValueError("no Ge atoms found")
    scale = factor if factor is not None else (
        COVALENT_RADIUS["Sn"] / COVALENT_RADIUS["Ge"]
    )
    out: list[tuple[str, float, float, float]] = [
        (("Sn" if sym == "Ge" else sym), x, y, z) for sym, x, y, z in atoms
    ]
    if not scale_metal_bonds:
        return out

    coords = [(x, y, z) for _, x, y, z in out]
    metals = [i for i, a in enumerate(out) if a[0] == "Sn"]
    new_coords = [list(c) for c in coords]
    for mi in metals:
        mx, my, mz = coords[mi]
        for j, (sym, x, y, z) in enumerate(out):
            if j == mi:
                continue
            dx, dy, dz = x - mx, y - my, z - mz
            dist = (dx * dx + dy * dy + dz * dz) ** 0.5
            if 0.5 < dist < neighbor_cutoff:
                nd = dist * scale
                s = nd / dist
                new_coords[j] = [mx + dx * s, my + dy * s, mz + dz * s]
    return [(out[i][0], *new_coords[i]) for i in range(len(out))]


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("input_xyz", type=Path)
    p.add_argument("-o", "--output", type=Path, required=True)
    p.add_argument("--no-scale", action="store_true", help="symbol swap only")
    p.add_argument(
        "--cutoff",
        type=float,
        default=2.85,
        help="metal–ligand scale cutoff (Å)",
    )
    args = p.parse_args(argv)
    comment, atoms = parse_xyz(args.input_xyz.read_text())
    sn = ge_to_sn(
        atoms,
        scale_metal_bonds=not args.no_scale,
        neighbor_cutoff=args.cutoff,
    )
    new_comment = (
        f"Ge→Sn from {args.input_xyz.name} | scale={not args.no_scale} | {comment}"
    )
    write_xyz(args.output, sn, new_comment)
    print(f"wrote {args.output} ({len(sn)} atoms, {sum(a[0]=='Sn' for a in sn)} Sn)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
