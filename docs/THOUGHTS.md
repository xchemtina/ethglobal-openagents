# Thoughts

These are working notes, not final doctrine.

## What feels structurally right

The core pattern is strong: everything becomes a signed artifact with explicit parents. That gives the project a single mental model across scientific work, procurement, optimization, and DAO governance.

The strongest demo path is not “an AI agent did chemistry” in the abstract. It is:

1. A planner creates a route artifact.
2. A quoter prices it.
3. A safety/procurement agent gates it.
4. A translator turns literature/database chemistry into an executable ADT.
5. A DAO can inspect lineage and decide what capabilities or funds to grant.

That is more credible than a black-box chat agent.

## Where the pressure is

The dangerous failure mode is breadth without a crisp artifact boundary. Every integration should answer:

- What artifact does it consume?
- What artifact does it produce?
- What schema tag proves that?
- What signature or capability gives it authority?
- What parent lineage must be preserved?

If an integration cannot answer those questions, it should remain outside the core.

## Chemistry-specific thoughts

ORD is a record format, not an execution plan. ADT is the bridge toward executable chemistry. The ORD→ADT bridge should stay conservative:

- preserve roles and provenance
- avoid inventing missing quantities silently
- make safety gates explicit
- keep product/outcome data for evaluation
- allow downstream chemistry agents to enrich the record

The next useful additions are not exotic models; they are dependable normalization, validation, and safety/procurement gates.

## ScienceClaw porting stance

ScienceClaw should be mined for skills and workflows, not adopted wholesale. The valuable pieces are chemistry/data science capability descriptions, not dependency sprawl.

Priority skills should support:

- molecule parsing and canonicalization
- PubChem/ChEMBL/CAS lookup
- chemical safety
- retrosynthesis
- DFT job creation and parsing
- docking/materials tasks later
- data cleaning and provenance

## DAO substrate thoughts

If ChimiaClaw becomes the DAO substrate, the DAO is not just voting UI. It is a running epistemic machine:

- agents make claims
- artifacts preserve evidence
- reputation weights future authority
- governance grants capabilities
- treasury actions are linked to scientific trace

That makes reputation more meaningful than a token balance alone.

## Demo thoughts

The submission video should show lineage more than code:

- print JSON artifact IDs and parents
- show tags changing across the DAG
- show ORD input becoming ADT output
- show a governance or capability anchor
- emphasize deterministic verification

The story: ChimiaDAO can run scientific work as auditable, composable, signed computation.
## MolADT geometry tiers and what they're worth
The tiered geometry story is now concrete and visible at `demo/molecules/`:
- the curated library is hand-built schematic geometry, perfect for visualization and connectivity sanity, useless for energies;
- `chimiaclaw_moladt::geometry::guess_coordinates` is a covalent-radii BFS embed plus a few spring iterations — still schematic, but enough to surface obvious connectivity bugs in seconds;
- `rdkit-etkdgv3-mmff94` is a real conformer + force-field optimization that costs RDKit but produces something a DFT worker can re-optimize from cheaply.
The key honesty pressure is to keep `provenance.source_kind` truthful at every tier so downstream agents (and human reviewers) cannot accidentally treat a schematic as a DFT-ready geometry.
## ASKCOS / retrosynthesis pressure
The ChimiaClaw `askcos-retro` worker is intentionally minimal: one `template-relevance` POST per template set, no tree expansion, no in-stock filtering, no caching. The next pressure points are:
- expand from template relevance to the full ASKCOS tree-expansion endpoint so we get multi-step routes, not single-step precursor candidates;
- add an in-stock filter (eMolecules / ChemSpace / Sigma-Aldrich) so route proposals are filtered to commercially-available reagents before they reach `apps/retroquoter`;
- ✅ a content-hashed disk cache (`~/.cache/chimiaclaw/askcos` by default) deduplicates identical `(endpoint, target_smiles, sorted_template_sets, top_k)` requests; the signed artifact now carries an `AskcosCacheRecord { hit, key, path }` so consumers can tell at a glance whether a given suggestion was served fresh or replayed;
- keep refusing to fabricate routes when the endpoint isn't configured — the previous ScienceClaw scraper fallback is the wrong shape for a signed graph.
A real downstream test is: feed the ASKCOS suggestion artifact into `apps/retroquoter` and confirm that the route quote, the procurement receipt, the safety gate, and the eventual MolADT artifacts all chain back to a single signed retrosynthesis root.
## Worker pressure more generally
Every worker boundary should pass the same five questions D7 / THOUGHTS pose for integrations: input artifact, output artifact, schema tag, signature/capability, parent lineage. The MolADT worker passes by signing `chem.molecule.adt`; the ASKCOS worker passes by signing `chem.retrosynth.template_suggestions`; the ENS publisher passes by signing `identity.ens.publication`; the 0G uploader passes by signing `storage.zerog.upload`. The future Skala/PySCF DFT worker has to pass the same way — it consumes a `chem.dft.request`, emits a `chem.dft.result`, signs both.
## Stub-mode integrations as a CI/demo affordance
The 0G uploader's `ZEROG_STUB=1` mode is the right shape for any sponsor adapter whose real path needs a heavy external binary or paid network. Three properties matter:
- the receipt is **deterministic** — same file in, same root_hash and tx_hash out — so tests and demos are reproducible without hitting the network;
- the receipt is **clearly labelled** — `audit_notes` explicitly says STUB MODE, and `provenance.source_kind` reflects the stub path, so a downstream reviewer cannot mistake it for a real on-chain anchor;
- the boundary is **flip-by-env** — installing `0g-storage-client` and unsetting `ZEROG_STUB` switches to real upload with zero code change.
This pattern should generalize: future Uniswap quoting, AXL transit, and on-chain anchoring should all expose a stub tier so CI is honest and operator-credentialed paths are an env-var away.
## Write-side ENS: keep the key out of Rust, out of argv
ENS publication is the first write-side adapter we shipped. The pressure that drove its design is keeping the controller key off argv and out of any Rust process: the key reaches `web3.py` only via `ENS_WRITE_PRIVATE_KEY`, the worker refuses mainnet without `--allow-mainnet`, and idempotent skip-if-equal means re-running the publisher cannot churn the registry. The Rust adapter only consumes the worker's JSON output. This boundary should stay even if we later add a native Rust ENS client — the operator surface is what makes the audit story credible.
