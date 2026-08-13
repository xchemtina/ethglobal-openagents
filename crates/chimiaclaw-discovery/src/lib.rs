//! Typed, bounded discovery dossiers for report-only scientific review.
//!
//! This crate deliberately does not run models, fetch sources, open a database,
//! dispatch work, or make scientific decisions. It validates inert dossier data,
//! binds that data to a signed `OpenAgents` artifact, and verifies an inline
//! artifact payload before exporting the same canonical JSON for independent
//! review by Zopf.

mod artifact;
mod model;
mod strict_json;
mod validation;

pub use artifact::{
    decode_artifact, dossier_artifact_draft, seal_dossier, verify_export, verify_export_bytes,
};
pub use model::{
    Claim, ClaimStatus, Conflict, DiscoveryDossier, DiscoveryError, DossierAuthority, Governance,
    Hypothesis, ProposedTest, Provenance, Source, SourceKind, Support, DISCOVERY_SKILL,
    DOSSIER_SCHEMA, MAX_DOSSIER_BYTES, MAX_JSON_DEPTH, MAX_JSON_NODES,
};
pub use strict_json::parse_strict_json;
pub use validation::{
    canonical_dossier_bytes, decode_dossier, normalized_sha256, validate_dossier,
};

#[cfg(test)]
mod tests;
