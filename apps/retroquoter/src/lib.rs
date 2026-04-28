//! RetroQuoter reference swarm profiles.
use chimiaclaw_artifact::{
    blake3_hex, canonical_bytes, Artifact, ArtifactDraft, ArtifactError, ArtifactId,
    ArtifactSigner, ArtifactStore, ArtifactStoreError, PayloadRef,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use chimiaclaw_skill::{Skill, SkillCtx, SkillError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PLANNER_AGENT: &str = "planner.retroquoter.eth";
pub const PROCUREMENT_AGENT: &str = "procurement.retroquoter.eth";
pub const SAFETY_AGENT: &str = "safety.retroquoter.eth";
pub const SETTLEMENT_AGENT: &str = "settlement.retroquoter.eth";
pub const ROUTE_PROPOSAL_TAG: &str = "chem.retrosynth.route_proposal";
pub const ROUTE_QUOTE_TAG: &str = "chem.procurement.route_quote";
pub const ROUTE_QUOTE_SKILL: &str = "chem.procurement.supplier_quote.v1";
pub const PROCUREMENT_PROCURED_TAG: &str = "chem.procurement.procured";
pub const PROCUREMENT_EXECUTION_SKILL: &str = "chem.procurement.execute.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteRequest {
    pub target_smiles: String,
    pub scale_grams: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteProposal {
    pub route_id: String,
    pub target_smiles: String,
    pub target_scale_milligrams: u64,
    pub steps: Vec<RouteStep>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteStep {
    pub step_id: String,
    pub description: String,
    pub requirements: Vec<ReagentRequirement>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReagentRequirement {
    pub reagent_id: String,
    pub display_name: String,
    pub quantity_milligrams: u64,
    pub role: ReagentRole,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReagentRole {
    StartingMaterial,
    Reagent,
    Catalyst,
    Solvent,
    Workup,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplierOffer {
    pub supplier: String,
    pub sku: String,
    pub unit_price_cents_per_gram: u64,
    pub available_grams: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReagentCatalog {
    offers: BTreeMap<String, SupplierOffer>,
}

impl ReagentCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_offer(mut self, reagent_id: impl Into<String>, offer: SupplierOffer) -> Self {
        self.offers.insert(reagent_id.into(), offer);
        self
    }

    #[must_use]
    pub fn offer(&self, reagent_id: &str) -> Option<&SupplierOffer> {
        self.offers.get(reagent_id)
    }
}

/// Deterministic aspirin route fixture used by CLI and runtime demos.
#[must_use]
pub fn demo_route_proposal() -> RouteProposal {
    RouteProposal {
        route_id: "route-aspirin-demo".to_string(),
        target_smiles: "CC(=O)Oc1ccccc1C(=O)O".to_string(),
        target_scale_milligrams: 5_000,
        steps: vec![RouteStep {
            step_id: "step-1".to_string(),
            description: "Acetylate salicylic acid with acetic anhydride".to_string(),
            requirements: vec![
                ReagentRequirement {
                    reagent_id: "salicylic-acid".to_string(),
                    display_name: "Salicylic acid".to_string(),
                    quantity_milligrams: 4_000,
                    role: ReagentRole::StartingMaterial,
                },
                ReagentRequirement {
                    reagent_id: "acetic-anhydride".to_string(),
                    display_name: "Acetic anhydride".to_string(),
                    quantity_milligrams: 6_000,
                    role: ReagentRole::Reagent,
                },
            ],
        }],
    }
}

/// Deterministic supplier catalog fixture used by CLI and runtime demos.
#[must_use]
pub fn demo_reagent_catalog() -> ReagentCatalog {
    ReagentCatalog::new()
        .with_offer(
            "salicylic-acid",
            SupplierOffer {
                supplier: "DemoChem".to_string(),
                sku: "SAL-001".to_string(),
                unit_price_cents_per_gram: 120,
                available_grams: 100,
            },
        )
        .with_offer(
            "acetic-anhydride",
            SupplierOffer {
                supplier: "DemoChem".to_string(),
                sku: "ACE-002".to_string(),
                unit_price_cents_per_gram: 80,
                available_grams: 250,
            },
        )
}

/// Deterministic execution request fixture used by the in-memory demo.
#[must_use]
pub fn demo_execution_request() -> ProcurementExecutionRequest {
    ProcurementExecutionRequest {
        buyer_agent: AgentId("buyer.chimiaclaw.eth".to_string()),
        payment_reference: "uniswap-swap-demo-001".to_string(),
        destination_profile_id: "sofia-lab-default".to_string(),
    }
}

/// Build an unsigned route quote draft for sealing by a runtime-owned signer.
pub fn build_route_quote_draft(
    agent: &AgentId,
    route_artifact: &Artifact,
    route: &RouteProposal,
    catalog: &ReagentCatalog,
    contingency_basis_points: u64,
) -> Result<(RouteQuote, ArtifactDraft), RetroQuoteError> {
    validate_route_artifact(route_artifact)?;
    route_artifact
        .verify_payload_value(route)
        .map_err(|error| match error {
            ArtifactError::PayloadHashMismatch { expected, actual } => {
                RetroQuoteError::RoutePayloadMismatch { expected, actual }
            }
            other => RetroQuoteError::Artifact(other),
        })?;
    let quote = price_route(route_artifact, route, catalog, contingency_basis_points)?;
    let quote_hash = quote_hash(&quote)?;
    let quote_payload = PayloadRef::inline_json(&quote).map_err(RetroQuoteError::Artifact)?;
    let draft = ArtifactDraft {
        skill: SkillId(ROUTE_QUOTE_SKILL.to_string()),
        agent: agent.clone(),
        topic: format!("supplier quote for route {}", route.route_id),
        input_fingerprint: blake3_hex(
            format!(
                "{}:{}",
                route_artifact.content_hash, quote.route_payload_hash
            )
            .as_bytes(),
        ),
        output_cid: Some(format!("inline://retroquoter/quote/{quote_hash}")),
        parent_artifact_ids: vec![route_artifact.id.clone()],
        schema_tags: BTreeSet::from([SchemaTag(ROUTE_QUOTE_TAG.to_string())]),
        payload: Some(quote_payload),
    };
    Ok((quote, draft))
}

/// Decode and verify the inline `RouteProposal` payload from a route artifact.
pub fn decode_route_payload(artifact: &Artifact) -> Result<RouteProposal, RetroQuoteError> {
    validate_route_artifact(artifact)?;
    let bytes = artifact
        .inline_payload_bytes()
        .map_err(RetroQuoteError::Artifact)?
        .ok_or_else(|| RetroQuoteError::PayloadUnavailable {
            artifact_id: artifact.id.clone(),
        })?;
    artifact
        .verify_payload_bytes(&bytes)
        .map_err(RetroQuoteError::Artifact)?;
    serde_json::from_slice(&bytes).map_err(|error| RetroQuoteError::Json(error.to_string()))
}

fn price_route(
    route_artifact: &Artifact,
    route: &RouteProposal,
    catalog: &ReagentCatalog,
    contingency_basis_points: u64,
) -> Result<RouteQuote, RetroQuoteError> {
    if route.steps.is_empty() {
        return Err(RetroQuoteError::EmptyRoute(route.route_id.clone()));
    }

    let route_payload_hash = route_payload_hash(route)?;
    let mut line_items = Vec::new();
    for step in &route.steps {
        for requirement in &step.requirements {
            let offer = catalog.offer(&requirement.reagent_id).ok_or_else(|| {
                RetroQuoteError::UnknownReagent {
                    reagent_id: requirement.reagent_id.clone(),
                }
            })?;
            let requested_grams = ceil_div(requirement.quantity_milligrams, 1_000);
            if requested_grams > offer.available_grams {
                return Err(RetroQuoteError::InsufficientAvailability {
                    reagent_id: requirement.reagent_id.clone(),
                    requested_grams,
                    available_grams: offer.available_grams,
                });
            }
            let line_total_cents = ceil_div(
                requirement
                    .quantity_milligrams
                    .saturating_mul(offer.unit_price_cents_per_gram),
                1_000,
            );
            line_items.push(QuoteLineItem {
                step_id: step.step_id.clone(),
                reagent_id: requirement.reagent_id.clone(),
                display_name: requirement.display_name.clone(),
                role: requirement.role.clone(),
                supplier: offer.supplier.clone(),
                sku: offer.sku.clone(),
                quantity_milligrams: requirement.quantity_milligrams,
                unit_price_cents_per_gram: offer.unit_price_cents_per_gram,
                line_total_cents,
            });
        }
    }

    let subtotal_cents = line_items
        .iter()
        .map(|item| item.line_total_cents)
        .sum::<u64>();
    let contingency_cents = ceil_div(
        subtotal_cents.saturating_mul(contingency_basis_points),
        10_000,
    );
    let total_cents = subtotal_cents.saturating_add(contingency_cents);
    let quote_id = format!(
        "quote_{}",
        &blake3_hex(
            format!(
                "{}:{}:{}",
                route_artifact.id.0, route_payload_hash, total_cents
            )
            .as_bytes()
        )[..16]
    );

    Ok(RouteQuote {
        quote_id,
        route_artifact_id: route_artifact.id.clone(),
        route_content_hash: route_artifact.content_hash.clone(),
        route_payload_hash,
        target_smiles: route.target_smiles.clone(),
        target_scale_milligrams: route.target_scale_milligrams,
        line_items,
        subtotal_cents,
        contingency_cents,
        total_cents,
        currency: "USD".to_string(),
    })
}

/// Skill wrapper that prices a payload-bound route proposal into a route quote.
pub struct RouteQuoteSkill {
    catalog: ReagentCatalog,
    contingency_basis_points: u64,
}

impl RouteQuoteSkill {
    #[must_use]
    pub fn new(catalog: ReagentCatalog) -> Self {
        Self {
            catalog,
            contingency_basis_points: 1_500,
        }
    }

    #[must_use]
    pub fn demo() -> Self {
        Self::new(demo_reagent_catalog())
    }

    #[must_use]
    pub fn with_contingency_basis_points(mut self, basis_points: u64) -> Self {
        self.contingency_basis_points = basis_points;
        self
    }
}

impl Skill for RouteQuoteSkill {
    fn id(&self) -> SkillId {
        SkillId(ROUTE_QUOTE_SKILL.to_string())
    }

    fn capabilities(&self) -> Vec<chimiaclaw_schema::Capability> {
        Vec::new()
    }

    fn consumes_tags(&self) -> Vec<SchemaTag> {
        vec![SchemaTag(ROUTE_PROPOSAL_TAG.to_string())]
    }

    fn produces_tags(&self) -> Vec<SchemaTag> {
        vec![SchemaTag(ROUTE_QUOTE_TAG.to_string())]
    }

    fn invoke(&self, ctx: &SkillCtx, parents: &[Artifact]) -> Result<ArtifactDraft, SkillError> {
        let parent = parents.first().ok_or_else(|| {
            SkillError::InvalidInput(
                "route quote skill requires exactly one route proposal artifact".to_string(),
            )
        })?;
        let route = decode_route_payload(parent).map_err(|error| match error {
            RetroQuoteError::Artifact(inner) => {
                SkillError::InvalidInput(format!("failed to decode route payload: {inner:?}"))
            }
            RetroQuoteError::Json(message) => {
                SkillError::InvalidInput(format!("route payload was not valid JSON: {message}"))
            }
            RetroQuoteError::PayloadUnavailable { artifact_id } => {
                SkillError::InvalidInput(format!(
                    "route artifact {} did not carry an inline payload",
                    artifact_id.0
                ))
            }
            other => SkillError::Execution(format!("route quote failed: {other:?}")),
        })?;
        let (_quote, draft) = build_route_quote_draft(
            &ctx.agent,
            parent,
            &route,
            &self.catalog,
            self.contingency_basis_points,
        )
        .map_err(|error| SkillError::Execution(format!("route quote failed: {error:?}")))?;
        Ok(draft)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteQuote {
    pub quote_id: String,
    pub route_artifact_id: ArtifactId,
    pub route_content_hash: String,
    pub route_payload_hash: String,
    pub target_smiles: String,
    pub target_scale_milligrams: u64,
    pub line_items: Vec<QuoteLineItem>,
    pub subtotal_cents: u64,
    pub contingency_cents: u64,
    pub total_cents: u64,
    pub currency: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuoteLineItem {
    pub step_id: String,
    pub reagent_id: String,
    pub display_name: String,
    pub role: ReagentRole,
    pub supplier: String,
    pub sku: String,
    pub quantity_milligrams: u64,
    pub unit_price_cents_per_gram: u64,
    pub line_total_cents: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedRouteQuote {
    pub quote: RouteQuote,
    pub artifact: Artifact,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcurementExecutionRequest {
    pub buyer_agent: AgentId,
    pub payment_reference: String,
    pub destination_profile_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProcurementState {
    Procured,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcurementReceipt {
    pub receipt_id: String,
    pub quote_id: String,
    pub quote_artifact_id: ArtifactId,
    pub quote_content_hash: String,
    pub quote_payload_hash: String,
    pub buyer_agent: AgentId,
    pub payment_reference: String,
    pub destination_profile_id: String,
    pub state: ProcurementState,
    pub procured_line_items: Vec<ProcuredLineItem>,
    pub total_cents: u64,
    pub currency: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcuredLineItem {
    pub step_id: String,
    pub reagent_id: String,
    pub supplier: String,
    pub sku: String,
    pub quantity_milligrams: u64,
    pub line_total_cents: u64,
    pub supplier_order_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedProcurementReceipt {
    pub receipt: ProcurementReceipt,
    pub artifact: Artifact,
}

pub struct RetroQuoter {
    agent: AgentId,
    signer: ArtifactSigner,
    catalog: ReagentCatalog,
    contingency_basis_points: u64,
}

impl RetroQuoter {
    #[must_use]
    pub fn new(agent: AgentId, signer: ArtifactSigner, catalog: ReagentCatalog) -> Self {
        Self {
            agent,
            signer,
            catalog,
            contingency_basis_points: 1_500,
        }
    }

    #[must_use]
    pub fn with_contingency_basis_points(mut self, basis_points: u64) -> Self {
        self.contingency_basis_points = basis_points;
        self
    }

    pub fn quote_route_from_store<S: ArtifactStore>(
        &self,
        store: &S,
        route_artifact_id: &ArtifactId,
        route: &RouteProposal,
        created_at_unix: u64,
    ) -> Result<SignedRouteQuote, RetroQuoteError> {
        let route_artifact = store
            .get(route_artifact_id)
            .map_err(RetroQuoteError::Store)?
            .ok_or_else(|| RetroQuoteError::MissingArtifact(route_artifact_id.clone()))?;
        self.verify_parent_chain(store, &route_artifact)?;
        self.quote_route(&route_artifact, route, created_at_unix)
    }

    pub fn quote_route(
        &self,
        route_artifact: &Artifact,
        route: &RouteProposal,
        created_at_unix: u64,
    ) -> Result<SignedRouteQuote, RetroQuoteError> {
        let (quote, draft) = build_route_quote_draft(
            &self.agent,
            route_artifact,
            route,
            &self.catalog,
            self.contingency_basis_points,
        )?;
        let quote_artifact = draft
            .seal(&self.signer, created_at_unix)
            .map_err(RetroQuoteError::Artifact)?;

        Ok(SignedRouteQuote {
            quote,
            artifact: quote_artifact,
        })
    }

    fn verify_parent_chain<S: ArtifactStore>(
        &self,
        store: &S,
        artifact: &Artifact,
    ) -> Result<(), RetroQuoteError> {
        artifact.verify().map_err(RetroQuoteError::Artifact)?;
        for parent_id in &artifact.parent_artifact_ids {
            let parent = store
                .get(parent_id)
                .map_err(RetroQuoteError::Store)?
                .ok_or_else(|| RetroQuoteError::MissingArtifact(parent_id.clone()))?;
            parent.verify().map_err(RetroQuoteError::Artifact)?;
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RetroQuoteError {
    Artifact(ArtifactError),
    Store(ArtifactStoreError),
    Json(String),
    MissingArtifact(ArtifactId),
    PayloadUnavailable {
        artifact_id: ArtifactId,
    },
    MissingRouteProposalTag {
        artifact_id: ArtifactId,
    },
    MissingRouteQuoteTag {
        artifact_id: ArtifactId,
    },
    EmptyRoute(String),
    UnknownReagent {
        reagent_id: String,
    },
    InsufficientAvailability {
        reagent_id: String,
        requested_grams: u64,
        available_grams: u64,
    },
    QuoteArtifactDoesNotReferenceRoute {
        quote_artifact_id: ArtifactId,
        route_artifact_id: ArtifactId,
    },
    RoutePayloadMismatch {
        expected: String,
        actual: String,
    },
    QuotePayloadMismatch {
        expected: String,
        actual: String,
    },
}

pub fn validate_route_artifact(artifact: &Artifact) -> Result<(), RetroQuoteError> {
    artifact.verify().map_err(RetroQuoteError::Artifact)?;
    let route_tag = SchemaTag(ROUTE_PROPOSAL_TAG.to_string());
    if !artifact.schema_tags.contains(&route_tag) {
        return Err(RetroQuoteError::MissingRouteProposalTag {
            artifact_id: artifact.id.clone(),
        });
    }
    Ok(())
}

pub fn route_payload_hash(route: &RouteProposal) -> Result<String, RetroQuoteError> {
    canonical_bytes(route)
        .map(|bytes| blake3_hex(&bytes))
        .map_err(RetroQuoteError::Artifact)
}

pub fn quote_hash(quote: &RouteQuote) -> Result<String, RetroQuoteError> {
    canonical_bytes(quote)
        .map(|bytes| blake3_hex(&bytes))
        .map_err(RetroQuoteError::Artifact)
}
pub fn validate_quote_artifact(
    artifact: &Artifact,
    quote: &RouteQuote,
) -> Result<(), RetroQuoteError> {
    artifact.verify().map_err(RetroQuoteError::Artifact)?;
    let quote_tag = SchemaTag(ROUTE_QUOTE_TAG.to_string());
    if !artifact.schema_tags.contains(&quote_tag) {
        return Err(RetroQuoteError::MissingRouteQuoteTag {
            artifact_id: artifact.id.clone(),
        });
    }
    if !artifact.has_parent(&quote.route_artifact_id) {
        return Err(RetroQuoteError::QuoteArtifactDoesNotReferenceRoute {
            quote_artifact_id: artifact.id.clone(),
            route_artifact_id: quote.route_artifact_id.clone(),
        });
    }
    artifact
        .verify_payload_value(quote)
        .map_err(|error| match error {
            ArtifactError::PayloadHashMismatch { expected, actual } => {
                RetroQuoteError::QuotePayloadMismatch { expected, actual }
            }
            other => RetroQuoteError::Artifact(other),
        })?;
    Ok(())
}

pub struct ProcurementExecutor {
    agent: AgentId,
    signer: ArtifactSigner,
}

impl ProcurementExecutor {
    #[must_use]
    pub fn new(agent: AgentId, signer: ArtifactSigner) -> Self {
        Self { agent, signer }
    }

    pub fn execute_from_store<S: ArtifactStore>(
        &self,
        store: &S,
        quote_artifact_id: &ArtifactId,
        quote: &RouteQuote,
        request: &ProcurementExecutionRequest,
        created_at_unix: u64,
    ) -> Result<SignedProcurementReceipt, RetroQuoteError> {
        let quote_artifact = store
            .get(quote_artifact_id)
            .map_err(RetroQuoteError::Store)?
            .ok_or_else(|| RetroQuoteError::MissingArtifact(quote_artifact_id.clone()))?;
        quote_artifact.verify().map_err(RetroQuoteError::Artifact)?;
        for parent_id in &quote_artifact.parent_artifact_ids {
            let parent = store
                .get(parent_id)
                .map_err(RetroQuoteError::Store)?
                .ok_or_else(|| RetroQuoteError::MissingArtifact(parent_id.clone()))?;
            parent.verify().map_err(RetroQuoteError::Artifact)?;
        }
        self.execute(&quote_artifact, quote, request, created_at_unix)
    }

    pub fn execute(
        &self,
        quote_artifact: &Artifact,
        quote: &RouteQuote,
        request: &ProcurementExecutionRequest,
        created_at_unix: u64,
    ) -> Result<SignedProcurementReceipt, RetroQuoteError> {
        validate_quote_artifact(quote_artifact, quote)?;
        let receipt = self.receipt_from_quote(quote_artifact, quote, request)?;
        let receipt_hash = receipt_hash(&receipt)?;
        let receipt_payload =
            PayloadRef::inline_json(&receipt).map_err(RetroQuoteError::Artifact)?;
        let artifact = ArtifactDraft {
            skill: SkillId(PROCUREMENT_EXECUTION_SKILL.to_string()),
            agent: self.agent.clone(),
            topic: format!("procurement completed for quote {}", quote.quote_id),
            input_fingerprint: blake3_hex(
                format!("{}:{receipt_hash}", quote_artifact.content_hash).as_bytes(),
            ),
            output_cid: Some(format!("inline://retroquoter/procured/{receipt_hash}")),
            parent_artifact_ids: vec![quote_artifact.id.clone()],
            schema_tags: BTreeSet::from([SchemaTag(PROCUREMENT_PROCURED_TAG.to_string())]),
            payload: Some(receipt_payload),
        }
        .seal(&self.signer, created_at_unix)
        .map_err(RetroQuoteError::Artifact)?;

        Ok(SignedProcurementReceipt { receipt, artifact })
    }

    fn receipt_from_quote(
        &self,
        quote_artifact: &Artifact,
        quote: &RouteQuote,
        request: &ProcurementExecutionRequest,
    ) -> Result<ProcurementReceipt, RetroQuoteError> {
        let quote_payload_hash = quote_hash(quote)?;
        let procured_line_items = quote
            .line_items
            .iter()
            .enumerate()
            .map(|(index, item)| ProcuredLineItem {
                step_id: item.step_id.clone(),
                reagent_id: item.reagent_id.clone(),
                supplier: item.supplier.clone(),
                sku: item.sku.clone(),
                quantity_milligrams: item.quantity_milligrams,
                line_total_cents: item.line_total_cents,
                supplier_order_id: deterministic_supplier_order_id(
                    &quote.quote_id,
                    &item.reagent_id,
                    index,
                    &request.payment_reference,
                ),
            })
            .collect::<Vec<_>>();
        let receipt_id = format!(
            "receipt_{}",
            &blake3_hex(
                format!(
                    "{}:{}:{}",
                    quote_artifact.id.0, quote_payload_hash, request.payment_reference
                )
                .as_bytes()
            )[..16]
        );
        Ok(ProcurementReceipt {
            receipt_id,
            quote_id: quote.quote_id.clone(),
            quote_artifact_id: quote_artifact.id.clone(),
            quote_content_hash: quote_artifact.content_hash.clone(),
            quote_payload_hash,
            buyer_agent: request.buyer_agent.clone(),
            payment_reference: request.payment_reference.clone(),
            destination_profile_id: request.destination_profile_id.clone(),
            state: ProcurementState::Procured,
            procured_line_items,
            total_cents: quote.total_cents,
            currency: quote.currency.clone(),
        })
    }
}

pub fn receipt_hash(receipt: &ProcurementReceipt) -> Result<String, RetroQuoteError> {
    canonical_bytes(receipt)
        .map(|bytes| blake3_hex(&bytes))
        .map_err(RetroQuoteError::Artifact)
}

const fn ceil_div(numerator: u64, denominator: u64) -> u64 {
    numerator.saturating_add(denominator - 1) / denominator
}

fn deterministic_supplier_order_id(
    quote_id: &str,
    reagent_id: &str,
    index: usize,
    payment_reference: &str,
) -> String {
    format!(
        "ord_{}",
        &blake3_hex(format!("{quote_id}:{reagent_id}:{index}:{payment_reference}").as_bytes())
            [..16]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chimiaclaw_artifact::InMemoryArtifactStore;

    fn signer() -> ArtifactSigner {
        ArtifactSigner::from_seed([9; 32])
    }

    fn route_artifact(signer: &ArtifactSigner, schema_tags: BTreeSet<SchemaTag>) -> Artifact {
        let proposal = proposal();
        let payload = PayloadRef::inline_json(&proposal).expect("route payload");
        ArtifactDraft {
            skill: SkillId("chem.retrosynth.aizynth.v1".to_string()),
            agent: AgentId(PLANNER_AGENT.to_string()),
            topic: "demo aspirin route".to_string(),
            input_fingerprint: "smiles:CC(=O)Oc1ccccc1C(=O)O".to_string(),
            output_cid: Some("zg://retroquoter/routes/demo-aspirin".to_string()),
            parent_artifact_ids: Vec::new(),
            schema_tags,
            payload: Some(payload),
        }
        .seal(signer, 1)
        .expect("route artifact")
    }

    fn proposal() -> RouteProposal {
        RouteProposal {
            route_id: "route-aspirin-demo".to_string(),
            target_smiles: "CC(=O)Oc1ccccc1C(=O)O".to_string(),
            target_scale_milligrams: 5_000,
            steps: vec![RouteStep {
                step_id: "step-1".to_string(),
                description: "Acetylate salicylic acid with acetic anhydride".to_string(),
                requirements: vec![
                    ReagentRequirement {
                        reagent_id: "salicylic-acid".to_string(),
                        display_name: "Salicylic acid".to_string(),
                        quantity_milligrams: 4_000,
                        role: ReagentRole::StartingMaterial,
                    },
                    ReagentRequirement {
                        reagent_id: "acetic-anhydride".to_string(),
                        display_name: "Acetic anhydride".to_string(),
                        quantity_milligrams: 6_000,
                        role: ReagentRole::Reagent,
                    },
                ],
            }],
        }
    }

    fn catalog() -> ReagentCatalog {
        ReagentCatalog::new()
            .with_offer(
                "salicylic-acid",
                SupplierOffer {
                    supplier: "DemoChem".to_string(),
                    sku: "SAL-001".to_string(),
                    unit_price_cents_per_gram: 120,
                    available_grams: 100,
                },
            )
            .with_offer(
                "acetic-anhydride",
                SupplierOffer {
                    supplier: "DemoChem".to_string(),
                    sku: "ACE-002".to_string(),
                    unit_price_cents_per_gram: 80,
                    available_grams: 250,
                },
            )
    }

    fn quoter() -> RetroQuoter {
        RetroQuoter::new(
            AgentId(PROCUREMENT_AGENT.to_string()),
            ArtifactSigner::from_seed([42; 32]),
            catalog(),
        )
    }

    fn execution_request() -> ProcurementExecutionRequest {
        ProcurementExecutionRequest {
            buyer_agent: AgentId("buyer.chimiaclaw.eth".to_string()),
            payment_reference: "uniswap-swap-demo-001".to_string(),
            destination_profile_id: "sofia-lab-default".to_string(),
        }
    }

    fn executor() -> ProcurementExecutor {
        ProcurementExecutor::new(
            AgentId(PROCUREMENT_AGENT.to_string()),
            ArtifactSigner::from_seed([43; 32]),
        )
    }

    fn quote_fixture() -> (InMemoryArtifactStore, Artifact, SignedRouteQuote) {
        let signer = signer();
        let route_artifact = route_artifact(
            &signer,
            BTreeSet::from([SchemaTag(ROUTE_PROPOSAL_TAG.to_string())]),
        );
        let mut store = InMemoryArtifactStore::new();
        store.put(route_artifact.clone()).expect("store route");
        let signed_quote = quoter()
            .quote_route_from_store(&store, &route_artifact.id, &proposal(), 2)
            .expect("quote");
        store
            .put(signed_quote.artifact.clone())
            .expect("store quote");
        (store, route_artifact, signed_quote)
    }

    #[test]
    fn generates_signed_quote_child_artifact_from_dag() {
        let signer = signer();
        let route_artifact = route_artifact(
            &signer,
            BTreeSet::from([SchemaTag(ROUTE_PROPOSAL_TAG.to_string())]),
        );
        let mut store = InMemoryArtifactStore::new();
        store.put(route_artifact.clone()).expect("store route");

        let signed_quote = quoter()
            .quote_route_from_store(&store, &route_artifact.id, &proposal(), 2)
            .expect("quote");

        assert_eq!(signed_quote.quote.subtotal_cents, 960);
        assert_eq!(signed_quote.quote.contingency_cents, 144);
        assert_eq!(signed_quote.quote.total_cents, 1_104);
        assert!(signed_quote.artifact.has_parent(&route_artifact.id));
        assert!(signed_quote
            .artifact
            .schema_tags
            .contains(&SchemaTag(ROUTE_QUOTE_TAG.to_string())));
        signed_quote
            .artifact
            .verify()
            .expect("signed quote verifies");
    }

    #[test]
    fn route_quote_skill_builds_payload_bound_draft() {
        let signer = signer();
        let route_artifact = route_artifact(
            &signer,
            BTreeSet::from([SchemaTag(ROUTE_PROPOSAL_TAG.to_string())]),
        );
        let skill = RouteQuoteSkill::new(catalog());
        let ctx = SkillCtx {
            agent: AgentId("node.local.chimiaclaw.eth".to_string()),
            topic: route_artifact.topic.clone(),
        };

        let draft = skill
            .invoke(&ctx, std::slice::from_ref(&route_artifact))
            .expect("skill draft");
        let quote_artifact = draft
            .seal(&ArtifactSigner::from_seed([44; 32]), 4)
            .expect("seal quote");
        let bytes = quote_artifact
            .inline_payload_bytes()
            .expect("payload bytes")
            .expect("inline payload");
        let quote: RouteQuote = serde_json::from_slice(&bytes).expect("quote payload");

        validate_quote_artifact(&quote_artifact, &quote).expect("valid quote artifact");
        assert!(quote_artifact.has_parent(&route_artifact.id));
        assert_eq!(quote.total_cents, 1_104);
    }

    #[test]
    fn rejects_tampered_route_artifact() {
        let signer = signer();
        let mut route_artifact = route_artifact(
            &signer,
            BTreeSet::from([SchemaTag(ROUTE_PROPOSAL_TAG.to_string())]),
        );
        route_artifact.topic = "tampered".to_string();

        let err = quoter()
            .quote_route(&route_artifact, &proposal(), 2)
            .expect_err("tampered artifact rejected");

        assert!(matches!(
            err,
            RetroQuoteError::Artifact(ArtifactError::ContentHashMismatch { .. })
        ));
    }

    #[test]
    fn rejects_missing_route_proposal_tag() {
        let signer = signer();
        let route_artifact = route_artifact(
            &signer,
            BTreeSet::from([SchemaTag("chem.dft.result".to_string())]),
        );

        let err = quoter()
            .quote_route(&route_artifact, &proposal(), 2)
            .expect_err("wrong schema tag rejected");

        assert_eq!(
            err,
            RetroQuoteError::MissingRouteProposalTag {
                artifact_id: route_artifact.id
            }
        );
    }

    #[test]
    fn rejects_unknown_reagent() {
        let signer = signer();
        let mut bad_proposal = proposal();
        bad_proposal.steps[0].requirements[0].reagent_id = "unknown".to_string();
        let payload = PayloadRef::inline_json(&bad_proposal).expect("route payload");
        let route_artifact = ArtifactDraft {
            skill: SkillId("chem.retrosynth.aizynth.v1".to_string()),
            agent: AgentId(PLANNER_AGENT.to_string()),
            topic: "demo aspirin route with unknown reagent".to_string(),
            input_fingerprint: "smiles:CC(=O)Oc1ccccc1C(=O)O".to_string(),
            output_cid: Some("zg://retroquoter/routes/demo-unknown".to_string()),
            parent_artifact_ids: Vec::new(),
            schema_tags: BTreeSet::from([SchemaTag(ROUTE_PROPOSAL_TAG.to_string())]),
            payload: Some(payload),
        }
        .seal(&signer, 1)
        .expect("route artifact");

        let err = quoter()
            .quote_route(&route_artifact, &bad_proposal, 2)
            .expect_err("unknown reagent rejected");

        assert_eq!(
            err,
            RetroQuoteError::UnknownReagent {
                reagent_id: "unknown".to_string()
            }
        );
    }

    #[test]
    fn executes_quote_into_signed_procured_artifact() {
        let (mut store, _route_artifact, signed_quote) = quote_fixture();

        let receipt = executor()
            .execute_from_store(
                &store,
                &signed_quote.artifact.id,
                &signed_quote.quote,
                &execution_request(),
                3,
            )
            .expect("execute procurement");
        store
            .put(receipt.artifact.clone())
            .expect("store procured artifact");

        assert_eq!(receipt.receipt.state, ProcurementState::Procured);
        assert_eq!(receipt.receipt.total_cents, signed_quote.quote.total_cents);
        assert_eq!(receipt.receipt.procured_line_items.len(), 2);
        assert!(receipt.artifact.has_parent(&signed_quote.artifact.id));
        assert!(receipt
            .artifact
            .schema_tags
            .contains(&SchemaTag(PROCUREMENT_PROCURED_TAG.to_string())));
        receipt
            .artifact
            .verify()
            .expect("procured artifact verifies");
    }

    #[test]
    fn rejects_tampered_quote_artifact_during_execution() {
        let (_store, _route_artifact, mut signed_quote) = quote_fixture();
        signed_quote.artifact.topic = "tampered quote".to_string();

        let err = executor()
            .execute(
                &signed_quote.artifact,
                &signed_quote.quote,
                &execution_request(),
                3,
            )
            .expect_err("tampered quote rejected");

        assert!(matches!(
            err,
            RetroQuoteError::Artifact(ArtifactError::ContentHashMismatch { .. })
        ));
    }

    #[test]
    fn rejects_quote_artifact_without_quote_tag() {
        let (_store, route_artifact, signed_quote) = quote_fixture();
        let quote_hash = quote_hash(&signed_quote.quote).expect("quote hash");
        let payload = PayloadRef::inline_json(&signed_quote.quote).expect("quote payload");
        let wrong_tag_quote_artifact = ArtifactDraft {
            skill: SkillId(ROUTE_QUOTE_SKILL.to_string()),
            agent: AgentId(PROCUREMENT_AGENT.to_string()),
            topic: "supplier quote for route route-aspirin-demo".to_string(),
            input_fingerprint: signed_quote.artifact.input_fingerprint.clone(),
            output_cid: Some(format!("inline://retroquoter/quote/{quote_hash}")),
            parent_artifact_ids: vec![route_artifact.id],
            schema_tags: BTreeSet::from([SchemaTag("chem.dft.result".to_string())]),
            payload: Some(payload),
        }
        .seal(&ArtifactSigner::from_seed([42; 32]), 2)
        .expect("wrong-tag quote artifact");

        let err = executor()
            .execute(
                &wrong_tag_quote_artifact,
                &signed_quote.quote,
                &execution_request(),
                3,
            )
            .expect_err("missing quote tag rejected");

        assert_eq!(
            err,
            RetroQuoteError::MissingRouteQuoteTag {
                artifact_id: wrong_tag_quote_artifact.id
            }
        );
    }

    #[test]
    fn rejects_quote_payload_mismatch_during_execution() {
        let (_store, _route_artifact, mut signed_quote) = quote_fixture();
        signed_quote.quote.total_cents += 1;

        let err = executor()
            .execute(
                &signed_quote.artifact,
                &signed_quote.quote,
                &execution_request(),
                3,
            )
            .expect_err("quote payload mismatch rejected");

        assert!(matches!(err, RetroQuoteError::QuotePayloadMismatch { .. }));
    }
}
