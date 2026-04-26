//! ORD-like reaction JSON to ChimiaDAO ADT reaction translation.
//!
//! The initial target is the minimal ADT schema from
//! `/Users/crischimiadao/Desktop/ChimiaDAO-OxAI_ADTHack/schema/adt.schema.json`
//! and the ORD-like JSON produced by that hackathon stack. The crate also
//! accepts a lightweight subset of official Open Reaction Database JSON as
//! emitted by `google.protobuf.json_format.MessageToJson(...,
//! preserving_proto_field_name=True)` without depending on protobuf bindings.

use chimiaclaw_artifact::{
    blake3_hex, canonical_bytes, Artifact, ArtifactDraft, ArtifactError, ArtifactSigner, PayloadRef,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const ORD_REACTION_TAG: &str = "chem.ord.reaction";
pub const ADT_REACTION_TAG: &str = "chem.adt.reaction";
pub const ORD_TO_ADT_SKILL: &str = "chem.ord.to_adt.v1";
pub const ORD_ADT_AGENT: &str = "ord-adt.chimiaclaw.eth";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrdLikeReaction {
    pub name: String,
    pub created_unix: u64,
    pub inputs: Vec<OrdInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auxiliary_samples: Vec<OrdInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub products: Vec<OrdInput>,
    pub setup: OrdSetup,
    pub conditions: OrdConditions,
    pub procedural_steps: Vec<AdtStep>,
    #[serde(default)]
    pub analyses: Vec<serde_json::Value>,
    #[serde(default)]
    pub provenance: BTreeMap<String, String>,
}

impl OrdLikeReaction {
    pub fn from_json_str(input: &str) -> Result<Self, OrdAdtError> {
        serde_json::from_str(input).map_err(|error| OrdAdtError::Json(error.to_string()))
    }

    pub fn from_official_ord_json_str(input: &str) -> Result<Self, OrdAdtError> {
        official_ord_json_to_ord_like(input)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrdInput {
    pub label: String,
    pub smiles: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_mmol: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<AdtPhase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yield_percent: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrdSetup {
    pub inert: bool,
    pub stir_rpm: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrdConditions {
    #[serde(rename = "temperature_C")]
    pub temperature_c: f64,
    pub time_min: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdtExperiment {
    pub metadata: AdtMetadata,
    pub samples: Vec<AdtSample>,
    pub reaction: AdtReaction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdtMetadata {
    pub title: String,
    pub version: String,
    pub authors: Vec<String>,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdtSample {
    pub id: String,
    pub label: String,
    pub smiles: String,
    pub amount_mmol: f64,
    pub phase: AdtPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_percent: Option<f64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AdtPhase {
    Solid,
    Liquid,
    Gas,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdtReaction {
    pub inputs: Vec<AdtReactionInput>,
    pub conditions: AdtReactionConditions,
    pub steps: Vec<AdtStep>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdtReactionInput {
    pub sample_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdtReactionConditions {
    #[serde(rename = "temperature_C")]
    pub temperature_c: f64,
    pub time_min: f64,
    pub inert: bool,
    pub stir_rpm: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdtStep {
    pub op: AdtOperation,
    #[serde(rename = "target_C", skip_serializing_if = "Option::is_none")]
    pub target_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold_min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reagent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyte: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AdtOperation {
    Charge,
    Heat,
    StirTo,
    Wait,
    Add,
    Quench,
    Measure,
    Purify,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrdToAdtOptions {
    pub version: String,
    pub authors: Vec<String>,
    pub default_amount_mmol: f64,
    pub default_phase: AdtPhase,
}

impl Default for OrdToAdtOptions {
    fn default() -> Self {
        Self {
            version: "0.1.0-chimiaclaw-ord-adt".to_string(),
            authors: vec!["ChimiaDAO".to_string()],
            default_amount_mmol: 0.0,
            default_phase: AdtPhase::Liquid,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum OrdAdtError {
    Json(String),
    Artifact(ArtifactError),
    InvalidOfficialOrdJson { reason: String },
    MissingOrdReactionTag { artifact_id: String },
    MissingReferencedReagent { reagent: String },
    PayloadUnavailable,
}

pub struct OrdToAdtTranslator {
    agent: AgentId,
    signer: ArtifactSigner,
    options: OrdToAdtOptions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignedAdtExperiment {
    pub experiment: AdtExperiment,
    pub artifact: Artifact,
}

impl OrdToAdtTranslator {
    #[must_use]
    pub fn new(agent: AgentId, signer: ArtifactSigner) -> Self {
        Self {
            agent,
            signer,
            options: OrdToAdtOptions::default(),
        }
    }

    #[must_use]
    pub fn with_options(mut self, options: OrdToAdtOptions) -> Self {
        self.options = options;
        self
    }

    pub fn translate(&self, ord: &OrdLikeReaction) -> Result<AdtExperiment, OrdAdtError> {
        translate_ord_like(ord, &self.options)
    }

    pub fn translate_official_ord_json(&self, input: &str) -> Result<AdtExperiment, OrdAdtError> {
        let ord = official_ord_json_to_ord_like(input)?;
        self.translate(&ord)
    }

    pub fn translate_and_seal(
        &self,
        ord_artifact: &Artifact,
        ord: &OrdLikeReaction,
        created_at_unix: u64,
    ) -> Result<SignedAdtExperiment, OrdAdtError> {
        validate_ord_artifact(ord_artifact)?;
        let experiment = self.translate(ord)?;
        let adt_hash = adt_experiment_hash(&experiment)?;
        let payload = PayloadRef::inline_json(&experiment).map_err(OrdAdtError::Artifact)?;
        let artifact = ArtifactDraft {
            skill: SkillId(ORD_TO_ADT_SKILL.to_string()),
            agent: self.agent.clone(),
            topic: format!("ADT reaction translated from {}", ord.name),
            input_fingerprint: blake3_hex(
                format!("{}:{adt_hash}", ord_artifact.content_hash).as_bytes(),
            ),
            output_cid: Some(format!("inline://chimiaclaw/adt/{adt_hash}")),
            parent_artifact_ids: vec![ord_artifact.id.clone()],
            schema_tags: BTreeSet::from([SchemaTag(ADT_REACTION_TAG.to_string())]),
            payload: Some(payload),
        }
        .seal(&self.signer, created_at_unix)
        .map_err(OrdAdtError::Artifact)?;

        Ok(SignedAdtExperiment {
            experiment,
            artifact,
        })
    }
}

/// Build a payload-bound `ArtifactDraft` from an ORD parent without sealing it.
///
/// The runtime that owns the signer is responsible for sealing the draft.
pub fn build_adt_draft(
    agent: &AgentId,
    ord_artifact: &Artifact,
    ord: &OrdLikeReaction,
    options: &OrdToAdtOptions,
) -> Result<(AdtExperiment, ArtifactDraft), OrdAdtError> {
    validate_ord_artifact(ord_artifact)?;
    let experiment = translate_ord_like(ord, options)?;
    let adt_hash = adt_experiment_hash(&experiment)?;
    let payload = PayloadRef::inline_json(&experiment).map_err(OrdAdtError::Artifact)?;
    let draft = ArtifactDraft {
        skill: SkillId(ORD_TO_ADT_SKILL.to_string()),
        agent: agent.clone(),
        topic: format!("ADT reaction translated from {}", ord.name),
        input_fingerprint: blake3_hex(
            format!("{}:{adt_hash}", ord_artifact.content_hash).as_bytes(),
        ),
        output_cid: Some(format!("inline://chimiaclaw/adt/{adt_hash}")),
        parent_artifact_ids: vec![ord_artifact.id.clone()],
        schema_tags: BTreeSet::from([SchemaTag(ADT_REACTION_TAG.to_string())]),
        payload: Some(payload),
    };
    Ok((experiment, draft))
}

/// Decode the ORD payload that an ORD reaction artifact commits to.
///
/// Currently only inline payloads are supported -- external CIDs would require
/// a fetcher adapter.
pub fn decode_ord_payload(artifact: &Artifact) -> Result<OrdLikeReaction, OrdAdtError> {
    let bytes = artifact
        .inline_payload_bytes()
        .map_err(OrdAdtError::Artifact)?
        .ok_or(OrdAdtError::PayloadUnavailable)?;
    artifact
        .verify_payload_bytes(&bytes)
        .map_err(OrdAdtError::Artifact)?;
    serde_json::from_slice(&bytes).map_err(|error| OrdAdtError::Json(error.to_string()))
}

/// Skill wrapper that turns the ORD→ADT translator into a `chimiaclaw-skill`
/// implementation, suitable for execution by `chimiaclaw-node`.
pub struct OrdToAdtSkill {
    options: OrdToAdtOptions,
}

impl OrdToAdtSkill {
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: OrdToAdtOptions::default(),
        }
    }

    #[must_use]
    pub fn with_options(options: OrdToAdtOptions) -> Self {
        Self { options }
    }
}

impl Default for OrdToAdtSkill {
    fn default() -> Self {
        Self::new()
    }
}

impl chimiaclaw_skill::Skill for OrdToAdtSkill {
    fn id(&self) -> SkillId {
        SkillId(ORD_TO_ADT_SKILL.to_string())
    }

    fn capabilities(&self) -> Vec<chimiaclaw_schema::Capability> {
        Vec::new()
    }

    fn consumes_tags(&self) -> Vec<SchemaTag> {
        vec![SchemaTag(ORD_REACTION_TAG.to_string())]
    }

    fn produces_tags(&self) -> Vec<SchemaTag> {
        vec![SchemaTag(ADT_REACTION_TAG.to_string())]
    }

    fn invoke(
        &self,
        ctx: &chimiaclaw_skill::SkillCtx,
        parents: &[Artifact],
    ) -> Result<ArtifactDraft, chimiaclaw_skill::SkillError> {
        let parent = parents.first().ok_or_else(|| {
            chimiaclaw_skill::SkillError::InvalidInput(
                "ord-adt skill requires exactly one parent ORD artifact".to_string(),
            )
        })?;
        let ord = decode_ord_payload(parent).map_err(|error| match error {
            OrdAdtError::Artifact(inner) => chimiaclaw_skill::SkillError::InvalidInput(format!(
                "failed to decode ORD payload: {inner:?}"
            )),
            OrdAdtError::Json(message) => chimiaclaw_skill::SkillError::InvalidInput(format!(
                "ORD payload was not valid JSON: {message}"
            )),
            OrdAdtError::PayloadUnavailable => chimiaclaw_skill::SkillError::InvalidInput(
                "ORD parent artifact did not carry an inline payload".to_string(),
            ),
            other => chimiaclaw_skill::SkillError::InvalidInput(format!(
                "unexpected ord-adt error: {other:?}"
            )),
        })?;
        let (_experiment, draft) = build_adt_draft(&ctx.agent, parent, &ord, &self.options)
            .map_err(|error| {
                chimiaclaw_skill::SkillError::Execution(format!("ord-adt translation: {error:?}"))
            })?;
        Ok(draft)
    }
}

pub fn validate_ord_artifact(artifact: &Artifact) -> Result<(), OrdAdtError> {
    artifact.verify().map_err(OrdAdtError::Artifact)?;
    if !artifact
        .schema_tags
        .contains(&SchemaTag(ORD_REACTION_TAG.to_string()))
    {
        return Err(OrdAdtError::MissingOrdReactionTag {
            artifact_id: artifact.id.0.clone(),
        });
    }
    Ok(())
}

pub fn translate_ord_like(
    ord: &OrdLikeReaction,
    options: &OrdToAdtOptions,
) -> Result<AdtExperiment, OrdAdtError> {
    let mut samples = Vec::new();
    let mut used_ids = BTreeSet::new();
    let mut reagent_lookup = BTreeMap::new();
    let mut reaction_input_ids = Vec::new();

    for (index, input) in ord.inputs.iter().enumerate() {
        let id = add_ord_sample(
            input,
            index + 1,
            None,
            &mut samples,
            &mut used_ids,
            &mut reagent_lookup,
            options,
        );
        reaction_input_ids.push(id);
    }
    for auxiliary in &ord.auxiliary_samples {
        add_ord_sample(
            auxiliary,
            samples.len() + 1,
            Some("WORKUP"),
            &mut samples,
            &mut used_ids,
            &mut reagent_lookup,
            options,
        );
    }
    for product in &ord.products {
        add_ord_sample(
            product,
            samples.len() + 1,
            Some("PRODUCT"),
            &mut samples,
            &mut used_ids,
            &mut reagent_lookup,
            options,
        );
    }

    let mut steps = ord.procedural_steps.clone();
    for step in &mut steps {
        if let Some(reagent) = step.reagent.clone() {
            let sample_id =
                ensure_reagent_sample(&reagent, &mut samples, &mut used_ids, &mut reagent_lookup)?;
            step.reagent = Some(sample_id);
        }
    }

    Ok(AdtExperiment {
        metadata: AdtMetadata {
            title: ord.name.clone(),
            version: options.version.clone(),
            authors: options.authors.clone(),
            notes: translation_notes(ord),
        },
        samples,
        reaction: AdtReaction {
            inputs: reaction_input_ids
                .into_iter()
                .map(|sample_id| AdtReactionInput { sample_id })
                .collect(),
            conditions: AdtReactionConditions {
                temperature_c: ord.conditions.temperature_c,
                time_min: ord.conditions.time_min,
                inert: ord.setup.inert,
                stir_rpm: ord.setup.stir_rpm,
            },
            steps,
        },
    })
}
fn add_ord_sample(
    input: &OrdInput,
    index: usize,
    fallback_role: Option<&str>,
    samples: &mut Vec<AdtSample>,
    used_ids: &mut BTreeSet<String>,
    reagent_lookup: &mut BTreeMap<String, String>,
    options: &OrdToAdtOptions,
) -> String {
    let id = unique_sample_id(&input.label, index, used_ids);
    reagent_lookup.insert(normalize_key(&id), id.clone());
    reagent_lookup.insert(normalize_key(&input.label), id.clone());
    samples.push(AdtSample {
        id: id.clone(),
        label: input.label.clone(),
        smiles: input.smiles.clone(),
        amount_mmol: input.amount_mmol.unwrap_or(options.default_amount_mmol),
        phase: input
            .phase
            .clone()
            .or_else(|| infer_phase(&input.label))
            .unwrap_or_else(|| options.default_phase.clone()),
        purity: input.purity,
        role: input
            .role
            .clone()
            .or_else(|| fallback_role.map(str::to_string)),
        yield_percent: input.yield_percent,
    });
    id
}

fn translation_notes(ord: &OrdLikeReaction) -> String {
    let mut notes = format!(
        "Translated from ORD-like reaction created_unix={} with {} provenance field(s).",
        ord.created_unix,
        ord.provenance.len()
    );
    if !ord.auxiliary_samples.is_empty() {
        notes.push_str(&format!(
            " Preserved {} auxiliary/workup sample(s).",
            ord.auxiliary_samples.len()
        ));
    }
    if !ord.products.is_empty() {
        notes.push_str(&format!(
            " Preserved {} outcome product sample(s).",
            ord.products.len()
        ));
    }
    notes
}

pub fn adt_experiment_hash(experiment: &AdtExperiment) -> Result<String, OrdAdtError> {
    canonical_bytes(experiment)
        .map(|bytes| blake3_hex(&bytes))
        .map_err(OrdAdtError::Artifact)
}

pub fn official_ord_json_to_ord_like(input: &str) -> Result<OrdLikeReaction, OrdAdtError> {
    let reaction: serde_json::Value =
        serde_json::from_str(input).map_err(|error| OrdAdtError::Json(error.to_string()))?;
    official_ord_value_to_ord_like(&reaction)
}

pub fn official_ord_value_to_ord_like(
    reaction: &serde_json::Value,
) -> Result<OrdLikeReaction, OrdAdtError> {
    if !reaction.is_object() {
        return Err(invalid_official_ord_json(
            "expected an ORD Reaction JSON object",
        ));
    }

    let reaction_id = string_field(reaction, "reaction_id");
    let reaction_smiles = identifier_value(
        reaction.get("identifiers"),
        &["REACTION_SMILES", "REACTION_CXSMILES"],
    );
    let name = reaction_id
        .as_ref()
        .map(|id| format!("ORD reaction {id}"))
        .or_else(|| {
            reaction_smiles
                .as_ref()
                .map(|smiles| format!("ORD reaction {smiles}"))
        })
        .unwrap_or_else(|| "ORD reaction".to_string());
    let created_unix = official_created_unix(reaction.get("provenance"));
    let provenance = collect_official_provenance(reaction, reaction_smiles.as_deref());

    let mut inputs = Vec::new();
    append_official_inputs(reaction.get("inputs"), &mut inputs);
    if inputs.is_empty() {
        return Err(invalid_official_ord_json(
            "reaction did not contain any input components",
        ));
    }

    let condition_summary = official_condition_summary(reaction);
    let mut auxiliary_samples = Vec::new();
    let mut products = Vec::new();
    let mut analyses = Vec::new();
    let mut analysis_methods = BTreeSet::new();
    let mut workup_steps = Vec::new();

    append_official_workups(
        reaction.get("workups"),
        &mut auxiliary_samples,
        &mut workup_steps,
    );

    let mut outcome_time_min = None;
    append_official_outcomes(
        reaction.get("outcomes"),
        &mut products,
        &mut analyses,
        &mut analysis_methods,
        &mut outcome_time_min,
    );

    let temperature_c = condition_summary.temperature_c.unwrap_or(25.0);
    let time_min = outcome_time_min
        .or(condition_summary.time_min)
        .unwrap_or(0.0);
    let hold_min = positive_duration(time_min);
    let mut procedural_steps = vec![AdtStep {
        op: AdtOperation::Charge,
        target_c: None,
        hold_min: None,
        rpm: None,
        reagent: None,
        mode: None,
        analyte: None,
        method: None,
    }];
    if condition_summary.temperature_c.is_some() {
        procedural_steps.push(AdtStep {
            op: AdtOperation::Heat,
            target_c: Some(temperature_c),
            hold_min,
            rpm: None,
            reagent: None,
            mode: None,
            analyte: None,
            method: None,
        });
    }
    if condition_summary.stir_rpm > 0 {
        procedural_steps.push(AdtStep {
            op: AdtOperation::StirTo,
            target_c: None,
            hold_min,
            rpm: Some(condition_summary.stir_rpm),
            reagent: None,
            mode: None,
            analyte: None,
            method: None,
        });
    }
    if procedural_steps.len() == 1 && hold_min.is_some() {
        procedural_steps.push(AdtStep {
            op: AdtOperation::Wait,
            target_c: None,
            hold_min,
            rpm: None,
            reagent: None,
            mode: None,
            analyte: None,
            method: None,
        });
    }
    procedural_steps.extend(workup_steps);
    if let Some(method) = analysis_methods.into_iter().next() {
        procedural_steps.push(AdtStep {
            op: AdtOperation::Measure,
            target_c: None,
            hold_min: None,
            rpm: None,
            reagent: None,
            mode: None,
            analyte: Some("product".to_string()),
            method: Some(method),
        });
    }

    Ok(OrdLikeReaction {
        name,
        created_unix,
        inputs,
        auxiliary_samples,
        products,
        setup: OrdSetup {
            inert: condition_summary.inert,
            stir_rpm: condition_summary.stir_rpm,
        },
        conditions: OrdConditions {
            temperature_c,
            time_min,
        },
        procedural_steps,
        analyses,
        provenance,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct OfficialConditionSummary {
    temperature_c: Option<f64>,
    time_min: Option<f64>,
    inert: bool,
    stir_rpm: u64,
}

fn invalid_official_ord_json(reason: &str) -> OrdAdtError {
    OrdAdtError::InvalidOfficialOrdJson {
        reason: reason.to_string(),
    }
}

fn append_official_inputs(inputs: Option<&serde_json::Value>, target: &mut Vec<OrdInput>) {
    let Some(input_map) = inputs.and_then(serde_json::Value::as_object) else {
        return;
    };
    let mut labels: Vec<_> = input_map.keys().collect();
    labels.sort();
    for label in labels {
        if let Some(input) = input_map.get(label) {
            append_components_from_reaction_input(label, input, None, target);
        }
    }
}

fn append_official_workups(
    workups: Option<&serde_json::Value>,
    auxiliary_samples: &mut Vec<OrdInput>,
    workup_steps: &mut Vec<AdtStep>,
) {
    let Some(workups) = workups.and_then(serde_json::Value::as_array) else {
        return;
    };
    for workup in workups {
        let workup_type = enum_field(workup, "type").unwrap_or_else(|| "CUSTOM".to_string());
        let details = string_field(workup, "details").unwrap_or_default();
        let mut labels = Vec::new();
        if let Some(input) = workup.get("input") {
            labels = append_components_from_reaction_input(
                details.as_str(),
                input,
                Some("WORKUP"),
                auxiliary_samples,
            );
        }
        match normalize_key(&workup_type).as_str() {
            "addition" | "wash" | "ph-adjust" | "dissolution" => {
                let reagent = labels
                    .first()
                    .cloned()
                    .or_else(|| (!details.is_empty()).then_some(details.clone()));
                workup_steps.push(AdtStep {
                    op: if normalize_key(&details).contains("quench")
                        || reagent
                            .as_ref()
                            .is_some_and(|label| normalize_key(label).contains("water"))
                    {
                        AdtOperation::Quench
                    } else {
                        AdtOperation::Add
                    },
                    target_c: None,
                    hold_min: None,
                    rpm: None,
                    reagent,
                    mode: None,
                    analyte: None,
                    method: None,
                });
            }
            "filtration" | "flash-chromatography" | "other-chromatography" => {
                workup_steps.push(AdtStep {
                    op: AdtOperation::Purify,
                    target_c: None,
                    hold_min: None,
                    rpm: None,
                    reagent: None,
                    mode: Some(workup_type),
                    analyte: None,
                    method: None,
                });
            }
            "wait" => {
                let hold_min = workup.get("duration").and_then(extract_time_min);
                workup_steps.push(AdtStep {
                    op: AdtOperation::Wait,
                    target_c: None,
                    hold_min,
                    rpm: None,
                    reagent: None,
                    mode: None,
                    analyte: None,
                    method: None,
                });
            }
            "temperature" => {
                let target_c = workup
                    .get("temperature")
                    .and_then(|value| value.get("setpoint"))
                    .and_then(extract_temperature_c);
                workup_steps.push(AdtStep {
                    op: AdtOperation::Heat,
                    target_c,
                    hold_min: None,
                    rpm: None,
                    reagent: None,
                    mode: None,
                    analyte: None,
                    method: None,
                });
            }
            "stirring" => {
                let rpm = workup
                    .get("stirring")
                    .and_then(|value| value.get("rate"))
                    .and_then(|value| u64_field(value, "rpm"));
                workup_steps.push(AdtStep {
                    op: AdtOperation::StirTo,
                    target_c: None,
                    hold_min: None,
                    rpm,
                    reagent: None,
                    mode: None,
                    analyte: None,
                    method: None,
                });
            }
            _ => {}
        }
    }
}

fn append_official_outcomes(
    outcomes: Option<&serde_json::Value>,
    products: &mut Vec<OrdInput>,
    analyses: &mut Vec<serde_json::Value>,
    analysis_methods: &mut BTreeSet<String>,
    outcome_time_min: &mut Option<f64>,
) {
    let Some(outcomes) = outcomes.and_then(serde_json::Value::as_array) else {
        return;
    };
    for outcome in outcomes {
        if outcome_time_min.is_none() {
            *outcome_time_min = outcome.get("reaction_time").and_then(extract_time_min);
        }
        if let Some(product_values) = outcome
            .get("products")
            .and_then(serde_json::Value::as_array)
        {
            for (index, product) in product_values.iter().enumerate() {
                if let Some(product) = product_to_ord_input(product, index + 1) {
                    products.push(product);
                }
            }
        }
        collect_analysis_methods(outcome.get("analyses"), analyses, analysis_methods);
    }
}

fn append_components_from_reaction_input(
    input_label: &str,
    input: &serde_json::Value,
    role_fallback: Option<&str>,
    target: &mut Vec<OrdInput>,
) -> Vec<String> {
    let mut labels = Vec::new();
    let fallback = role_fallback.or_else(|| role_from_label(input_label));
    let input_phase = input.get("texture").and_then(phase_from_texture);
    let Some(components) = input
        .get("components")
        .and_then(serde_json::Value::as_array)
    else {
        return labels;
    };
    for (index, component) in components.iter().enumerate() {
        let label_hint = if components.len() == 1 {
            input_label.to_string()
        } else {
            format!("{input_label} component {}", index + 1)
        };
        if let Some(mut ord_input) = compound_to_ord_input(component, &label_hint, fallback) {
            if ord_input.phase.is_none() {
                ord_input.phase = input_phase.clone();
            }
            labels.push(ord_input.label.clone());
            target.push(ord_input);
        }
    }
    labels
}

fn compound_to_ord_input(
    compound: &serde_json::Value,
    label_hint: &str,
    fallback_role: Option<&str>,
) -> Option<OrdInput> {
    let label = compound_label(compound).unwrap_or_else(|| label_hint.to_string());
    let smiles = compound_structural_identifier(compound)
        .or_else(|| first_identifier_value(compound.get("identifiers")))
        .unwrap_or_else(|| label.clone());
    Some(OrdInput {
        label,
        smiles,
        role: enum_field(compound, "reaction_role")
            .filter(|role| role != "UNSPECIFIED")
            .or_else(|| fallback_role.map(str::to_string)),
        amount_mmol: compound
            .get("amount")
            .and_then(|amount| amount.get("moles"))
            .and_then(extract_moles_mmol),
        phase: compound.get("texture").and_then(phase_from_texture),
        purity: None,
        yield_percent: None,
    })
}

fn product_to_ord_input(product: &serde_json::Value, index: usize) -> Option<OrdInput> {
    let label = compound_label(product).unwrap_or_else(|| format!("product {index}"));
    let smiles = compound_structural_identifier(product)
        .or_else(|| first_identifier_value(product.get("identifiers")))
        .unwrap_or_else(|| label.clone());
    let (yield_percent, purity) = product_measurements(product.get("measurements"));
    Some(OrdInput {
        label,
        smiles,
        role: enum_field(product, "reaction_role")
            .filter(|role| role != "UNSPECIFIED")
            .or_else(|| Some("PRODUCT".to_string())),
        amount_mmol: None,
        phase: product.get("texture").and_then(phase_from_texture),
        purity,
        yield_percent,
    })
}

fn product_measurements(measurements: Option<&serde_json::Value>) -> (Option<f64>, Option<f64>) {
    let mut yield_percent = None;
    let mut purity = None;
    let Some(measurements) = measurements.and_then(serde_json::Value::as_array) else {
        return (yield_percent, purity);
    };
    for measurement in measurements {
        match enum_field(measurement, "type").as_deref() {
            Some("YIELD") => {
                yield_percent = measurement
                    .get("percentage")
                    .and_then(|value| f64_field(value, "value"));
            }
            Some("PURITY") => {
                purity = measurement
                    .get("percentage")
                    .and_then(|value| f64_field(value, "value"))
                    .map(|percent| percent / 100.0);
            }
            _ => {}
        }
    }
    (yield_percent, purity)
}

fn official_condition_summary(reaction: &serde_json::Value) -> OfficialConditionSummary {
    let conditions = reaction.get("conditions");
    let temperature_c = conditions
        .and_then(|value| value.get("temperature"))
        .and_then(|value| value.get("setpoint"))
        .and_then(extract_temperature_c);
    let stir_rpm = conditions
        .and_then(|value| value.get("stirring"))
        .and_then(|value| value.get("rate"))
        .and_then(|value| u64_field(value, "rpm"))
        .unwrap_or(0);
    OfficialConditionSummary {
        temperature_c,
        time_min: None,
        inert: official_inert(reaction),
        stir_rpm,
    }
}

fn official_inert(reaction: &serde_json::Value) -> bool {
    let setup_environment = reaction
        .get("setup")
        .and_then(|value| value.get("environment"))
        .and_then(|value| enum_field(value, "type"));
    if matches!(
        setup_environment.as_deref(),
        Some("GLOVE_BOX" | "GLOVE_BAG")
    ) {
        return true;
    }

    let pressure_atmosphere = reaction
        .get("conditions")
        .and_then(|value| value.get("pressure"))
        .and_then(|value| value.get("atmosphere"))
        .and_then(|value| enum_field(value, "type"));
    if matches!(pressure_atmosphere.as_deref(), Some("NITROGEN" | "ARGON")) {
        return true;
    }

    if reaction
        .get("notes")
        .is_some_and(|notes| bool_field(notes, "is_sensitive_to_oxygen").unwrap_or(false))
    {
        return true;
    }

    reaction
        .get("setup")
        .and_then(|value| value.get("vessel"))
        .and_then(|value| value.get("preparations"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|preparations| {
            preparations.iter().any(|preparation| {
                matches!(
                    enum_field(preparation, "type").as_deref(),
                    Some("EVACUATED_BACKFILLED" | "PURGED")
                )
            })
        })
}

fn collect_official_provenance(
    reaction: &serde_json::Value,
    reaction_smiles: Option<&str>,
) -> BTreeMap<String, String> {
    let mut provenance = BTreeMap::new();
    if let Some(reaction_id) = string_field(reaction, "reaction_id") {
        provenance.insert("reaction_id".to_string(), reaction_id);
    }
    if let Some(reaction_smiles) = reaction_smiles {
        provenance.insert("reaction_smiles".to_string(), reaction_smiles.to_string());
    }
    if let Some(provenance_value) = reaction.get("provenance") {
        for key in ["doi", "patent", "publication_url", "city"] {
            if let Some(value) = string_field(provenance_value, key) {
                provenance.insert(key.to_string(), value);
            }
        }
        if let Some(value) = provenance_value
            .get("record_created")
            .and_then(|value| value.get("time"))
            .and_then(|value| string_field(value, "value"))
        {
            provenance.insert("record_created".to_string(), value);
        }
        if let Some(experimenter) = provenance_value.get("experimenter") {
            for key in ["name", "organization", "orcid"] {
                if let Some(value) = string_field(experimenter, key) {
                    provenance.insert(format!("experimenter_{key}"), value);
                }
            }
        }
    }
    provenance
}

fn official_created_unix(provenance: Option<&serde_json::Value>) -> u64 {
    provenance
        .and_then(|value| value.get("record_created"))
        .and_then(|value| value.get("time"))
        .and_then(|value| string_field(value, "value"))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

fn collect_analysis_methods(
    analyses_value: Option<&serde_json::Value>,
    analyses: &mut Vec<serde_json::Value>,
    analysis_methods: &mut BTreeSet<String>,
) {
    let Some(analyses_map) = analyses_value.and_then(serde_json::Value::as_object) else {
        return;
    };
    if analyses_map.is_empty() {
        return;
    }
    analyses.push(serde_json::Value::Object(analyses_map.clone()));
    for analysis in analyses_map.values() {
        if let Some(method) = enum_field(analysis, "type").map(|method| {
            if method.starts_with("NMR_") {
                "NMR".to_string()
            } else {
                method
            }
        }) {
            analysis_methods.insert(method);
        }
    }
}

fn compound_label(compound: &serde_json::Value) -> Option<String> {
    identifier_value(
        compound.get("identifiers"),
        &["NAME", "IUPAC_NAME", "CAS_NUMBER"],
    )
}

fn compound_structural_identifier(compound: &serde_json::Value) -> Option<String> {
    identifier_value(
        compound.get("identifiers"),
        &["SMILES", "CXSMILES", "INCHI", "MOLBLOCK"],
    )
}

fn identifier_value(
    identifiers: Option<&serde_json::Value>,
    preferred_types: &[&str],
) -> Option<String> {
    preferred_types.iter().find_map(|preferred_type| {
        identifiers
            .and_then(serde_json::Value::as_array)
            .and_then(|identifiers| {
                identifiers.iter().find_map(|identifier| {
                    (enum_field(identifier, "type").as_deref() == Some(*preferred_type))
                        .then(|| string_field(identifier, "value"))
                        .flatten()
                })
            })
    })
}

fn first_identifier_value(identifiers: Option<&serde_json::Value>) -> Option<String> {
    identifiers
        .and_then(serde_json::Value::as_array)
        .and_then(|identifiers| {
            identifiers
                .iter()
                .find_map(|identifier| string_field(identifier, "value"))
        })
}

fn role_from_label(label: &str) -> Option<&'static str> {
    let key = normalize_key(label);
    if key.contains("solvent") {
        Some("SOLVENT")
    } else if key.contains("catalyst") || key.contains("ligand") || key.contains("pd") {
        Some("CATALYST")
    } else if key.contains("reagent") || key.contains("base") || key.contains("acid") {
        Some("REAGENT")
    } else if key.contains("reactant") || key.contains("substrate") {
        Some("REACTANT")
    } else {
        None
    }
}

fn extract_moles_mmol(moles: &serde_json::Value) -> Option<f64> {
    let value = f64_field(moles, "value")?;
    match enum_field(moles, "units").as_deref() {
        Some("MOLE") => Some(value * 1_000.0),
        Some("MILLIMOLE") | None => Some(value),
        Some("MICROMOLE") => Some(value / 1_000.0),
        Some("NANOMOLE") => Some(value / 1_000_000.0),
        _ => Some(value),
    }
}

fn extract_time_min(time: &serde_json::Value) -> Option<f64> {
    let value = f64_field(time, "value")?;
    match enum_field(time, "units").as_deref() {
        Some("DAY") => Some(value * 1_440.0),
        Some("HOUR") => Some(value * 60.0),
        Some("MINUTE") | None => Some(value),
        Some("SECOND") => Some(value / 60.0),
        _ => Some(value),
    }
}

fn extract_temperature_c(temperature: &serde_json::Value) -> Option<f64> {
    let value = f64_field(temperature, "value")?;
    match enum_field(temperature, "units").as_deref() {
        Some("KELVIN") => Some(value - 273.15),
        Some("FAHRENHEIT") => Some((value - 32.0) * 5.0 / 9.0),
        Some("CELSIUS") | None => Some(value),
        _ => Some(value),
    }
}

fn positive_duration(value: f64) -> Option<f64> {
    (value > 0.0).then_some(value)
}

fn phase_from_texture(texture: &serde_json::Value) -> Option<AdtPhase> {
    match enum_field(texture, "type").as_deref() {
        Some(
            "POWDER" | "CRYSTAL" | "AMORPHOUS_SOLID" | "FOAM" | "WAX" | "SEMI_SOLID" | "SOLID",
        ) => Some(AdtPhase::Solid),
        Some("OIL" | "LIQUID") => Some(AdtPhase::Liquid),
        Some("GAS") => Some(AdtPhase::Gas),
        _ => None,
    }
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| match value {
        serde_json::Value::String(text) if !text.is_empty() => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

fn enum_field(value: &serde_json::Value, key: &str) -> Option<String> {
    string_field(value, key)
}

fn f64_field(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|value| match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    })
}

fn u64_field(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| match value {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    })
}

fn bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(|value| match value {
        serde_json::Value::Bool(value) => Some(*value),
        serde_json::Value::String(text) => text.parse::<bool>().ok(),
        _ => None,
    })
}

#[must_use]
pub fn demo_suzuki_ord_like() -> OrdLikeReaction {
    OrdLikeReaction::from_json_str(DEMO_SUZUKI_ORD_LIKE).expect("valid demo ORD-like JSON")
}

fn ensure_reagent_sample(
    reagent: &str,
    samples: &mut Vec<AdtSample>,
    used_ids: &mut BTreeSet<String>,
    reagent_lookup: &mut BTreeMap<String, String>,
) -> Result<String, OrdAdtError> {
    let key = normalize_key(reagent);
    if let Some(sample_id) = reagent_lookup.get(&key) {
        return Ok(sample_id.clone());
    }
    let known = known_reagent(reagent).ok_or_else(|| OrdAdtError::MissingReferencedReagent {
        reagent: reagent.to_string(),
    })?;
    let id = unique_sample_id(known.id_hint, samples.len() + 1, used_ids);
    reagent_lookup.insert(normalize_key(&id), id.clone());
    reagent_lookup.insert(key, id.clone());
    reagent_lookup.insert(normalize_key(known.label), id.clone());
    samples.push(AdtSample {
        id: id.clone(),
        label: known.label.to_string(),
        smiles: known.smiles.to_string(),
        amount_mmol: 0.0,
        phase: known.phase,
        purity: None,
        role: Some("WORKUP".to_string()),
        yield_percent: None,
    });
    Ok(id)
}

struct KnownReagent {
    id_hint: &'static str,
    label: &'static str,
    smiles: &'static str,
    phase: AdtPhase,
}

fn known_reagent(reagent: &str) -> Option<KnownReagent> {
    match normalize_key(reagent).as_str() {
        "water" | "h2o" => Some(KnownReagent {
            id_hint: "water",
            label: "Water",
            smiles: "O",
            phase: AdtPhase::Liquid,
        }),
        _ => None,
    }
}

fn unique_sample_id(label: &str, index: usize, used_ids: &mut BTreeSet<String>) -> String {
    let base = slugify(label).unwrap_or_else(|| format!("sample-{index}"));
    if used_ids.insert(base.clone()) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if used_ids.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search returns")
}

fn slugify(label: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in label.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

fn normalize_key(value: &str) -> String {
    slugify(value).unwrap_or_else(|| value.trim().to_ascii_lowercase())
}

fn infer_phase(label: &str) -> Option<AdtPhase> {
    let key = normalize_key(label);
    if key.contains("carbonate")
        || key.contains("k2co3")
        || key.contains("boronic")
        || key.contains("pd")
        || key.contains("catalyst")
    {
        Some(AdtPhase::Solid)
    } else if key.contains("water")
        || key.contains("toluene")
        || key.contains("bromobenzene")
        || key.contains("solvent")
    {
        Some(AdtPhase::Liquid)
    } else {
        None
    }
}

const DEMO_SUZUKI_ORD_LIKE: &str = r#"{
  "name": "Suzuki coupling — biphenyl (hackathon v0)",
  "created_unix": 1761764220,
  "inputs": [
    {"label": "Bromobenzene", "smiles": "Brc1ccccc1"},
    {"label": "Phenylboronic pinacol ester", "smiles": "B(Oc1ccccc1)O"},
    {"label": "K2CO3", "smiles": "O=C([O-])O[K+] . O=C([O-])O[K+] . [K+].[O-]C(=O)O"},
    {"label": "Pd(PPh3)4", "smiles": "P(c1ccccc1)(c1ccccc1)c1ccccc1.Pd"},
    {"label": "Toluene", "smiles": "Cc1ccccc1"}
  ],
  "setup": {"inert": true, "stir_rpm": 600},
  "conditions": {"temperature_C": 80, "time_min": 120},
  "procedural_steps": [
    {"op": "Charge"},
    {"op": "Heat", "target_C": 80, "hold_min": 60},
    {"op": "StirTo", "rpm": 600, "hold_min": 60},
    {"op": "Quench", "reagent": "water"},
    {"op": "Purify", "mode": "PrepLC"},
    {"op": "Measure", "analyte": "product", "method": "NMR"}
  ],
  "analyses": [],
  "provenance": {
    "adt_sha256": "demo"
  }
}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use chimiaclaw_artifact::{ArtifactDraft, PayloadLocation, PayloadRef};

    fn translator() -> OrdToAdtTranslator {
        OrdToAdtTranslator::new(
            AgentId(ORD_ADT_AGENT.to_string()),
            ArtifactSigner::from_seed([51; 32]),
        )
    }

    fn ord_artifact(schema_tags: BTreeSet<SchemaTag>) -> Artifact {
        let ord = demo_suzuki_ord_like();
        let payload = PayloadRef::inline_json(&ord).expect("ord payload");
        ArtifactDraft {
            skill: SkillId("chem.ord.import.v1".to_string()),
            agent: AgentId("ord.importer.eth".to_string()),
            topic: "demo ORD-like Suzuki reaction".to_string(),
            input_fingerprint: "ord-like:suzuki".to_string(),
            output_cid: Some("inline://ord-like/suzuki".to_string()),
            parent_artifact_ids: Vec::new(),
            schema_tags,
            payload: Some(payload),
        }
        .seal(&ArtifactSigner::from_seed([50; 32]), 1)
        .expect("ord artifact")
    }

    #[test]
    fn translates_suzuki_ord_like_to_minimal_adt() {
        let ord = demo_suzuki_ord_like();
        let experiment = translator().translate(&ord).expect("translate");

        assert_eq!(experiment.metadata.title, ord.name);
        assert_eq!(experiment.samples.len(), 6);
        assert_eq!(experiment.reaction.inputs.len(), 5);
        assert_eq!(experiment.reaction.steps.len(), 6);
        assert_eq!(experiment.reaction.conditions.temperature_c, 80.0);
        assert!(experiment.reaction.conditions.inert);

        let water = experiment
            .samples
            .iter()
            .find(|sample| sample.id == "water")
            .expect("water quench sample");
        assert_eq!(water.smiles, "O");
        assert_eq!(water.phase, AdtPhase::Liquid);

        let quench = &experiment.reaction.steps[3];
        assert_eq!(quench.op, AdtOperation::Quench);
        assert_eq!(quench.reagent.as_deref(), Some("water"));
    }

    #[test]
    fn translates_official_ord_json_with_roles_workup_and_outcome() {
        let ord = OrdLikeReaction::from_official_ord_json_str(OFFICIAL_ORD_JSON)
            .expect("official ORD JSON parses");

        assert_eq!(
            ord.name,
            "ORD reaction ord-00000000000000000000000000000001"
        );
        assert_eq!(ord.inputs.len(), 5);
        assert_eq!(ord.auxiliary_samples.len(), 1);
        assert_eq!(ord.products.len(), 1);
        assert_eq!(ord.conditions.temperature_c, 80.0);
        assert_eq!(ord.conditions.time_min, 120.0);
        assert!(ord.setup.inert);

        let experiment = translator().translate(&ord).expect("translate");
        assert_eq!(experiment.samples.len(), 7);
        assert_eq!(experiment.reaction.inputs.len(), 5);
        assert_eq!(experiment.reaction.conditions.stir_rpm, 600);

        let bromobenzene = experiment
            .samples
            .iter()
            .find(|sample| sample.label == "Bromobenzene")
            .expect("bromobenzene sample");
        assert_eq!(bromobenzene.role.as_deref(), Some("REACTANT"));
        assert_eq!(bromobenzene.amount_mmol, 10.0);
        assert_eq!(bromobenzene.phase, AdtPhase::Liquid);

        let water = experiment
            .samples
            .iter()
            .find(|sample| sample.label == "Water")
            .expect("water workup sample");
        assert_eq!(water.role.as_deref(), Some("WORKUP"));

        let product = experiment
            .samples
            .iter()
            .find(|sample| sample.label == "Biphenyl")
            .expect("product sample");
        assert_eq!(product.role.as_deref(), Some("PRODUCT"));
        assert_eq!(product.yield_percent, Some(76.0));

        assert!(experiment.reaction.steps.iter().any(|step| {
            step.op == AdtOperation::Quench && step.reagent.as_deref() == Some("water")
        }));
        assert!(experiment.reaction.steps.iter().any(|step| {
            step.op == AdtOperation::Measure && step.method.as_deref() == Some("NMR")
        }));
    }

    #[test]
    fn seals_adt_translation_as_child_artifact() {
        let ord = demo_suzuki_ord_like();
        let ord_artifact = ord_artifact(BTreeSet::from([SchemaTag(ORD_REACTION_TAG.to_string())]));
        let signed = translator()
            .translate_and_seal(&ord_artifact, &ord, 2)
            .expect("signed adt");

        assert!(signed.artifact.has_parent(&ord_artifact.id));
        assert!(signed
            .artifact
            .schema_tags
            .contains(&SchemaTag(ADT_REACTION_TAG.to_string())));
        signed.artifact.verify().expect("adt artifact verifies");
        assert_eq!(
            signed.artifact.output_cid,
            Some(format!(
                "inline://chimiaclaw/adt/{}",
                adt_experiment_hash(&signed.experiment).expect("adt hash")
            ))
        );
    }

    #[test]
    fn signed_adt_artifact_is_payload_bound() {
        let ord = demo_suzuki_ord_like();
        let ord_artifact = ord_artifact(BTreeSet::from([SchemaTag(ORD_REACTION_TAG.to_string())]));
        let signed = translator()
            .translate_and_seal(&ord_artifact, &ord, 2)
            .expect("signed adt");

        signed
            .artifact
            .verify_payload_value(&signed.experiment)
            .expect("adt payload digest matches experiment");

        let bytes = signed
            .artifact
            .inline_payload_bytes()
            .expect("decode")
            .expect("inline payload present");
        let recovered: AdtExperiment = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(recovered, signed.experiment);
    }

    #[test]
    fn tampered_inline_adt_payload_breaks_artifact_signature() {
        let ord = demo_suzuki_ord_like();
        let ord_artifact = ord_artifact(BTreeSet::from([SchemaTag(ORD_REACTION_TAG.to_string())]));
        let mut signed = translator()
            .translate_and_seal(&ord_artifact, &ord, 2)
            .expect("signed adt");

        let mut tampered_experiment = signed.experiment.clone();
        tampered_experiment.metadata.title = "Forged Suzuki coupling".to_string();
        let attacker_bytes =
            chimiaclaw_artifact::canonical_bytes(&tampered_experiment).expect("canonical");

        if let Some(payload_ref) = signed.artifact.payload.as_mut() {
            if let PayloadLocation::Inline { bytes_hex } = &mut payload_ref.location {
                *bytes_hex = hex::encode(&attacker_bytes);
            } else {
                panic!("expected inline payload");
            }
        } else {
            panic!("expected payload reference on signed adt artifact");
        }

        assert!(matches!(
            signed.artifact.verify(),
            Err(ArtifactError::ContentHashMismatch { .. })
        ));
    }

    #[test]
    fn rejects_unknown_reagent_reference() {
        let mut ord = demo_suzuki_ord_like();
        ord.procedural_steps[3].reagent = Some("mystery-quench".to_string());

        let err = translator()
            .translate(&ord)
            .expect_err("missing reagent rejected");

        assert_eq!(
            err,
            OrdAdtError::MissingReferencedReagent {
                reagent: "mystery-quench".to_string()
            }
        );
    }

    #[test]
    fn rejects_unsigned_parent_without_ord_tag() {
        let ord = demo_suzuki_ord_like();
        let ord_artifact = ord_artifact(BTreeSet::from([SchemaTag("chem.other".to_string())]));

        let err = translator()
            .translate_and_seal(&ord_artifact, &ord, 2)
            .expect_err("missing tag rejected");

        assert_eq!(
            err,
            OrdAdtError::MissingOrdReactionTag {
                artifact_id: ord_artifact.id.0
            }
        );
    }

    const OFFICIAL_ORD_JSON: &str = r#"{
      "reaction_id": "ord-00000000000000000000000000000001",
      "identifiers": [
        {"type": "REACTION_SMILES", "value": "Brc1ccccc1.B(O)c1ccccc1>>c1ccc(-c2ccccc2)cc1"}
      ],
      "inputs": {
        "01 aryl bromide": {
          "components": [{
            "identifiers": [
              {"type": "NAME", "value": "Bromobenzene"},
              {"type": "SMILES", "value": "Brc1ccccc1"}
            ],
            "reaction_role": "REACTANT",
            "amount": {"moles": {"value": 10, "units": "MILLIMOLE"}},
            "texture": {"type": "LIQUID"}
          }]
        },
        "02 boronic acid": {
          "components": [{
            "identifiers": [
              {"type": "NAME", "value": "Phenylboronic acid"},
              {"type": "SMILES", "value": "OB(O)c1ccccc1"}
            ],
            "reaction_role": "REACTANT",
            "amount": {"moles": {"value": 0.012, "units": "MOLE"}},
            "texture": {"type": "SOLID"}
          }]
        },
        "03 base": {
          "components": [{
            "identifiers": [
              {"type": "NAME", "value": "Potassium carbonate"},
              {"type": "SMILES", "value": "O=C([O-])[O-].[K+].[K+]"}
            ],
            "reaction_role": "REAGENT",
            "amount": {"moles": {"value": 30, "units": "MILLIMOLE"}},
            "texture": {"type": "POWDER"}
          }]
        },
        "04 catalyst": {
          "components": [{
            "identifiers": [
              {"type": "NAME", "value": "Pd(PPh3)4"},
              {"type": "SMILES", "value": "P(c1ccccc1)(c1ccccc1)c1ccccc1.Pd"}
            ],
            "reaction_role": "CATALYST",
            "amount": {"moles": {"value": 500, "units": "MICROMOLE"}},
            "texture": {"type": "SOLID"}
          }]
        },
        "05 solvent": {
          "components": [{
            "identifiers": [
              {"type": "NAME", "value": "Toluene"},
              {"type": "SMILES", "value": "Cc1ccccc1"}
            ],
            "reaction_role": "SOLVENT",
            "texture": {"type": "LIQUID"}
          }]
        }
      },
      "setup": {
        "vessel": {"preparations": [{"type": "EVACUATED_BACKFILLED"}]},
        "environment": {"type": "FUME_HOOD"}
      },
      "conditions": {
        "temperature": {"setpoint": {"value": 80, "units": "CELSIUS"}},
        "stirring": {"rate": {"rpm": 600}},
        "pressure": {"atmosphere": {"type": "NITROGEN"}}
      },
      "workups": [{
        "type": "ADDITION",
        "details": "quench with water",
        "input": {
          "components": [{
            "identifiers": [
              {"type": "NAME", "value": "Water"},
              {"type": "SMILES", "value": "O"}
            ],
            "reaction_role": "WORKUP",
            "texture": {"type": "LIQUID"}
          }]
        }
      }],
      "outcomes": [{
        "reaction_time": {"value": 2, "units": "HOUR"},
        "products": [{
          "identifiers": [
            {"type": "NAME", "value": "Biphenyl"},
            {"type": "SMILES", "value": "c1ccc(-c2ccccc2)cc1"}
          ],
          "is_desired_product": true,
          "measurements": [{
            "type": "YIELD",
            "percentage": {"value": 76}
          }],
          "reaction_role": "PRODUCT",
          "texture": {"type": "SOLID"}
        }],
        "analyses": {
          "nmr": {"type": "NMR_1H", "details": "identity confirmed"}
        }
      }],
      "provenance": {
        "doi": "10.0000/example",
        "record_created": {"time": {"value": "1761764220"}},
        "experimenter": {"name": "ChimiaDAO"}
      }
    }"#;
}
