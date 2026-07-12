"""MCP server for UCSF ChimeraX.

Two backends:

1. **CLI** (default) — spawn ChimeraX ``--nogui`` / GUI with ``--cmd`` for each
   tool call. Reliable, no extra ChimeraX setup.
2. **REST** — if ``CHIMERAX_REST_URL`` is set (or after ``rest_start``), send
   commands to ChimeraX's ``remotecontrol rest`` HTTP API for a live session.

Env:

- ``CHIMERAX_BIN`` — path to ChimeraX binary
  (default: ``/Applications/ChimeraX-1.11.1.app/Contents/bin/ChimeraX``)
- ``CHIMERAX_REST_URL`` — e.g. ``http://127.0.0.1:61886/run``
- ``CHIMERAX_GUI`` — ``1`` to open GUI instead of ``--nogui`` for CLI mode

Why ChimeraX over Avogadro here: session scripting, REST remote control,
publication-quality rendering, volumes/cubes, and multi-model compare — better
fit for signed DFT galleries and Ge/Sn atrane overlays.
"""

from __future__ import annotations

import os
import shlex
import subprocess
import tempfile
from pathlib import Path
from typing import Any
from urllib.parse import quote

import httpx
from mcp.server.fastmcp import FastMCP

mcp = FastMCP("chimerax")

DEFAULT_BIN = "/Applications/ChimeraX-1.11.1.app/Contents/bin/ChimeraX"


def _bin() -> str:
    path = os.environ.get("CHIMERAX_BIN", DEFAULT_BIN)
    if not Path(path).exists():
        # fall back to PATH
        return os.environ.get("CHIMERAX_BIN", "ChimeraX")
    return path


def _rest_url() -> str | None:
    return os.environ.get("CHIMERAX_REST_URL") or None


def _xyz_to_pdb(xyz_path: Path) -> Path:
    """ChimeraX 1.11 has no built-in .xyz opener — convert to PDB HETATM.

    Writes beside the xyz (or under /tmp) as ``*.chimerax.pdb``.
    """
    text = xyz_path.read_text().strip().splitlines()
    if not text:
        raise ValueError(f"empty xyz: {xyz_path}")
    try:
        n = int(text[0].split()[0])
        atom_lines = text[2 : 2 + n]
    except ValueError:
        # bare element lines
        atom_lines = [ln for ln in text if len(ln.split()) >= 4 and ln.split()[0][0].isalpha()]
        n = len(atom_lines)
    out = xyz_path.with_suffix(".chimerax.pdb")
    rows = ["REMARK converted from " + xyz_path.name]
    for i, ln in enumerate(atom_lines, 1):
        parts = ln.split()
        sym = parts[0]
        # Normalize element: first letter upper, rest lower (Sn, Cl, …)
        el = sym[0].upper() + (sym[1:].lower() if len(sym) > 1 else "")
        x, y, z = float(parts[1]), float(parts[2]), float(parts[3])
        # Fixed-width PDB HETATM (atom name cols 13-16, element 77-78).
        atom_name = f"{el:>2s}"[:4].ljust(4)
        elem = f"{el:>2s}"[:2]
        rows.append(
            f"HETATM{i:5d} {atom_name} UNL A   1    "
            f"{x:8.3f}{y:8.3f}{z:8.3f}  1.00  0.00          {elem}"
        )
    rows.append("END")
    out.write_text("\n".join(rows) + "\n")
    return out


def _structure_path_for_chimerax(path: Path) -> Path:
    if path.suffix.lower() == ".xyz":
        return _xyz_to_pdb(path)
    return path


