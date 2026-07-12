//! Literature lane: payload-bound signed artifacts for the
//! Literature -> Retrosynthesis -> DFT pipeline.
//!
//! This crate owns the canonical payload schema for two artifact kinds:
//!
//! * `science.literature.ingest` -- a manifest of open-access papers fetched
//!   from arXiv / Crossref / ChemRxiv / OpenAlex / Unpaywall, naming the
//!   per-source identifiers, URLs, and licence strings.
//! * `science.literature.synthesis` -- a synthesis over those ingested papers
//!   carrying citation-grounded claims, optional reaction candidates, optional
//!   extracted molecule candidates with SMILES, and full model provenance.
//!
//! Every claim and candidate must reference a citation by index, and every
//! `evidence_span` must be non-empty. The constructors reject payloads that
//! violate these invariants so that downstream consumers (the dashboard,
//! `world-model verify`, the Retrosynthesis hand-off, MolADT-Bayes) never see
//! a literature artifact that points at thin air.

use chimiaclaw_artifact::{Artifact, ArtifactDraft, ArtifactError, ArtifactSigner, PayloadRef};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

// -------------------- schema tags / skill ids --------------------

/// Schema tag for a `science.literature.ingest` artifact.
pub const LITERATURE_INGEST_TAG: &str = "science.literature.ingest";
/// Schema tag for a `science.literature.synthesis` artifact.
pub const LITERATURE_SYNTHESIS_TAG: &str = "science.literature.synthesis";

/// Skill id for the ingest worker.
pub const LITERATURE_INGEST_SKILL: &str = "science.literature.ingest.v1";
/// Skill id for the synthesis worker.
pub const LITERATURE_SYNTHESIS_SKILL: &str = "science.literature.synthesis.v1";

/// Default agent id for the literature service.
pub const LITERATURE_AGENT: &str = "literature.service.chimiaclaw.eth";

// -------------------- sources / citations --------------------

/// Open-access source kind. Used as a typed discriminator on the worker side
/// so the artifact records exactly which API the paper came from.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiteratureSourceKind {
    Arxiv,
    Chemrxiv,
    Crossref,
    Openalex,
    Unpaywall,
    LocalPdf,
}

/// A reference to a single open-access source as captured by the ingest
/// worker before any extraction has happened. Stored inside the
/// `LiteratureIngestManifest`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiteratureSource {
    pub kind: LiteratureSourceKind,
    pub identifier: String,
    pub url: String,
    /// Verbatim licence string returned by the source API (e.g. `cc-by`,
    /// `cc-by-nc`, `arxiv-perpetual`). The ingest worker enforces a
    /// whitelist; this field captures the original string for audit.
    pub license_hint: String,
}

/// A citation that backs at least one extracted claim, candidate, or
/// reaction candidate in a `LiteratureSynthesis`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiteratureCitation {
    pub title: String,
    pub authors: Vec<String>,
    pub year: u16,
    pub doi: Option<String>,
    pub source_url: Option<String>,
    pub license: String,
    pub retrieved_at_unix: u64,
}

// -------------------- extracted artefacts --------------------

/// A single citation-grounded claim. The `source_citation_index` references
/// `LiteratureSynthesis::citations` by position.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtractedClaim {
    pub claim: String,
    pub evidence_span: String,
    pub source_citation_index: usize,
}

/// Functional role a candidate molecule plays in a synthesis.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoleculeRole {
    Target,
    Precursor,
    Catalyst,
    Reagent,
    Solvent,
    Byproduct,
    Other,
}

/// A molecule pulled out of a paper.
///
/// Older workers emitted validated SMILES. The current Python worker emits a
/// compact MolADT/Haskell-shaped structural molecule under `molecule` and no
/// SMILES at all. Both are accepted here so signed Literature artifacts remain
/// backward compatible while the downstream MolADT handoff catches up.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtractedMoleculeCandidate {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smiles: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub molecule: Option<serde_json::Value>,
    pub role: MoleculeRole,
    pub source_citation_index: usize,
    pub evidence_span: String,
}

/// A reaction candidate pulled out of a paper.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtractedReactionCandidate {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactants_smiles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub products_smiles: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction: Option<serde_json::Value>,
    /// Worker-supplied confidence in [0.0, 1.0]; the constructor does not
    /// reweight or threshold this value.
    pub confidence: f32,
    pub evidence_span: String,
    pub source_citation_index: usize,
}

// -------------------- model provenance --------------------

