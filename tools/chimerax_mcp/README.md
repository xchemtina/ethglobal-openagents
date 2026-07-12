# ChimeraX MCP

Model Context Protocol server for **UCSF ChimeraX** — preferred here for
Ge/Sn atrane overlays, multi-model tile layouts, and (later) cube volumes.

Avogadro remains fine for quick sketch/edit; ChimeraX wins for session
scripting + publication views.

## Install

```bash
cd tools/chimerax_mcp
uv sync
# or: pip install -e .
```

Requires ChimeraX installed. Default binary:

```text
/Applications/ChimeraX-1.11.1.app/Contents/bin/ChimeraX
```

Override with `CHIMERAX_BIN`.

## Run (stdio MCP)

```bash
uv run chimerax-mcp
# or
uv run python -m chimerax_mcp
```

### Claude / MCP client config sketch

```json
{
  "mcpServers": {
    "chimerax": {
      "command": "uv",
      "args": ["--directory", "/Users/crischimiadao/OpenAgents/tools/chimerax_mcp", "run", "chimerax-mcp"],
      "env": {
        "CHIMERAX_BIN": "/Applications/ChimeraX-1.11.1.app/Contents/bin/ChimeraX"
      }
    }
  }
}
```

## Tools

| Tool | Purpose |
|------|---------|
| `chimerax_info` | Binary / version / REST status |
| `open_structure` | Open xyz/pdb/mol2/… |
| `run_command` | Arbitrary ChimeraX command |
| `overlay_ge_sn` | Side-by-side Ge vs Sn pair + optional PNG |
| `open_ge_sn_batch` | Open `demo/ge-sn-batch/xyz/*.xyz` tiled |
| `save_snapshot` | PNG export |
| `rest_start_instructions` | Wire live GUI REST |

## Live GUI (REST)

In ChimeraX:

```text
remotecontrol rest start port 61886 json true
```

```bash
export CHIMERAX_REST_URL=http://127.0.0.1:61886/run
```

## Demo

```bash
# one-shot CLI (no MCP client needed)
$CHIMERAX_BIN --nogui --offscreen --cmd \
  "open demo/ge-sn-batch/xyz/NC3Sn_H.xyz; style stick; color byelement; save /tmp/nc3sn_h.png" \
  --exit
```