def _run_cli(commands: list[str], *, timeout: float = 120.0) -> str:
    """Run a batch of ChimeraX commands in one process."""
    # Join with semicolons; ChimeraX --cmd accepts a command string.
    cmd_str = "; ".join(commands)
    argv = [_bin()]
    if os.environ.get("CHIMERAX_GUI", "0") not in ("1", "true", "True"):
        argv.append("--nogui")
        argv.append("--offscreen")
    argv.extend(["--cmd", cmd_str, "--exit"])
    try:
        proc = subprocess.run(
            argv,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except FileNotFoundError as exc:
        raise RuntimeError(
            f"ChimeraX not found at {_bin()!r}. Set CHIMERAX_BIN."
        ) from exc
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError(f"ChimeraX timed out after {timeout}s") from exc
    out = (proc.stdout or "") + (proc.stderr or "")
    if proc.returncode != 0:
        raise RuntimeError(
            f"ChimeraX exit {proc.returncode}\n{out[-4000:]}"
        )
    return out[-8000:] if out else "ok (no stdout)"


def _run_rest(command: str, timeout: float = 60.0) -> str:
    url = _rest_url()
    if not url:
        raise RuntimeError("CHIMERAX_REST_URL not set")
    # ChimeraX REST expects command as query or JSON depending on version.
    # Common pattern: GET /run?command=...
    try:
        r = httpx.get(
            url,
            params={"command": command},
            timeout=timeout,
        )
        if r.status_code >= 400:
            # try POST json
            r = httpx.post(url, json={"command": command}, timeout=timeout)
        r.raise_for_status()
        return r.text[:8000]
    except Exception as exc:
        raise RuntimeError(f"REST command failed: {exc}") from exc


def run_cx(commands: list[str] | str, *, prefer_rest: bool = True) -> str:
    if isinstance(commands, str):
        commands = [commands]
    if prefer_rest and _rest_url():
        return _run_rest("; ".join(commands))
    return _run_cli(commands)


@mcp.tool()
def chimerax_info() -> dict[str, Any]:
    """Report ChimeraX binary path, REST URL, and version if available."""
    binary = _bin()
    version = None
    if Path(binary).exists() or binary == "ChimeraX":
        try:
            proc = subprocess.run(
                [binary, "--version"],
                capture_output=True,
                text=True,
                timeout=30,
            )
            version = (proc.stdout or proc.stderr or "").strip().splitlines()[:3]
        except Exception as exc:  # noqa: BLE001
            version = [f"version probe failed: {exc}"]
    return {
        "binary": binary,
        "binary_exists": Path(binary).exists() if binary.startswith("/") else None,
        "rest_url": _rest_url(),
        "gui": os.environ.get("CHIMERAX_GUI", "0"),
        "version": version,
        "hint": "Prefer ChimeraX for multi-model Ge/Sn overlays and cube volumes; Avogadro is fine for quick edits.",
    }


@mcp.tool()
def open_structure(
    path: str,
    *,
    clear: bool = False,
    style: str = "stick",
) -> str:
    """Open a structure file (xyz, pdb, mol2, cif, …) in ChimeraX.

    Args:
        path: Absolute or workspace-relative path to the structure.
        clear: If true, close existing models first.
        style: Representation — stick | ball | sphere | cartoon.
    """
    p = Path(path).expanduser().resolve()
    if not p.exists():
        return f"error: file not found: {p}"
    try:
        open_path = _structure_path_for_chimerax(p)
    except Exception as exc:  # noqa: BLE001
        return f"error converting structure: {exc}"
    cmds: list[str] = []
    if clear:
        cmds.append("close session")
    cmds.append(f"open {shlex.quote(str(open_path))}")
    style_map = {
        "stick": "style stick",
        "ball": "style ball",
        "sphere": "style sphere",
        "cartoon": "cartoon",
    }
    cmds.append(style_map.get(style, "style stick"))
    cmds.append("color byelement")
    try:
        log = run_cx(cmds)
        note = ""
        if open_path != p:
            note = f" (xyz→pdb via {open_path.name}; ChimeraX has no native .xyz)"
        return f"opened {p}{note}\n{log}"
    except Exception as exc:  # noqa: BLE001
        return f"error: {exc}"


@mcp.tool()
def run_command(command: str) -> str:
    """Run an arbitrary ChimeraX command string (advanced).

    Example: ``color #1 byhetero; surface``
    """
    if not command.strip():
        return "error: empty command"
    try:
        return run_cx([command])
    except Exception as exc:  # noqa: BLE001
        return f"error: {exc}"


@mcp.tool()
def overlay_ge_sn(
    ge_xyz: str,
    sn_xyz: str,
    *,
    snapshot: str | None = None,
) -> str:
    """Open Ge and Sn sibling XYZs together, tile, color distinctly, optional PNG.

    Ideal for reviewing ``demo/ge-sn-batch`` pairs before Olympus DFT.
    """
    ge = Path(ge_xyz).expanduser().resolve()
    sn = Path(sn_xyz).expanduser().resolve()
    for p in (ge, sn):
        if not p.exists():
            return f"error: missing {p}"
    try:
        ge_o = _structure_path_for_chimerax(ge)
        sn_o = _structure_path_for_chimerax(sn)
    except Exception as exc:  # noqa: BLE001
        return f"error converting: {exc}"
    cmds = [
        "close session",
        f"open {shlex.quote(str(ge_o))}",
        f"open {shlex.quote(str(sn_o))}",
        "tile",
        "style stick",
        "color byelement",
    ]
    if snapshot:
        snap = Path(snapshot).expanduser().resolve()
        snap.parent.mkdir(parents=True, exist_ok=True)
        # PNG needs OpenGL; prefer session save in headless, PNG when REST/GUI.
        if snap.suffix.lower() == ".png" and not _rest_url():
            cmds.append(f"save {shlex.quote(str(snap.with_suffix('.cxs')))}")
        else:
            cmds.append(f"save {shlex.quote(str(snap))}")
    try:
        log = run_cx(cmds)
        return f"overlay Ge={ge.name} Sn={sn.name}\n{log}"
    except Exception as exc:  # noqa: BLE001
        return f"error: {exc}"


@mcp.tool()
def save_snapshot(path: str, *, width: int = 1200, height: int = 900) -> str:
    """Save a PNG snapshot of the current ChimeraX scene (CLI batch session).

    For CLI mode this opens a temporary empty session unless you chain via
    run_command after open_structure in REST mode. Prefer ``overlay_ge_sn``
    with snapshot= for one-shot captures.
    """
    p = Path(path).expanduser().resolve()
    p.parent.mkdir(parents=True, exist_ok=True)
    cmds = [
        f"set bgColor white",
        f"save {shlex.quote(str(p))} width {width} height {height}",
    ]
    try:
        log = run_cx(cmds)
        return f"saved {p}\n{log}"
    except Exception as exc:  # noqa: BLE001
        return f"error: {exc}"


@mcp.tool()
def rest_start_instructions(port: int = 61886) -> str:
    """How to attach MCP to a live ChimeraX GUI via REST remotecontrol."""
    return (
        "In a running ChimeraX GUI, execute:\n\n"
        f"  remotecontrol rest start port {port} json true\n\n"
        "Then set:\n\n"
        f"  export CHIMERAX_REST_URL=http://127.0.0.1:{port}/run\n\n"
        "and restart this MCP server. Subsequent tools use the live session.\n"
        "Docs: ChimeraX remotecontrol rest (UCSF)."
    )


@mcp.tool()
def open_ge_sn_batch(
    batch_dir: str = "demo/ge-sn-batch",
    *,
    max_models: int = 10,
) -> str:
    """Open up to N Sn XYZs from the Ge→Sn batch directory."""
    d = Path(batch_dir).expanduser()
    if not d.is_absolute():
        # try repo-relative from cwd
        d = (Path.cwd() / d).resolve()
    xyz_dir = d / "xyz"
    if not xyz_dir.is_dir():
        return f"error: {xyz_dir} not found"
    files = sorted(xyz_dir.glob("*.xyz"))[:max_models]
    if not files:
        return f"error: no xyz in {xyz_dir}"
    cmds = ["close session"]
    opened: list[str] = []
    for f in files:
        try:
            op = _structure_path_for_chimerax(f)
        except Exception as exc:  # noqa: BLE001
            return f"error converting {f.name}: {exc}"
        cmds.append(f"open {shlex.quote(str(op))}")
        opened.append(f.name)
    cmds.append("tile")
    cmds.append("style stick")
    cmds.append("color byelement")
    try:
        log = run_cx(cmds)
        return (
            f"opened {len(opened)} models from {xyz_dir}\n"
            + "\n".join(opened)
            + f"\n{log}"
        )
    except Exception as exc:  # noqa: BLE001
        return f"error: {exc}"


def main() -> None:
    mcp.run()


if __name__ == "__main__":
    main()