/// Which extraction harness produced the synthesis. The MLX local runtime is
/// the canonical Phase-1 default; the other variants exist so that future
/// swaps to Ollama, OpenRouter / OpenAI, the Clojure paper-RAG, or the
/// Recursive Language Model harness do not require a schema migration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LiteratureRuntime {
    MlxLocal,
    LocalOllama,
    Openrouter,
    Openai,
    ClojureRag,
    Rlm,
}

/// Provenance metadata for the model that produced the synthesis. Stored
/// payload-bound so an audit can reproduce the run end-to-end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelProvenance {
    pub runtime: LiteratureRuntime,
    pub model_id: String,
    pub model_version: Option<String>,
    /// Filesystem path to the model weights when relevant (e.g.
    /// `~/mlx-models/gemma-4-e4b-it-4bit`); `None` for hosted runtimes.
    pub model_path: Option<String>,
    pub temperature: f32,
    /// Hex digest (the worker uses Blake3) of the canonicalised prompt
    /// template plus tool-input bytes, so an audit can replay the exact
    /// prompt without storing it inline.
    pub prompt_hash: String,
    /// `true` only when the harness guarantees byte-identical output for the
    /// same input (temperature 0, fixed seed, no upstream randomness).
    pub deterministic: bool,
}

// -------------------- manifests --------------------

/// Payload sealed into a `science.literature.ingest` artifact. Records what
/// the ingest worker fetched, before any extraction has happened.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiteratureIngestManifest {
    pub query: String,
    pub sector: String,
    pub requested_at_unix: u64,
    pub max_papers: u32,
    pub sources: Vec<LiteratureSource>,
    /// Path on disk where the worker wrote PDFs / metadata; informational.
    pub local_dir: Option<String>,
    /// License whitelist applied during ingestion; e.g.
    /// `["cc-by", "cc-by-sa", "cc0", "arxiv-perpetual"]`.
    pub license_whitelist: Vec<String>,
}

/// Payload sealed into a `science.literature.synthesis` artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiteratureSynthesis {
    pub query: String,
    pub sector: String,
    pub summary: String,
    pub citations: Vec<LiteratureCitation>,
    pub extracted_claims: Vec<ExtractedClaim>,
    pub conflicts: Vec<String>,
    pub molecule_candidates: Vec<ExtractedMoleculeCandidate>,
    pub reaction_candidates: Vec<ExtractedReactionCandidate>,
    pub model_provenance: ModelProvenance,
}

// -------------------- errors --------------------

#[derive(Debug)]
pub enum LiteratureError {
    Artifact(ArtifactError),
    EmptySummary,
    EmptyCitations,
    EmptyEvidenceSpan,
    EmptySmiles,
    MissingMoleculeRepresentation,
    CitationIndexOutOfRange { index: usize, citations: usize },
    ConfidenceOutOfRange(f32),
    PromptHashEmpty,
}

impl Display for LiteratureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
            Self::EmptySummary => write!(f, "synthesis summary must be non-empty"),
            Self::EmptyCitations => {
                write!(f, "literature synthesis must carry at least one citation")
            }
            Self::EmptyEvidenceSpan => {
                write!(
                    f,
                    "every claim or candidate must carry a non-empty evidence_span"
                )
            }
            Self::EmptySmiles => write!(f, "extracted molecule SMILES must be non-empty"),
            Self::MissingMoleculeRepresentation => {
                write!(f, "extracted molecule must carry either smiles or molecule")
            }
            Self::CitationIndexOutOfRange { index, citations } => write!(
                f,
                "citation index {index} is out of range for {citations} citation(s)"
            ),
            Self::ConfidenceOutOfRange(value) => {
                write!(f, "reaction confidence {value} must lie in [0.0, 1.0]")
            }
            Self::PromptHashEmpty => write!(f, "ModelProvenance.prompt_hash must be non-empty"),
        }
    }
}

impl std::error::Error for LiteratureError {}

impl From<ArtifactError> for LiteratureError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

// -------------------- verifier helpers --------------------

