#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=1.26", "matplotlib>=3.8"]
# ///
"""Render Gaussian-cube orbital/density files as 2D PNG slices.

For each .cube file in --cube-dir, produce one PNG showing a 2D projection
(integrated along the principal molecular axis) plus an annotation of the
cube label, grid resolution, and SHA-256 prefix.  HOMO/LUMO orbitals get a
diverging colormap (positive lobes red, negative lobes blue, nodal surface
white); total densities get a sequential colormap.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np


ELEMENT_BY_Z = {
    1: "H", 5: "B", 6: "C", 7: "N", 8: "O", 9: "F", 11: "Na",
    15: "P", 16: "S", 17: "Cl", 26: "Fe", 35: "Br", 53: "I",
}


def parse_cube(path: Path):
    """Parse a Gaussian cube file.

    Returns (atoms, origin_bohr, axes_bohr, grid_data_3d).
    """
    with path.open("r") as f:
        # Two title lines
        f.readline()
        f.readline()
        # natoms + origin
        parts = f.readline().split()
        natoms = int(parts[0])
        origin = np.array([float(parts[1]), float(parts[2]), float(parts[3])])
        # axes
        axes = []
        n_voxels = []
        for _ in range(3):
            parts = f.readline().split()
            n_voxels.append(int(parts[0]))
            axes.append([float(parts[1]), float(parts[2]), float(parts[3])])
        axes = np.array(axes)
        # atoms
        atoms = []
        for _ in range(abs(natoms)):
            parts = f.readline().split()
            atoms.append({
                "Z": int(parts[0]),
                "charge": float(parts[1]),
                "xyz": np.array([float(parts[2]), float(parts[3]), float(parts[4])]),
            })
        # grid data
        nx, ny, nz = n_voxels
        data = np.zeros(nx * ny * nz, dtype=np.float64)
        idx = 0
        for line in f:
            for tok in line.split():
                if idx >= data.size:
                    break
                data[idx] = float(tok)
                idx += 1
            if idx >= data.size:
                break
        data = data.reshape(nx, ny, nz)
    return atoms, origin, axes, data


def render_cube(path: Path, out_path: Path) -> None:
    atoms, origin, axes, data = parse_cube(path)
    label_match = re.search(r"_(HOMO|LUMO|TOTAL_DENSITY)_", path.name)
    label = label_match.group(1) if label_match else "ORBITAL"

    nx, ny, nz = data.shape
    if label == "TOTAL_DENSITY":
        # Density is positive; integrate along z for a column-density view.
        proj = data.sum(axis=2)
    else:
        # Orbitals are typically antisymmetric about the molecular plane
        # (π systems) so a signed sum along z cancels.  Pick the z-slice
        # at which |ψ(x,y,z)| is maximal for each (x,y), keeping the sign
        # — this surfaces the dominant lobe and preserves chemistry.
        abs_data = np.abs(data)
        max_z_idx = abs_data.argmax(axis=2)
        ix, iy = np.meshgrid(np.arange(nx), np.arange(ny), indexing="ij")
        proj = data[ix, iy, max_z_idx]

    # Real-space extents (axes are in Bohr; convert to Angstrom for axis labels).
    bohr_to_angstrom = 0.529177
    nx, ny, nz = data.shape
    x_extent = np.linalg.norm(axes[0]) * nx * bohr_to_angstrom
    y_extent = np.linalg.norm(axes[1]) * ny * bohr_to_angstrom
    origin_ang = origin * bohr_to_angstrom

    fig, ax = plt.subplots(figsize=(5.5, 5.0), dpi=140)

    if label == "TOTAL_DENSITY":
        # Symmetric log to compress the dynamic range; densities are non-negative.
        vmax = float(np.percentile(np.abs(proj), 99.9))
        cmap = "viridis"
        im = ax.imshow(
            proj.T,
            origin="lower",
            extent=(
                origin_ang[0],
                origin_ang[0] + x_extent,
                origin_ang[1],
                origin_ang[1] + y_extent,
            ),
            cmap=cmap,
            vmin=0,
            vmax=vmax,
            aspect="equal",
            interpolation="bilinear",
        )
    else:
        vmax = float(np.percentile(np.abs(proj), 99.5))
        if vmax == 0.0:
            vmax = float(np.abs(proj).max() or 1.0)
        cmap = "RdBu_r"  # red = positive lobe, blue = negative lobe
        im = ax.imshow(
            proj.T,
            origin="lower",
            extent=(
                origin_ang[0],
                origin_ang[0] + x_extent,
                origin_ang[1],
                origin_ang[1] + y_extent,
            ),
            cmap=cmap,
            vmin=-vmax,
            vmax=vmax,
            aspect="equal",
            interpolation="bilinear",
        )

    # Overlay nuclei (project x,y of atom positions, also Bohr -> Angstrom).
    for atom in atoms:
        x_a, y_a, _ = atom["xyz"] * bohr_to_angstrom
        sym = ELEMENT_BY_Z.get(atom["Z"], "?")
        size = 70 if atom["Z"] > 1 else 28
        face = {
            "C": "#222",
            "H": "#bbb",
            "O": "#cc2222",
            "N": "#3333cc",
            "F": "#33aa33",
            "Cl": "#33aa33",
            "Br": "#993",
            "Fe": "#cc7000",
        }.get(sym, "#444")
        ax.scatter(
            x_a,
            y_a,
            s=size,
            c=face,
            edgecolors="white",
            linewidths=1.2,
            zorder=3,
        )
        if atom["Z"] > 1:
            ax.text(x_a, y_a, sym, color="white", ha="center", va="center",
                    fontsize=7, fontweight="bold", zorder=4)

    sha_match = re.search(r"_([0-9a-f]{16})\.cube$", path.name)
    sha_prefix = sha_match.group(1) if sha_match else "?" * 16
    file_size_kb = path.stat().st_size // 1024
    ax.set_title(
        f"{path.stem.split('_')[0]} \u2014 {label}\nresolution {nx}\u00d7{ny}\u00d7{nz}, "
        f"sha256: {sha_prefix}\u2026, {file_size_kb} KB",
        fontsize=10,
    )
    ax.set_xlabel(r"x (\u00c5)")
    ax.set_ylabel(r"y (\u00c5)")
    plt.colorbar(im, ax=ax, fraction=0.046, pad=0.04, label="signed amplitude" if label != "TOTAL_DENSITY" else r"$\rho(r)$ projected along z")
    plt.tight_layout()
    plt.savefig(out_path, dpi=140, bbox_inches="tight")
    plt.close(fig)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cube-dir", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, required=True)
    args = parser.parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    cubes = sorted(args.cube_dir.glob("*.cube"))
    print(f"rendering {len(cubes)} cubes \u2192 {args.out_dir}", file=sys.stderr)
    for cube in cubes:
        png_name = cube.stem + ".png"
        out_path = args.out_dir / png_name
        try:
            render_cube(cube, out_path)
            print(f"  {cube.name} -> {out_path.name}", file=sys.stderr)
        except Exception as exc:  # pylint: disable=broad-except
            print(f"  {cube.name} FAILED: {exc}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
