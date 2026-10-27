# Optional external molecule visualizer

The Molecule Visualizer MCP server is an optional external project:

- Upstream: https://github.com/leelasd/molecule_mcp
- Previously recorded revision: `92a0959b2e13389726a8bb427c313a589f9e6efe`
- Exact upstream commit: https://github.com/leelasd/molecule_mcp/commit/92a0959b2e13389726a8bb427c313a589f9e6efe

The former Git submodule entry had no `.gitmodules` definition and broke checkout
credential cleanup. This pointer preserves its source identity without retaining
the orphaned entry. The visualizer source is not vendored or required by the
current OpenAgents build. Install and qualify it separately when needed.