/// Verify the structural invariants of a `LiteratureSynthesis`. Returns
/// `Ok(())` if the payload is publishable.
pub fn verify_synthesis_payload(synthesis: &LiteratureSynthesis) -> Result<(), LiteratureError> {
    if synthesis.summary.trim().is_empty() {
        return Err(LiteratureError::EmptySummary);
    }
    if synthesis.citations.is_empty() {
        return Err(LiteratureError::EmptyCitations);
    }
    if synthesis.model_provenance.prompt_hash.trim().is_empty() {
        return Err(LiteratureError::PromptHashEmpty);
    }
    let citation_count = synthesis.citations.len();
    for claim in &synthesis.extracted_claims {
        if claim.evidence_span.trim().is_empty() {
            return Err(LiteratureError::EmptyEvidenceSpan);
        }
        check_citation_index(claim.source_citation_index, citation_count)?;
    }
    for candidate in &synthesis.molecule_candidates {
        if candidate.evidence_span.trim().is_empty() {
            return Err(LiteratureError::EmptyEvidenceSpan);
        }
        match candidate.smiles.as_ref() {
            Some(smiles) if smiles.trim().is_empty() => return Err(LiteratureError::EmptySmiles),
            Some(_) => {}
            None if candidate.molecule.is_none() => {
                return Err(LiteratureError::MissingMoleculeRepresentation);
            }
            None => {}
        }
        check_citation_index(candidate.source_citation_index, citation_count)?;
    }
    for reaction in &synthesis.reaction_candidates {
        if reaction.evidence_span.trim().is_empty() {
            return Err(LiteratureError::EmptyEvidenceSpan);
        }
        if !(0.0..=1.0).contains(&reaction.confidence) {
            return Err(LiteratureError::ConfidenceOutOfRange(reaction.confidence));
        }
        check_citation_index(reaction.source_citation_index, citation_count)?;
    }
    Ok(())
}

/// Verify the structural invariants of a `LiteratureIngestManifest`.
pub fn verify_ingest_manifest(manifest: &LiteratureIngestManifest) -> Result<(), LiteratureError> {
    if manifest.sources.is_empty() {
        return Err(LiteratureError::EmptyCitations);
    }
    Ok(())
}

fn check_citation_index(index: usize, citations: usize) -> Result<(), LiteratureError> {
    if index >= citations {
        Err(LiteratureError::CitationIndexOutOfRange { index, citations })
    } else {
        Ok(())
    }
}

// -------------------- artifact constructors --------------------

