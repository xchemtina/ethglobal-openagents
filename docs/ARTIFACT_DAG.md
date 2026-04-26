# Artifact DAG

The artifact DAG is the shared state model for ChimiaClaw.

## Artifact fields

An artifact records:

- skill ID
- agent ID
- topic
- input fingerprint
- optional output CID
- parent artifact IDs
- schema tags
- content hash
- signing public key
- signature
- creation timestamp

## Invariants

- Artifacts are immutable once sealed.
- Hashes and IDs are deterministic over canonical content.
- Signatures verify the sealed content.
- Parent IDs encode lineage.
- State transitions create child artifacts rather than mutating parents.

## Example DAGs

```mermaid
flowchart TD
    RP[chem.retrosynth.route_proposal] --> RQ[chem.procurement.route_quote]
    RQ --> PR[chem.procurement.procured]

    OR[chem.ord.reaction] --> ADT[chem.adt.reaction]
    ADT --> SAFE[chem.safety.assessment]
    SAFE --> JOB[chem.exec.job]
```

## Why this matters

The DAG lets agents work independently while preserving auditability. A downstream agent does not need to trust the upstream agent’s process; it can verify signatures, schema tags, parent lineage, and output hashes.

## Future store layers

```mermaid
flowchart LR
    Local[(Local file store)] --> Root[Artifact root]
    Remote[(0G / decentralized storage)] --> Root
    Root --> Contract[On-chain anchor]
    Contract --> Governance[DAO decisions]
    Root --> RDF[World Avatar RDF projection]
```

Local stores should support fast development. Storage adapters and contracts should anchor selected roots for public verification.
