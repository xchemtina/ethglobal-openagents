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