/// Seal an `science.literature.ingest` artifact from a manifest.
pub fn ingest_manifest_artifact(
    manifest: &LiteratureIngestManifest,
    agent: AgentId,
    signer: &ArtifactSigner,
    parent_request_artifact: Option<chimiaclaw_artifact::ArtifactId>,
    created_at_unix: u64,
) -> Result<Artifact, LiteratureError> {
    verify_ingest_manifest(manifest)?;
    let parent_artifact_ids = parent_request_artifact.into_iter().collect();
    ArtifactDraft {
        skill: SkillId(LITERATURE_INGEST_SKILL.to_string()),
        agent,
        topic: format!(
            "Literature ingest: {} ({} source{})",
            manifest.query,
            manifest.sources.len(),
            if manifest.sources.len() == 1 { "" } else { "s" },
        ),
        input_fingerprint: format!(
            "literature:ingest:{}:{}:{}",
            manifest.query, manifest.sector, manifest.max_papers
        ),
        output_cid: None,
        parent_artifact_ids,
        schema_tags: BTreeSet::from([SchemaTag(LITERATURE_INGEST_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(manifest)?),
    }
    .seal(signer, created_at_unix)
    .map_err(LiteratureError::Artifact)
}

/// Seal a `science.literature.synthesis` artifact from a synthesis payload.
///
/// `parent_artifact_ids` should typically include the corresponding
/// `science.literature.ingest` artifact id and, if applicable, the upstream
/// `science.literature.service_request` artifact id.
pub fn synthesis_artifact(
    synthesis: &LiteratureSynthesis,
    agent: AgentId,
    signer: &ArtifactSigner,
    parent_artifact_ids: Vec<chimiaclaw_artifact::ArtifactId>,
    created_at_unix: u64,
) -> Result<Artifact, LiteratureError> {
    verify_synthesis_payload(synthesis)?;
    ArtifactDraft {
        skill: SkillId(LITERATURE_SYNTHESIS_SKILL.to_string()),
        agent,
        topic: format!(
            "Literature synthesis: {} ({} citation{}, {} molecule{}, {} reaction{})",
            synthesis.query,
            synthesis.citations.len(),
            if synthesis.citations.len() == 1 {
                ""
            } else {
                "s"
            },
            synthesis.molecule_candidates.len(),
            if synthesis.molecule_candidates.len() == 1 {
                ""
            } else {
                "s"
            },
            synthesis.reaction_candidates.len(),
            if synthesis.reaction_candidates.len() == 1 {
                ""
            } else {
                "s"
            },
        ),
        input_fingerprint: format!(
            "literature:synthesis:{}:{}:{}:{}",
            synthesis.query,
            synthesis.sector,
            synthesis.citations.len(),
            synthesis.model_provenance.prompt_hash,
        ),
        output_cid: None,
        parent_artifact_ids,
        schema_tags: BTreeSet::from([SchemaTag(LITERATURE_SYNTHESIS_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(synthesis)?),
    }
    .seal(signer, created_at_unix)
    .map_err(LiteratureError::Artifact)
}

// -------------------- tests --------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn signer() -> ArtifactSigner {
        ArtifactSigner::from_seed([19; 32])
    }

    fn agent() -> AgentId {
        AgentId(LITERATURE_AGENT.to_string())
    }

    fn sample_manifest() -> LiteratureIngestManifest {
        LiteratureIngestManifest {
            query: "main-group hypovalent carbenoids".to_string(),
            sector: "automated-synthesis".to_string(),
            requested_at_unix: 1_700_000_000,
            max_papers: 6,
            sources: vec![
                LiteratureSource {
                    kind: LiteratureSourceKind::Arxiv,
                    identifier: "2401.00001".to_string(),
                    url: "https://arxiv.org/abs/2401.00001".to_string(),
                    license_hint: "arxiv-perpetual".to_string(),
                },
                LiteratureSource {
                    kind: LiteratureSourceKind::Crossref,
                    identifier: "10.1000/example".to_string(),
                    url: "https://doi.org/10.1000/example".to_string(),
                    license_hint: "cc-by".to_string(),
                },
            ],
            local_dir: Some("demo/overnight-full-out/literature/ingest/".to_string()),
            license_whitelist: vec![
                "cc-by".to_string(),
                "cc-by-sa".to_string(),
                "cc0".to_string(),
                "arxiv-perpetual".to_string(),
            ],
        }
    }

    fn sample_provenance() -> ModelProvenance {
        ModelProvenance {
            runtime: LiteratureRuntime::MlxLocal,
            model_id: "gemma-4-e4b-it-4bit".to_string(),
            model_version: Some("1".to_string()),
            model_path: Some("~/mlx-models/gemma-4-e4b-it-4bit".to_string()),
            temperature: 0.0,
            prompt_hash: "blake3:0123456789abcdef".to_string(),
            deterministic: true,
        }
    }

    fn sample_synthesis() -> LiteratureSynthesis {
        LiteratureSynthesis {
            query: "main-group hypovalent carbenoids".to_string(),
            sector: "automated-synthesis".to_string(),
            summary: "Carbenoids of Ge / Sn / Si stabilised by amido ligands\
                appear in three independent open-access reports."
                .to_string(),
            citations: vec![LiteratureCitation {
                title: "Stable diaminogermylene".to_string(),
                authors: vec!["Doe, J.".to_string()],
                year: 2024,
                doi: Some("10.1000/example".to_string()),
                source_url: Some("https://doi.org/10.1000/example".to_string()),
                license: "cc-by".to_string(),
                retrieved_at_unix: 1_700_000_000,
            }],
            extracted_claims: vec![ExtractedClaim {
                claim: "Diaminogermylene is isolable at room temperature".to_string(),
                evidence_span: "the diaminogermylene was isolated as a yellow solid".to_string(),
                source_citation_index: 0,
            }],
            conflicts: vec![],
            molecule_candidates: vec![ExtractedMoleculeCandidate {
                name: "diaminogermylene".to_string(),
                smiles: Some("N([H])[Ge]N([H])".to_string()),
                molecule: None,
                role: MoleculeRole::Target,
                source_citation_index: 0,
                evidence_span: "the diaminogermylene was isolated".to_string(),
            }],
            reaction_candidates: vec![],
            model_provenance: sample_provenance(),
        }
    }

    #[test]
    fn ingest_manifest_artifact_round_trips() {
        let manifest = sample_manifest();
        let artifact = ingest_manifest_artifact(&manifest, agent(), &signer(), None, 1)
            .expect("seal manifest");
        artifact.verify().expect("signature");
        artifact
            .verify_payload_value(&manifest)
            .expect("payload binding holds");
        assert!(artifact
            .schema_tags
            .contains(&SchemaTag(LITERATURE_INGEST_TAG.to_string())));
    }

    #[test]
    fn synthesis_artifact_round_trips() {
        let synthesis = sample_synthesis();
        let artifact =
            synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 2).expect("seal");
        artifact.verify().expect("signature");
        artifact
            .verify_payload_value(&synthesis)
            .expect("payload binding holds");
        assert!(artifact
            .schema_tags
            .contains(&SchemaTag(LITERATURE_SYNTHESIS_TAG.to_string())));
    }

    #[test]
    fn synthesis_with_parents_records_lineage() {
        let manifest = sample_manifest();
        let ingest =
            ingest_manifest_artifact(&manifest, agent(), &signer(), None, 1).expect("ingest");
        let synthesis = sample_synthesis();
        let artifact =
            synthesis_artifact(&synthesis, agent(), &signer(), vec![ingest.id.clone()], 2)
                .expect("synthesis");
        assert!(artifact.has_parent(&ingest.id));
    }

    #[test]
    fn deterministic_input_yields_byte_identical_artifact() {
        let synthesis = sample_synthesis();
        let a = synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 99).expect("seal a");
        let b = synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 99).expect("seal b");
        assert_eq!(a.id, b.id);
        assert_eq!(a.content_hash, b.content_hash);
        assert_eq!(a.signature, b.signature);
    }

    #[test]
    fn rejects_empty_summary() {
        let mut synthesis = sample_synthesis();
        synthesis.summary = "   ".to_string();
        match synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 1) {
            Err(LiteratureError::EmptySummary) => {}
            other => panic!("expected EmptySummary, got {other:?}"),
        }
    }

    #[test]
    fn rejects_no_citations() {
        let mut synthesis = sample_synthesis();
        synthesis.citations.clear();
        // Drop dependents that reference citations[0] so we hit the citation
        // check before the per-claim index check.
        synthesis.extracted_claims.clear();
        synthesis.molecule_candidates.clear();
        match synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 1) {
            Err(LiteratureError::EmptyCitations) => {}
            other => panic!("expected EmptyCitations, got {other:?}"),
        }
    }

    #[test]
    fn rejects_out_of_range_citation_index() {
        let mut synthesis = sample_synthesis();
        synthesis.extracted_claims[0].source_citation_index = 99;
        match synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 1) {
            Err(LiteratureError::CitationIndexOutOfRange {
                index: 99,
                citations: 1,
            }) => {}
            other => panic!("expected CitationIndexOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_evidence_span() {
        let mut synthesis = sample_synthesis();
        synthesis.extracted_claims[0].evidence_span = String::new();
        match synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 1) {
            Err(LiteratureError::EmptyEvidenceSpan) => {}
            other => panic!("expected EmptyEvidenceSpan, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_molecule_smiles() {
        let mut synthesis = sample_synthesis();
        synthesis.molecule_candidates[0].smiles = Some("  ".to_string());
        match synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 1) {
            Err(LiteratureError::EmptySmiles) => {}
            other => panic!("expected EmptySmiles, got {other:?}"),
        }
    }

    #[test]
    fn rejects_out_of_range_confidence() {
        let mut synthesis = sample_synthesis();
        synthesis
            .reaction_candidates
            .push(ExtractedReactionCandidate {
                reactants_smiles: vec!["CCO".to_string()],
                products_smiles: vec!["CCOC".to_string()],
                conditions_text: Some("neat, 80 degC".to_string()),
                reaction: None,
                confidence: 1.5,
                evidence_span: "ethanol was etherified".to_string(),
                source_citation_index: 0,
            });
        match synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 1) {
            Err(LiteratureError::ConfidenceOutOfRange(c)) if (c - 1.5).abs() < 1e-6 => {}
            other => panic!("expected ConfidenceOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_prompt_hash() {
        let mut synthesis = sample_synthesis();
        synthesis.model_provenance.prompt_hash = String::new();
        match synthesis_artifact(&synthesis, agent(), &signer(), Vec::new(), 1) {
            Err(LiteratureError::PromptHashEmpty) => {}
            other => panic!("expected PromptHashEmpty, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_ingest_manifest() {
        let mut manifest = sample_manifest();
        manifest.sources.clear();
        match ingest_manifest_artifact(&manifest, agent(), &signer(), None, 1) {
            Err(LiteratureError::EmptyCitations) => {}
            other => panic!("expected EmptyCitations, got {other:?}"),
        }
    }
}
