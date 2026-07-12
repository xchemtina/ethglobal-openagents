//! Scientific service transaction primitives for ChimiaClaw.
//!
//! This crate models the common market spine used by DFT, retrosynthesis, and
//! literature agents: an ENS-shaped provider profile, a service offer, a user
//! request, a quote, explicit settlement lifecycle artifacts, and a signed
//! result artifact. It is deterministic and dependency-light so the hackathon
//! demo can run without credentials, funds, or live sponsor services. The
//! payloads are intentionally shaped so ENS, AXL, 0G, Uniswap, and KeeperHub
//! adapters have precise places to attach later.

use chimiaclaw_artifact::{Artifact, ArtifactDraft, ArtifactError, ArtifactSigner, PayloadRef};
use chimiaclaw_moladt::{
    demo_ferrocene_moladt, molecule_artifact, DftBackend, DftJobKind, DftMethodSpec,
    DftMoleculeRef, MolAdtError, MoleculeAdt,
};
use chimiaclaw_schema::{AgentId, Capability, CapabilityKind, SchemaTag, SkillId, StrategySetId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const ENS_AGENT_PROFILE_TAG: &str = "market.ens.agent_profile";
pub const SERVICE_OFFER_TAG: &str = "market.service.offer";
pub const SERVICE_REQUEST_TAG: &str = "market.service.request";
pub const SERVICE_QUOTE_TAG: &str = "market.service.quote";
pub const QUOTE_ACCEPTANCE_TAG: &str = "market.quote.acceptance";
pub const ESCROW_AUTHORIZATION_TAG: &str = "market.escrow.authorization";
pub const SETTLEMENT_INTENT_TAG: &str = "market.settlement.intent";
pub const RESULT_ACKNOWLEDGEMENT_TAG: &str = "market.result.acknowledgement";
pub const SETTLEMENT_RELEASE_TAG: &str = "market.settlement.release";
pub const SETTLEMENT_REFUND_TAG: &str = "market.settlement.refund";
pub const SERVICE_RESULT_TAG: &str = "market.service.result";

pub const RETROSYNTH_REQUEST_TAG: &str = "chem.retrosynth.service_request";
pub const DFT_REQUEST_TAG: &str = "chem.dft.service_request";
pub const LITERATURE_REQUEST_TAG: &str = "science.literature.service_request";

pub const MARKET_PROFILE_SKILL: &str = "market.ens.profile.v1";
pub const MARKET_OFFER_SKILL: &str = "market.service.offer.v1";
pub const MARKET_REQUEST_SKILL: &str = "market.service.request.v1";
pub const MARKET_QUOTE_SKILL: &str = "market.service.quote.v1";
pub const MARKET_QUOTE_ACCEPTANCE_SKILL: &str = "market.quote.acceptance.v1";
pub const MARKET_ESCROW_AUTHORIZATION_SKILL: &str = "market.escrow.authorization.v1";
pub const MARKET_SETTLEMENT_SKILL: &str = "market.settlement.intent.v1";
pub const MARKET_RESULT_ACKNOWLEDGEMENT_SKILL: &str = "market.result.acknowledgement.v1";
pub const MARKET_SETTLEMENT_RELEASE_SKILL: &str = "market.settlement.release.v1";
pub const MARKET_SETTLEMENT_REFUND_SKILL: &str = "market.settlement.refund.v1";
pub const MARKET_RESULT_SKILL: &str = "market.service.result.v1";

pub const USER_AGENT: &str = "operator.chimiaclaw.eth";
pub const RETROSYNTH_AGENT: &str = "retro.service.chimiaclaw.eth";
pub const DFT_AGENT: &str = "dft.service.chimiaclaw.eth";
pub const LITERATURE_AGENT: &str = "literature.service.chimiaclaw.eth";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ServiceKind {
    Retrosynthesis,
    Dft,
    Literature,
}

impl ServiceKind {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retrosynthesis => "retrosynthesis",
            Self::Dft => "dft",
            Self::Literature => "literature",
        }
    }

    #[must_use]
    pub fn request_tag(&self) -> SchemaTag {
        match self {
            Self::Retrosynthesis => SchemaTag(RETROSYNTH_REQUEST_TAG.to_string()),
            Self::Dft => SchemaTag(DFT_REQUEST_TAG.to_string()),
            Self::Literature => SchemaTag(LITERATURE_REQUEST_TAG.to_string()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsTextRecord {
    pub key: String,
    pub value: String,
    pub live_status: LiveStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LiveStatus {
    Fixture,
    PlannedLive,
    Live,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsAgentProfile {
    pub agent: AgentId,
    pub ens_name: String,
    pub address: String,
    pub signing_public_key_hint: String,
    pub axl_peer_id: String,
    pub service_catalog_cid: Option<String>,
    pub head_artifact_cid: Option<String>,
    pub active_strategy_sets: Vec<StrategySetId>,
    pub text_records: Vec<EnsTextRecord>,
    pub capabilities: Vec<Capability>,
    pub live_status: LiveStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsorBindings {
    pub ens_identity: String,
    pub axl_peer_id: String,
    pub zero_g_storage_hint: Option<String>,
    pub uniswap_settlement_hint: Option<String>,
    pub keeperhub_execution_hint: Option<String>,
    /// Optional x402 payTo / facilitator attachment point for agentic HTTP settlement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x402_settlement_hint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceServiceOffer {
    pub offer_id: String,
    pub service_kind: ServiceKind,
    pub provider_agent: AgentId,
    pub provider_ens: String,
    pub title: String,
    pub description: String,
    pub consumes_schema_tags: Vec<SchemaTag>,
    pub produces_schema_tags: Vec<SchemaTag>,
    pub base_price_usdc_micros: u64,
    pub estimated_latency_seconds: u64,
    pub custody_policy: String,
    pub settlement_assets: Vec<String>,
    pub sponsor_bindings: SponsorBindings,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceServiceRequest {
    pub request_id: String,
    pub service_kind: ServiceKind,
    pub requester_agent: AgentId,
    pub target_lab_id: String,
    pub input: ScienceServiceInput,
    pub max_price_usdc_micros: u64,
    pub settlement_asset: String,
    pub required_outputs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScienceServiceInput {
    Retrosynthesis {
        target_smiles: String,
        scale_milligrams: u64,
        constraints: Vec<String>,
    },
    Dft {
        molecule: DftMoleculeRef,
        total_charge: i32,
        multiplicity: u8,
        method: DftMethodSpec,
        job_kind: DftJobKind,
        requested_properties: Vec<String>,
    },
    Literature {
        query: String,
        sector: String,
        sources: Vec<String>,
        open_access_only: bool,
        max_papers: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SciencePaymentTerms {
    pub escrow_required: bool,
    pub payer_confirmation_required: bool,
    pub release_trigger: ScienceReleaseTrigger,
    pub dispute_window_seconds: u64,
    pub refund_window_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScienceReleaseTrigger {
    ResultAcknowledged,
    OperatorSignedRelease,
    KeeperHubCompletion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScienceSettlementMethod {
    SimulatedArtifactLedger,
    UniswapPreparedTransfer,
    OnChainEscrow,
    /// HTTP 402 Payment Required (x402) stablecoin micropayment.
    ///
    /// Live verification is performed by `services/api-gateway` via a
    /// facilitator; this enum marks market quotes that settle through that path.
    /// Payload shapes live in `chimiaclaw-x402` (`market.x402.*` artifacts).
    X402HttpPayment,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceServiceQuote {
    pub quote_id: String,
    pub request_id: String,
    pub service_kind: ServiceKind,
    pub provider_agent: AgentId,
    pub price_usdc_micros: u64,
    pub asset: String,
    pub payment_terms: SciencePaymentTerms,
    pub settlement_method: ScienceSettlementMethod,
    pub estimated_latency_seconds: u64,
    pub expires_at_unix: u64,
    pub assumptions: Vec<String>,
    pub settlement_hint: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceQuoteAcceptance {
    pub acceptance_id: String,
    pub quote_id: String,
    pub request_id: String,
    pub requester_agent: AgentId,
    pub provider_agent: AgentId,
    pub accepted_amount_usdc_micros: u64,
    pub asset: String,
    pub accepted_at_unix: u64,
    pub quote_expires_at_unix: u64,
    pub conditions: Vec<String>,
    pub operator_confirmation_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceRefundPolicy {
    pub refund_to_agent: AgentId,
    pub refundable_amount_usdc_micros: u64,
    pub refund_asset: String,
    pub allowed_reasons: Vec<ScienceRefundReason>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScienceRefundReason {
    QuoteExpired,
    ResultRejected,
    ProviderFailed,
    OperatorCancelledBeforeExecution,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceEscrowAuthorization {
    pub escrow_id: String,
    pub acceptance_id: String,
    pub quote_id: String,
    pub payer_agent: AgentId,
    pub payee_agent: AgentId,
    pub amount_usdc_micros: u64,
    pub asset: String,
    pub settlement_method: ScienceSettlementMethod,
    pub route_hint: String,
    pub live_transfer_prepared: bool,
    pub live_transfer_executed: bool,
    pub release_conditions: Vec<String>,
    pub refund_policy: ScienceRefundPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceSettlementIntent {
    pub settlement_id: String,
    pub escrow_id: String,
    pub quote_id: String,
    pub payer_agent: AgentId,
    pub payee_agent: AgentId,
    pub amount_usdc_micros: u64,
    pub asset: String,
    pub route_hint: String,
    pub settlement_method: ScienceSettlementMethod,
    pub live_execution_required: bool,
    pub operator_confirmation_required: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScienceServiceResult {
    pub result_id: String,
    pub request_id: String,
    pub service_kind: ServiceKind,
    pub provider_agent: AgentId,
    pub outputs: ScienceServiceOutput,
    pub citations: Vec<Citation>,
    pub world_avatar_projection_hints: Vec<String>,
    pub execution_trace: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScienceServiceOutput {
    Retrosynthesis {
        route_summary: String,
        target_smiles: String,
        steps: Vec<String>,
        safety_notes: Vec<String>,
    },
    Dft {
        molecule: String,
        method: String,
        total_energy_hartree: f64,
        homo_lumo_gap_ev: f64,
        dipole_debye: f64,
    },
    Literature {
        synthesis_summary: String,
        extracted_claims: Vec<String>,
        conflicts: Vec<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    pub title: String,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub year: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceResultAcknowledgement {
    pub acknowledgement_id: String,
    pub result_id: String,
    pub request_id: String,
    pub quote_id: String,
    pub escrow_id: String,
    pub requester_agent: AgentId,
    pub provider_agent: AgentId,
    pub status: ScienceResultAcknowledgementStatus,
    pub acknowledged_at_unix: u64,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScienceResultAcknowledgementStatus {
    Accepted,
    NeedsReview,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceSettlementRelease {
    pub release_id: String,
    pub escrow_id: String,
    pub quote_id: String,
    pub result_id: String,
    pub acknowledgement_id: String,
    pub payer_agent: AgentId,
    pub payee_agent: AgentId,
    pub amount_usdc_micros: u64,
    pub asset: String,
    pub release_status: ScienceSettlementReleaseStatus,
    pub released_at_unix: u64,
    pub transaction_ref: Option<String>,
    pub audit_notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScienceSettlementReleaseStatus {
    SimulatedReleased,
    PreparedLiveTransfer,
    PendingOperatorSignature,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScienceSettlementRefund {
    pub refund_id: String,
    pub escrow_id: String,
    pub quote_id: String,
    pub payer_agent: AgentId,
    pub payee_agent: AgentId,
    pub amount_usdc_micros: u64,
    pub asset: String,
    pub reason: ScienceRefundReason,
    pub refund_status: ScienceSettlementRefundStatus,
    pub transaction_ref: Option<String>,
    pub audit_notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScienceSettlementRefundStatus {
    SimulatedRefunded,
    PreparedLiveRefund,
    PendingOperatorSignature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScienceEconomicSettlement {
    pub quote_acceptance: ScienceQuoteAcceptance,
    pub escrow_authorization: ScienceEscrowAuthorization,
    pub result_acknowledgement: ScienceResultAcknowledgement,
    pub settlement_release: ScienceSettlementRelease,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScienceTransactionFlow {
    pub service_kind: ServiceKind,
    pub profile: EnsAgentProfile,
    pub offer: ScienceServiceOffer,
    pub request: ScienceServiceRequest,
    pub quote: ScienceServiceQuote,
    pub economic_settlement: ScienceEconomicSettlement,
    pub settlement_intent: ScienceSettlementIntent,
    pub result: ScienceServiceResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dft_molecule: Option<MoleculeAdt>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedScienceTransactionFlow {
    pub flow: ScienceTransactionFlow,
    pub artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScienceMarketDemo {
    pub demo_id: String,
    pub maturity: String,
    pub current_truth: Vec<String>,
    pub flows: Vec<SignedScienceTransactionFlow>,
    pub sponsor_next_steps: BTreeMap<String, String>,
}

#[derive(Debug)]
pub enum MarketError {
    Artifact(ArtifactError),
    Settlement(String),
}

impl From<ArtifactError> for MarketError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<MolAdtError> for MarketError {
    fn from(value: MolAdtError) -> Self {
        match value {
            MolAdtError::Artifact(error) => Self::Artifact(error),
            other => Self::Settlement(format!("molecule adt error: {other}")),
        }
    }
}

impl ScienceTransactionFlow {
    pub fn validate_economic_settlement(&self) -> Result<(), MarketError> {
        self.economic_settlement.validate(
            &self.request,
            &self.quote,
            &self.settlement_intent,
            &self.result,
        )
    }
}

impl ScienceEconomicSettlement {
    pub fn validate(
        &self,
        request: &ScienceServiceRequest,
        quote: &ScienceServiceQuote,
        settlement_intent: &ScienceSettlementIntent,
        result: &ScienceServiceResult,
    ) -> Result<(), MarketError> {
        if quote.request_id != request.request_id {
            return Err(MarketError::Settlement(
                "quote request_id must match request".to_string(),
            ));
        }
        if quote.price_usdc_micros > request.max_price_usdc_micros {
            return Err(MarketError::Settlement(
                "quote price exceeds request max price".to_string(),
            ));
        }
        if quote.asset != request.settlement_asset {
            return Err(MarketError::Settlement(
                "quote asset must match requested settlement asset".to_string(),
            ));
        }

        let acceptance = &self.quote_acceptance;
        if acceptance.quote_id != quote.quote_id
            || acceptance.request_id != request.request_id
            || acceptance.requester_agent != request.requester_agent
            || acceptance.provider_agent != quote.provider_agent
            || acceptance.accepted_amount_usdc_micros != quote.price_usdc_micros
            || acceptance.asset != quote.asset
        {
            return Err(MarketError::Settlement(
                "quote acceptance does not match request/quote terms".to_string(),
            ));
        }
        if acceptance.accepted_at_unix > acceptance.quote_expires_at_unix {
            return Err(MarketError::Settlement(
                "quote acceptance occurs after quote expiry".to_string(),
            ));
        }

        let escrow = &self.escrow_authorization;
        if escrow.acceptance_id != acceptance.acceptance_id
            || escrow.quote_id != quote.quote_id
            || escrow.payer_agent != request.requester_agent
            || escrow.payee_agent != quote.provider_agent
            || escrow.amount_usdc_micros != quote.price_usdc_micros
            || escrow.asset != quote.asset
            || escrow.live_transfer_executed
        {
            return Err(MarketError::Settlement(
                "escrow authorization does not match accepted quote".to_string(),
            ));
        }
        if escrow.refund_policy.refund_to_agent != escrow.payer_agent
            || escrow.refund_policy.refundable_amount_usdc_micros != escrow.amount_usdc_micros
            || escrow.refund_policy.refund_asset != escrow.asset
        {
            return Err(MarketError::Settlement(
                "refund policy must return the full escrow amount to the payer".to_string(),
            ));
        }

        if settlement_intent.escrow_id != escrow.escrow_id
            || settlement_intent.quote_id != quote.quote_id
            || settlement_intent.payer_agent != escrow.payer_agent
            || settlement_intent.payee_agent != escrow.payee_agent
            || settlement_intent.amount_usdc_micros != escrow.amount_usdc_micros
            || settlement_intent.asset != escrow.asset
            || settlement_intent.settlement_method != escrow.settlement_method
        {
            return Err(MarketError::Settlement(
                "settlement intent does not match escrow authorization".to_string(),
            ));
        }

        let acknowledgement = &self.result_acknowledgement;
        if result.request_id != request.request_id
            || result.provider_agent != quote.provider_agent
            || acknowledgement.result_id != result.result_id
            || acknowledgement.request_id != request.request_id
            || acknowledgement.quote_id != quote.quote_id
            || acknowledgement.escrow_id != escrow.escrow_id
            || acknowledgement.requester_agent != request.requester_agent
            || acknowledgement.provider_agent != quote.provider_agent
        {
            return Err(MarketError::Settlement(
                "result acknowledgement does not match result/request/escrow".to_string(),
            ));
        }

        let release = &self.settlement_release;
        if acknowledgement.status != ScienceResultAcknowledgementStatus::Accepted {
            return Err(MarketError::Settlement(
                "settlement release requires an accepted result acknowledgement".to_string(),
            ));
        }
        if release.escrow_id != escrow.escrow_id
            || release.quote_id != quote.quote_id
            || release.result_id != result.result_id
            || release.acknowledgement_id != acknowledgement.acknowledgement_id
            || release.payer_agent != escrow.payer_agent
            || release.payee_agent != escrow.payee_agent
            || release.amount_usdc_micros != escrow.amount_usdc_micros
            || release.asset != escrow.asset
        {
            return Err(MarketError::Settlement(
                "settlement release does not match escrow/result acknowledgement".to_string(),
            ));
        }
        Ok(())
    }
}

#[must_use]
pub fn demo_science_market() -> ScienceMarketDemo {
    let flows = vec![
        sign_flow(retrosynthesis_flow(), 10).expect("retrosynthesis flow signs"),
        sign_flow(dft_flow(), 100).expect("dft flow signs"),
        sign_flow(literature_flow(), 200).expect("literature flow signs"),
    ];
    ScienceMarketDemo {
        demo_id: "SCIENCE.MARKET.DEMO.001".to_string(),
        maturity: "deterministic-local-fixture".to_string(),
        current_truth: vec![
            "Artifacts are signed and payload-bound.".to_string(),
            "ENS records are shaped as text-record fixtures; live ENS resolution is the next integration step.".to_string(),
            "Settlement lifecycle artifacts are non-custodial and do not move funds.".to_string(),
            "Quote acceptance, escrow authorization, result acknowledgement, and release are signed audit records.".to_string(),
            "DFT and literature outputs are deterministic fixtures until live workers are attached.".to_string(),
        ],
        flows,
        sponsor_next_steps: BTreeMap::from([
            (
                "ENS".to_string(),
                "resolve provider ENS text records from a configured RPC endpoint".to_string(),
            ),
            (
                "Gensyn AXL".to_string(),
                "send service request and signed result artifacts across two AXL nodes".to_string(),
            ),
            (
                "0G".to_string(),
                "persist large request/result payloads and service catalog roots in 0G Storage".to_string(),
            ),
            (
                "Uniswap".to_string(),
                "replace simulated artifact-ledger releases with Uniswap API quotes and prepared transfers".to_string(),
            ),
            (
                "KeeperHub".to_string(),
                "schedule DFT or settlement job artifacts through KeeperHub CLI/MCP".to_string(),
            ),
        ]),
    }
}

fn sign_flow(
    mut flow: ScienceTransactionFlow,
    seed_offset: u8,
) -> Result<SignedScienceTransactionFlow, MarketError> {
    flow.validate_economic_settlement()?;
    let provider_signer = ArtifactSigner::from_seed([seed_offset; 32]);
    let requester_signer = ArtifactSigner::from_seed([seed_offset.saturating_add(1); 32]);

    let profile = artifact_draft(
        MARKET_PROFILE_SKILL,
        &flow.profile.agent,
        format!("ENS-shaped profile for {}", flow.profile.ens_name),
        format!("ens-profile:{}", flow.profile.ens_name),
        vec![SchemaTag(ENS_AGENT_PROFILE_TAG.to_string())],
        &flow.profile,
        Vec::new(),
    )?
    .seal(&provider_signer, u64::from(seed_offset))?;

    let offer = artifact_draft(
        MARKET_OFFER_SKILL,
        &flow.offer.provider_agent,
        format!("science service offer {}", flow.offer.offer_id),
        flow.offer.offer_id.clone(),
        vec![SchemaTag(SERVICE_OFFER_TAG.to_string())],
        &flow.offer,
        vec![profile.id.clone()],
    )?
    .seal(&provider_signer, u64::from(seed_offset) + 1)?;

    let molecule_artifact_opt = if let Some(molecule) = flow.dft_molecule.as_ref() {
        let artifact = molecule_artifact(
            molecule,
            flow.request.requester_agent.clone(),
            &requester_signer,
            u64::from(seed_offset).saturating_add(100),
        )?;
        if let ScienceServiceInput::Dft {
            molecule: ref mut molecule_ref,
            ..
        } = flow.request.input
        {
            *molecule_ref = molecule_ref.clone().with_artifact(&artifact);
        }
        Some(artifact)
    } else {
        None
    };

    let mut request_parents = vec![offer.id.clone()];
    if let Some(molecule) = molecule_artifact_opt.as_ref() {
        request_parents.push(molecule.id.clone());
    }

    let request = artifact_draft(
        MARKET_REQUEST_SKILL,
        &flow.request.requester_agent,
        format!("science service request {}", flow.request.request_id),
        flow.request.request_id.clone(),
        vec![
            SchemaTag(SERVICE_REQUEST_TAG.to_string()),
            flow.request.service_kind.request_tag(),
        ],
        &flow.request,
        request_parents,
    )?
    .seal(&requester_signer, u64::from(seed_offset) + 2)?;

    let quote = artifact_draft(
        MARKET_QUOTE_SKILL,
        &flow.quote.provider_agent,
        format!("science service quote {}", flow.quote.quote_id),
        flow.quote.quote_id.clone(),
        vec![SchemaTag(SERVICE_QUOTE_TAG.to_string())],
        &flow.quote,
        vec![request.id.clone()],
    )?
    .seal(&provider_signer, u64::from(seed_offset) + 3)?;

    let acceptance = artifact_draft(
        MARKET_QUOTE_ACCEPTANCE_SKILL,
        &flow.economic_settlement.quote_acceptance.requester_agent,
        format!(
            "quote acceptance {}",
            flow.economic_settlement.quote_acceptance.acceptance_id
        ),
        flow.economic_settlement
            .quote_acceptance
            .acceptance_id
            .clone(),
        vec![SchemaTag(QUOTE_ACCEPTANCE_TAG.to_string())],
        &flow.economic_settlement.quote_acceptance,
        vec![quote.id.clone()],
    )?
    .seal(&requester_signer, u64::from(seed_offset) + 4)?;

    let escrow = artifact_draft(
        MARKET_ESCROW_AUTHORIZATION_SKILL,
        &flow.economic_settlement.escrow_authorization.payer_agent,
        format!(
            "escrow authorization {}",
            flow.economic_settlement.escrow_authorization.escrow_id
        ),
        flow.economic_settlement
            .escrow_authorization
            .escrow_id
            .clone(),
        vec![SchemaTag(ESCROW_AUTHORIZATION_TAG.to_string())],
        &flow.economic_settlement.escrow_authorization,
        vec![acceptance.id.clone()],
    )?
    .seal(&requester_signer, u64::from(seed_offset) + 5)?;

    let settlement = artifact_draft(
        MARKET_SETTLEMENT_SKILL,
        &flow.settlement_intent.payer_agent,
        format!("settlement intent {}", flow.settlement_intent.settlement_id),
        flow.settlement_intent.settlement_id.clone(),
        vec![SchemaTag(SETTLEMENT_INTENT_TAG.to_string())],
        &flow.settlement_intent,
        vec![escrow.id.clone()],
    )?
    .seal(&requester_signer, u64::from(seed_offset) + 6)?;

    let result = artifact_draft(
        MARKET_RESULT_SKILL,
        &flow.result.provider_agent,
        format!("science service result {}", flow.result.result_id),
        flow.result.result_id.clone(),
        vec![SchemaTag(SERVICE_RESULT_TAG.to_string())],
        &flow.result,
        vec![request.id.clone(), settlement.id.clone()],
    )?
    .seal(&provider_signer, u64::from(seed_offset) + 7)?;

    let acknowledgement = artifact_draft(
        MARKET_RESULT_ACKNOWLEDGEMENT_SKILL,
        &flow
            .economic_settlement
            .result_acknowledgement
            .requester_agent,
        format!(
            "result acknowledgement {}",
            flow.economic_settlement
                .result_acknowledgement
                .acknowledgement_id
        ),
        flow.economic_settlement
            .result_acknowledgement
            .acknowledgement_id
            .clone(),
        vec![SchemaTag(RESULT_ACKNOWLEDGEMENT_TAG.to_string())],
        &flow.economic_settlement.result_acknowledgement,
        vec![result.id.clone(), escrow.id.clone()],
    )?
    .seal(&requester_signer, u64::from(seed_offset) + 8)?;

    let release = artifact_draft(
        MARKET_SETTLEMENT_RELEASE_SKILL,
        &flow.economic_settlement.settlement_release.payer_agent,
        format!(
            "settlement release {}",
            flow.economic_settlement.settlement_release.release_id
        ),
        flow.economic_settlement
            .settlement_release
            .release_id
            .clone(),
        vec![SchemaTag(SETTLEMENT_RELEASE_TAG.to_string())],
        &flow.economic_settlement.settlement_release,
        vec![acknowledgement.id.clone(), settlement.id.clone()],
    )?
    .seal(&requester_signer, u64::from(seed_offset) + 9)?;

    let mut artifacts = vec![profile, offer];
    if let Some(molecule) = molecule_artifact_opt {
        artifacts.push(molecule);
    }
    artifacts.extend([
        request,
        quote,
        acceptance,
        escrow,
        settlement,
        result,
        acknowledgement,
        release,
    ]);
    Ok(SignedScienceTransactionFlow { flow, artifacts })
}

fn artifact_draft<T: Serialize>(
    skill: &str,
    agent: &AgentId,
    topic: String,
    input_fingerprint: String,
    tags: Vec<SchemaTag>,
    payload: &T,
    parents: Vec<chimiaclaw_artifact::ArtifactId>,
) -> Result<ArtifactDraft, MarketError> {
    Ok(ArtifactDraft {
        skill: SkillId(skill.to_string()),
        agent: agent.clone(),
        topic,
        input_fingerprint,
        output_cid: None,
        parent_artifact_ids: parents,
        schema_tags: BTreeSet::from_iter(tags),
        payload: Some(PayloadRef::inline_json(payload)?),
    })
}

fn retrosynthesis_flow() -> ScienceTransactionFlow {
    let profile = provider_profile(
        RETROSYNTH_AGENT,
        "retro.service.chimiaclaw.eth",
        "0x0000000000000000000000000000000000000a11",
        "axl-retro-demo-peer",
        vec![capability(
            "cap.retrosynthesis.quote",
            CapabilityKind::Chemistry,
            vec![RETROSYNTH_REQUEST_TAG],
            vec![
                "chem.retrosynth.route_proposal",
                "chem.procurement.route_quote",
            ],
        )],
    );
    let offer = service_offer(
        "OFFER.RETRO.001",
        ServiceKind::Retrosynthesis,
        &profile,
        "Retrosynthesis route quote",
        "Propose and price a synthesis route with custody warnings.",
        vec![RETROSYNTH_REQUEST_TAG],
        vec![
            "chem.retrosynth.route_proposal",
            "chem.procurement.route_quote",
        ],
        2_500_000,
        45,
        "virtual planning only; wetlab execution requires safety and human custody gates",
    );
    let request = ScienceServiceRequest {
        request_id: "REQ.RETRO.ASPIRIN.001".to_string(),
        service_kind: ServiceKind::Retrosynthesis,
        requester_agent: AgentId(USER_AGENT.to_string()),
        target_lab_id: "LAB.CHIMIA.01".to_string(),
        input: ScienceServiceInput::Retrosynthesis {
            target_smiles: "CC(=O)Oc1ccccc1C(=O)O".to_string(),
            scale_milligrams: 5_000,
            constraints: vec![
                "prefer commodity reagents".to_string(),
                "block wetlab execution until safety artifact exists".to_string(),
            ],
        },
        max_price_usdc_micros: 5_000_000,
        settlement_asset: "USDC".to_string(),
        required_outputs: vec![
            "route proposal".to_string(),
            "supplier quote".to_string(),
            "custody warnings".to_string(),
        ],
    };
    let quote = quote(
        "QUOTE.RETRO.ASPIRIN.001",
        &request,
        &profile.agent,
        3_100_000,
        60,
    );
    let economic_settlement = economic_settlement(
        "RETRO.ASPIRIN.001",
        &request,
        &quote,
        &profile.agent,
        "RESULT.RETRO.ASPIRIN.001",
    );
    let settlement_intent = settlement(
        "SETTLE.RETRO.ASPIRIN.001",
        &quote,
        &economic_settlement.escrow_authorization,
    );
    let result = ScienceServiceResult {
        result_id: "RESULT.RETRO.ASPIRIN.001".to_string(),
        request_id: request.request_id.clone(),
        service_kind: ServiceKind::Retrosynthesis,
        provider_agent: profile.agent.clone(),
        outputs: ScienceServiceOutput::Retrosynthesis {
            route_summary: "Acetylate salicylic acid with acetic anhydride; route remains virtual until safety gate passes.".to_string(),
            target_smiles: "CC(=O)Oc1ccccc1C(=O)O".to_string(),
            steps: vec!["salicylic acid + acetic anhydride -> aspirin".to_string()],
            safety_notes: vec![
                "acetic anhydride is corrosive".to_string(),
                "operator approval required before procurement or wetlab execution".to_string(),
            ],
        },
        citations: vec![Citation {
            title: "Open Reaction Database style route record".to_string(),
            doi: None,
            url: Some("https://open-reaction-database.org".to_string()),
            year: 2024,
        }],
        world_avatar_projection_hints: vec![
            "ontoreaction:hasReactant salicylic-acid".to_string(),
            "prov:wasGeneratedBy retrosynthesis service".to_string(),
        ],
        execution_trace: vec![
            "resolved ENS-shaped provider profile".to_string(),
            "priced deterministic route proposal".to_string(),
            "emitted settlement lifecycle requiring operator acknowledgement".to_string(),
        ],
    };
    ScienceTransactionFlow {
        service_kind: ServiceKind::Retrosynthesis,
        profile,
        offer,
        request,
        quote,
        economic_settlement,
        settlement_intent,
        result,
        dft_molecule: None,
    }
}

fn dft_flow() -> ScienceTransactionFlow {
    let profile = provider_profile(
        DFT_AGENT,
        "dft.service.chimiaclaw.eth",
        "0x0000000000000000000000000000000000000d47",
        "axl-dft-demo-peer",
        vec![capability(
            "cap.dft.single_point",
            CapabilityKind::Execution,
            vec![DFT_REQUEST_TAG],
            vec!["chem.dft.result"],
        )],
    );
    let offer = service_offer(
        "OFFER.DFT.001",
        ServiceKind::Dft,
        &profile,
        "DFT single-point calculation",
        "Return deterministic fixture properties for a molecule; live compute will attach through KeeperHub/0G.",
        vec![DFT_REQUEST_TAG],
        vec!["chem.dft.result"],
        7_500_000,
        900,
        "compute-only; no physical custody",
    );
    let molecule = demo_ferrocene_moladt();
    let request = ScienceServiceRequest {
        request_id: "REQ.DFT.FERROCENE.001".to_string(),
        service_kind: ServiceKind::Dft,
        requester_agent: AgentId(USER_AGENT.to_string()),
        target_lab_id: "LAB.VIRTUAL.01".to_string(),
        input: ScienceServiceInput::Dft {
            molecule: DftMoleculeRef::unbound(&molecule),
            total_charge: molecule.total_formal_charge(),
            multiplicity: 1,
            method: DftMethodSpec {
                functional: "skala-1.1".to_string(),
                basis_set: "def2-tzvp".to_string(),
                backend: DftBackend::PyScf,
                dispersion: Some("dftd3".to_string()),
                grid_level: Some(3),
            },
            job_kind: DftJobKind::SinglePoint,
            requested_properties: vec![
                "total_energy".to_string(),
                "homo_lumo_gap".to_string(),
                "dipole".to_string(),
            ],
        },
        max_price_usdc_micros: 10_000_000,
        settlement_asset: "USDC".to_string(),
        required_outputs: vec![
            "energy".to_string(),
            "frontier orbital gap".to_string(),
            "World Avatar projection hints".to_string(),
        ],
    };
    let quote = quote(
        "QUOTE.DFT.FERROCENE.001",
        &request,
        &profile.agent,
        8_250_000,
        1_200,
    );
    let economic_settlement = economic_settlement(
        "DFT.FERROCENE.001",
        &request,
        &quote,
        &profile.agent,
        "RESULT.DFT.FERROCENE.001",
    );
    let settlement_intent = settlement(
        "SETTLE.DFT.FERROCENE.001",
        &quote,
        &economic_settlement.escrow_authorization,
    );
    let result = ScienceServiceResult {
        result_id: "RESULT.DFT.FERROCENE.001".to_string(),
        request_id: request.request_id.clone(),
        service_kind: ServiceKind::Dft,
        provider_agent: profile.agent.clone(),
        outputs: ScienceServiceOutput::Dft {
            molecule: "ferrocene".to_string(),
            method: "r2SCAN-3c/def2-mTZVPP fixture".to_string(),
            total_energy_hartree: -1650.248_731,
            homo_lumo_gap_ev: 5.42,
            dipole_debye: 0.0,
        },
        citations: vec![Citation {
            title: "r2SCAN-3c composite electronic-structure method".to_string(),
            doi: Some("10.1063/5.0040021".to_string()),
            url: None,
            year: 2021,
        }],
        world_avatar_projection_hints: vec![
            "ontospecies:hasMolecularComputation RESULT.DFT.FERROCENE.001".to_string(),
            "prov:wasAssociatedWith dft.service.chimiaclaw.eth".to_string(),
        ],
        execution_trace: vec![
            "quoted DFT job artifact".to_string(),
            "would schedule through KeeperHub in live mode".to_string(),
            "would store raw output bundle on 0G Storage in live mode".to_string(),
        ],
    };
    ScienceTransactionFlow {
        service_kind: ServiceKind::Dft,
        profile,
        offer,
        request,
        quote,
        economic_settlement,
        settlement_intent,
        result,
        dft_molecule: Some(molecule),
    }
}

fn literature_flow() -> ScienceTransactionFlow {
    let profile = provider_profile(
        LITERATURE_AGENT,
        "literature.service.chimiaclaw.eth",
        "0x0000000000000000000000000000000000001147",
        "axl-literature-demo-peer",
        vec![capability(
            "cap.literature.synthesis",
            CapabilityKind::Storage,
            vec![LITERATURE_REQUEST_TAG],
            vec!["science.literature.synthesis"],
        )],
    );
    let offer = service_offer(
        "OFFER.LIT.001",
        ServiceKind::Literature,
        &profile,
        "Literature synthesis and claim extraction",
        "Open-access-first literature synthesis with citations and conflict notes.",
        vec![LITERATURE_REQUEST_TAG],
        vec!["science.literature.synthesis"],
        1_250_000,
        120,
        "data-only; source citations required",
    );
    let request = ScienceServiceRequest {
        request_id: "REQ.LIT.FLOW.001".to_string(),
        service_kind: ServiceKind::Literature,
        requester_agent: AgentId(USER_AGENT.to_string()),
        target_lab_id: "LAB.CHIMIA.03".to_string(),
        input: ScienceServiceInput::Literature {
            query: "continuous-flow Buchwald-Hartwig amination automation".to_string(),
            sector: "automated-synthesis".to_string(),
            sources: vec![
                "ChemRxiv".to_string(),
                "Crossref".to_string(),
                "Unpaywall".to_string(),
                "arXiv chem-ph".to_string(),
            ],
            open_access_only: true,
            max_papers: 12,
        },
        max_price_usdc_micros: 2_000_000,
        settlement_asset: "USDC".to_string(),
        required_outputs: vec![
            "claim list".to_string(),
            "citation list".to_string(),
            "conflict notes".to_string(),
        ],
    };
    let quote = quote(
        "QUOTE.LIT.FLOW.001",
        &request,
        &profile.agent,
        1_500_000,
        180,
    );
    let economic_settlement = economic_settlement(
        "LIT.FLOW.001",
        &request,
        &quote,
        &profile.agent,
        "RESULT.LIT.FLOW.001",
    );
    let settlement_intent = settlement(
        "SETTLE.LIT.FLOW.001",
        &quote,
        &economic_settlement.escrow_authorization,
    );
    let result = ScienceServiceResult {
        result_id: "RESULT.LIT.FLOW.001".to_string(),
        request_id: request.request_id.clone(),
        service_kind: ServiceKind::Literature,
        provider_agent: profile.agent.clone(),
        outputs: ScienceServiceOutput::Literature {
            synthesis_summary: "Closed-loop flow chemistry literature supports inline analytics plus Bayesian optimization, but hardware portability remains the main unsolved boundary.".to_string(),
            extracted_claims: vec![
                "Inline FTIR/HPLC feedback can reduce optimization cycles versus random search.".to_string(),
                "XDL-style protocol portability requires explicit hardware abstraction.".to_string(),
                "Human review is still needed before wetlab transfer.".to_string(),
            ],
            conflicts: vec![
                "Yield comparisons across platforms often use non-identical residence-time conventions.".to_string(),
            ],
        },
        citations: vec![
            Citation {
                title: "Self-optimising reactions in flow chemistry".to_string(),
                doi: Some("10.1039/C8RE00060H".to_string()),
                url: None,
                year: 2018,
            },
            Citation {
                title: "Towards a Universal XDL Interpreter for Heterogeneous Chemputer Platforms".to_string(),
                doi: None,
                url: Some("https://chemrxiv.org".to_string()),
                year: 2025,
            },
        ],
        world_avatar_projection_hints: vec![
            "prov:wasDerivedFrom cited literature artifacts".to_string(),
            "ontochimia:hasExtractedClaim continuous-flow-automation".to_string(),
        ],
        execution_trace: vec![
            "queried open-access-only source fixture".to_string(),
            "extracted citation-bound claims".to_string(),
            "flagged one cross-platform comparison conflict".to_string(),
        ],
    };
    ScienceTransactionFlow {
        service_kind: ServiceKind::Literature,
        profile,
        offer,
        request,
        quote,
        economic_settlement,
        settlement_intent,
        result,
        dft_molecule: None,
    }
}

fn provider_profile(
    agent: &str,
    ens_name: &str,
    address: &str,
    axl_peer_id: &str,
    capabilities: Vec<Capability>,
) -> EnsAgentProfile {
    EnsAgentProfile {
        agent: AgentId(agent.to_string()),
        ens_name: ens_name.to_string(),
        address: address.to_string(),
        signing_public_key_hint: "ed25519:demo-key-bound-in-artifact-signature".to_string(),
        axl_peer_id: axl_peer_id.to_string(),
        service_catalog_cid: Some(format!("0g://service-catalog/{ens_name}")),
        head_artifact_cid: Some(format!("0g://artifact-head/{agent}")),
        active_strategy_sets: vec![StrategySetId("strategy.science-market.v1".to_string())],
        text_records: vec![
            EnsTextRecord {
                key: "chimiaclaw.agent".to_string(),
                value: agent.to_string(),
                live_status: LiveStatus::Fixture,
            },
            EnsTextRecord {
                key: "chimiaclaw.axl.peer".to_string(),
                value: axl_peer_id.to_string(),
                live_status: LiveStatus::PlannedLive,
            },
            EnsTextRecord {
                key: "chimiaclaw.service.catalog".to_string(),
                value: format!("0g://service-catalog/{ens_name}"),
                live_status: LiveStatus::PlannedLive,
            },
        ],
        capabilities,
        live_status: LiveStatus::Fixture,
    }
}

fn capability(
    id: &str,
    kind: CapabilityKind,
    consumes: Vec<&str>,
    produces: Vec<&str>,
) -> Capability {
    Capability {
        id: id.to_string(),
        kind,
        consumes: consumes
            .into_iter()
            .map(|tag| SchemaTag(tag.to_string()))
            .collect(),
        produces: produces
            .into_iter()
            .map(|tag| SchemaTag(tag.to_string()))
            .collect(),
    }
}

fn service_offer(
    offer_id: &str,
    service_kind: ServiceKind,
    profile: &EnsAgentProfile,
    title: &str,
    description: &str,
    consumes: Vec<&str>,
    produces: Vec<&str>,
    base_price_usdc_micros: u64,
    estimated_latency_seconds: u64,
    custody_policy: &str,
) -> ScienceServiceOffer {
    ScienceServiceOffer {
        offer_id: offer_id.to_string(),
        service_kind,
        provider_agent: profile.agent.clone(),
        provider_ens: profile.ens_name.clone(),
        title: title.to_string(),
        description: description.to_string(),
        consumes_schema_tags: consumes
            .into_iter()
            .map(|tag| SchemaTag(tag.to_string()))
            .collect(),
        produces_schema_tags: produces
            .into_iter()
            .map(|tag| SchemaTag(tag.to_string()))
            .collect(),
        base_price_usdc_micros,
        estimated_latency_seconds,
        custody_policy: custody_policy.to_string(),
        settlement_assets: vec!["USDC".to_string(), "testnet-USDC".to_string()],
        sponsor_bindings: SponsorBindings {
            ens_identity: profile.ens_name.clone(),
            axl_peer_id: profile.axl_peer_id.clone(),
            zero_g_storage_hint: profile.service_catalog_cid.clone(),
            uniswap_settlement_hint: Some(
                "Uniswap API quote replaces artifact-ledger route hint".to_string(),
            ),
            keeperhub_execution_hint: Some(
                "KeeperHub schedules execution artifact in live mode".to_string(),
            ),
            x402_settlement_hint: Some(
                "x402 HTTP 402 micropayment via services/api-gateway; see chimiaclaw-x402"
                    .to_string(),
            ),
        },
    }
}

fn quote(
    quote_id: &str,
    request: &ScienceServiceRequest,
    provider_agent: &AgentId,
    price_usdc_micros: u64,
    estimated_latency_seconds: u64,
) -> ScienceServiceQuote {
    ScienceServiceQuote {
        quote_id: quote_id.to_string(),
        request_id: request.request_id.clone(),
        service_kind: request.service_kind.clone(),
        provider_agent: provider_agent.clone(),
        price_usdc_micros,
        asset: request.settlement_asset.clone(),
        payment_terms: SciencePaymentTerms {
            escrow_required: true,
            payer_confirmation_required: true,
            release_trigger: ScienceReleaseTrigger::ResultAcknowledged,
            dispute_window_seconds: 86_400,
            refund_window_seconds: 604_800,
        },
        settlement_method: ScienceSettlementMethod::SimulatedArtifactLedger,
        estimated_latency_seconds,
        expires_at_unix: 1_777_777_777,
        assumptions: vec![
            "deterministic local fixture".to_string(),
            "live sponsor integrations not invoked".to_string(),
            "operator confirmation required before real settlement".to_string(),
        ],
        settlement_hint: "artifact-ledger:simulated-release".to_string(),
    }
}

fn economic_settlement(
    suffix: &str,
    request: &ScienceServiceRequest,
    quote: &ScienceServiceQuote,
    provider_agent: &AgentId,
    result_id: &str,
) -> ScienceEconomicSettlement {
    let quote_acceptance = ScienceQuoteAcceptance {
        acceptance_id: format!("ACCEPT.{suffix}"),
        quote_id: quote.quote_id.clone(),
        request_id: request.request_id.clone(),
        requester_agent: request.requester_agent.clone(),
        provider_agent: provider_agent.clone(),
        accepted_amount_usdc_micros: quote.price_usdc_micros,
        asset: quote.asset.clone(),
        accepted_at_unix: 1_700_000_010,
        quote_expires_at_unix: quote.expires_at_unix,
        conditions: vec![
            "escrow authorization must be signed before result release".to_string(),
            "release requires requester acknowledgement of the result artifact".to_string(),
            "live token transfer remains disabled in fixture mode".to_string(),
        ],
        operator_confirmation_required: true,
    };
    let escrow_authorization = ScienceEscrowAuthorization {
        escrow_id: format!("ESCROW.{suffix}"),
        acceptance_id: quote_acceptance.acceptance_id.clone(),
        quote_id: quote.quote_id.clone(),
        payer_agent: request.requester_agent.clone(),
        payee_agent: provider_agent.clone(),
        amount_usdc_micros: quote.price_usdc_micros,
        asset: quote.asset.clone(),
        settlement_method: quote.settlement_method.clone(),
        route_hint: quote.settlement_hint.clone(),
        live_transfer_prepared: false,
        live_transfer_executed: false,
        release_conditions: vec![
            "provider result artifact verifies".to_string(),
            "result request_id matches the accepted request".to_string(),
            "requester acknowledgement status is accepted".to_string(),
        ],
        refund_policy: ScienceRefundPolicy {
            refund_to_agent: request.requester_agent.clone(),
            refundable_amount_usdc_micros: quote.price_usdc_micros,
            refund_asset: quote.asset.clone(),
            allowed_reasons: vec![
                ScienceRefundReason::QuoteExpired,
                ScienceRefundReason::ResultRejected,
                ScienceRefundReason::ProviderFailed,
                ScienceRefundReason::OperatorCancelledBeforeExecution,
            ],
        },
    };
    let result_acknowledgement = ScienceResultAcknowledgement {
        acknowledgement_id: format!("ACK.{suffix}"),
        result_id: result_id.to_string(),
        request_id: request.request_id.clone(),
        quote_id: quote.quote_id.clone(),
        escrow_id: escrow_authorization.escrow_id.clone(),
        requester_agent: request.requester_agent.clone(),
        provider_agent: provider_agent.clone(),
        status: ScienceResultAcknowledgementStatus::Accepted,
        acknowledged_at_unix: 1_700_000_020,
        notes: vec![
            "result artifact payload and signature verified".to_string(),
            "fixture acknowledgement authorizes simulated release".to_string(),
        ],
    };
    let settlement_release = ScienceSettlementRelease {
        release_id: format!("RELEASE.{suffix}"),
        escrow_id: escrow_authorization.escrow_id.clone(),
        quote_id: quote.quote_id.clone(),
        result_id: result_id.to_string(),
        acknowledgement_id: result_acknowledgement.acknowledgement_id.clone(),
        payer_agent: request.requester_agent.clone(),
        payee_agent: provider_agent.clone(),
        amount_usdc_micros: quote.price_usdc_micros,
        asset: quote.asset.clone(),
        release_status: ScienceSettlementReleaseStatus::SimulatedReleased,
        released_at_unix: 1_700_000_030,
        transaction_ref: Some(format!("artifact-ledger://release/{suffix}")),
        audit_notes: vec![
            "no live funds moved".to_string(),
            "release is an auditable signed artifact for later adapter execution".to_string(),
        ],
    };
    ScienceEconomicSettlement {
        quote_acceptance,
        escrow_authorization,
        result_acknowledgement,
        settlement_release,
    }
}

fn settlement(
    settlement_id: &str,
    quote: &ScienceServiceQuote,
    escrow: &ScienceEscrowAuthorization,
) -> ScienceSettlementIntent {
    ScienceSettlementIntent {
        settlement_id: settlement_id.to_string(),
        escrow_id: escrow.escrow_id.clone(),
        quote_id: quote.quote_id.clone(),
        payer_agent: escrow.payer_agent.clone(),
        payee_agent: escrow.payee_agent.clone(),
        amount_usdc_micros: quote.price_usdc_micros,
        asset: quote.asset.clone(),
        route_hint: escrow.route_hint.clone(),
        settlement_method: escrow.settlement_method.clone(),
        live_execution_required: false,
        operator_confirmation_required: true,
    }
}

pub fn refund_after_rejection(
    refund_id: impl Into<String>,
    escrow: &ScienceEscrowAuthorization,
    acknowledgement: &ScienceResultAcknowledgement,
    reason: ScienceRefundReason,
) -> Result<ScienceSettlementRefund, MarketError> {
    if acknowledgement.status == ScienceResultAcknowledgementStatus::Accepted {
        return Err(MarketError::Settlement(
            "accepted results must release settlement instead of refunding".to_string(),
        ));
    }
    if !escrow.refund_policy.allowed_reasons.contains(&reason) {
        return Err(MarketError::Settlement(
            "refund reason is not allowed by escrow policy".to_string(),
        ));
    }
    Ok(ScienceSettlementRefund {
        refund_id: refund_id.into(),
        escrow_id: escrow.escrow_id.clone(),
        quote_id: escrow.quote_id.clone(),
        payer_agent: escrow.payer_agent.clone(),
        payee_agent: escrow.payee_agent.clone(),
        amount_usdc_micros: escrow.refund_policy.refundable_amount_usdc_micros,
        asset: escrow.refund_policy.refund_asset.clone(),
        reason,
        refund_status: ScienceSettlementRefundStatus::SimulatedRefunded,
        transaction_ref: Some(format!("artifact-ledger://refund/{}", escrow.escrow_id)),
        audit_notes: vec![
            "no live funds moved".to_string(),
            "refund is represented as a signed artifact before any live adapter is attached"
                .to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_market_has_three_signed_flows() {
        let demo = demo_science_market();
        assert_eq!(demo.flows.len(), 3);
        for flow in &demo.flows {
            flow.flow
                .validate_economic_settlement()
                .expect("settlement validates");
            for artifact in &flow.artifacts {
                artifact.verify().expect("artifact verifies");
                assert!(artifact.payload.is_some(), "payload-bound artifact");
            }
            let molecule_offset = match flow.flow.service_kind {
                ServiceKind::Dft => {
                    assert_eq!(flow.artifacts.len(), 11);
                    1
                }
                _ => {
                    assert_eq!(flow.artifacts.len(), 10);
                    0
                }
            };
            let profile_idx = 0;
            let offer_idx = 1;
            let request_idx = 2 + molecule_offset;
            let quote_idx = request_idx + 1;
            let acceptance_idx = quote_idx + 1;
            let escrow_idx = acceptance_idx + 1;
            let settlement_idx = escrow_idx + 1;
            let result_idx = settlement_idx + 1;
            let acknowledgement_idx = result_idx + 1;
            let release_idx = acknowledgement_idx + 1;
            assert!(flow.artifacts[offer_idx].has_parent(&flow.artifacts[profile_idx].id));
            assert!(flow.artifacts[request_idx].has_parent(&flow.artifacts[offer_idx].id));
            if molecule_offset == 1 {
                let molecule_idx = 2;
                assert!(flow.artifacts[request_idx].has_parent(&flow.artifacts[molecule_idx].id));
                assert!(flow.artifacts[molecule_idx]
                    .schema_tags
                    .contains(&SchemaTag("chem.molecule.adt".to_string())));
            }
            assert!(flow.artifacts[quote_idx].has_parent(&flow.artifacts[request_idx].id));
            assert!(flow.artifacts[acceptance_idx].has_parent(&flow.artifacts[quote_idx].id));
            assert!(flow.artifacts[escrow_idx].has_parent(&flow.artifacts[acceptance_idx].id));
            assert!(flow.artifacts[settlement_idx].has_parent(&flow.artifacts[escrow_idx].id));
            assert!(flow.artifacts[result_idx].has_parent(&flow.artifacts[request_idx].id));
            assert!(flow.artifacts[result_idx].has_parent(&flow.artifacts[settlement_idx].id));
            assert!(flow.artifacts[acknowledgement_idx].has_parent(&flow.artifacts[result_idx].id));
            assert!(flow.artifacts[acknowledgement_idx].has_parent(&flow.artifacts[escrow_idx].id));
            assert!(flow.artifacts[release_idx].has_parent(&flow.artifacts[acknowledgement_idx].id));
            assert!(flow.artifacts[release_idx].has_parent(&flow.artifacts[settlement_idx].id));
        }
    }

    #[test]
    fn dft_flow_carries_signed_molecule_artifact() {
        let signed = sign_flow(dft_flow(), 100).expect("dft flow signs");
        let molecule_artifact = signed
            .artifacts
            .iter()
            .find(|artifact| {
                artifact
                    .schema_tags
                    .contains(&SchemaTag("chem.molecule.adt".to_string()))
            })
            .expect("molecule artifact present in signed flow");
        let request_artifact = signed
            .artifacts
            .iter()
            .find(|artifact| {
                artifact
                    .schema_tags
                    .contains(&SchemaTag(DFT_REQUEST_TAG.to_string()))
            })
            .expect("dft service request artifact present");
        assert!(request_artifact.has_parent(&molecule_artifact.id));
        match &signed.flow.request.input {
            ScienceServiceInput::Dft { molecule, .. } => {
                assert_eq!(
                    molecule.molecule_artifact_id.as_ref(),
                    Some(&molecule_artifact.id),
                );
                assert_eq!(
                    molecule.molecule_payload_hash.as_ref(),
                    molecule_artifact.payload.as_ref().map(|p| &p.hash),
                );
            }
            _ => panic!("DFT flow request must carry a Dft input"),
        }
    }

    #[test]
    fn demo_market_serializes_to_json() {
        let demo = demo_science_market();
        let value = serde_json::to_value(demo).expect("json value");
        assert_eq!(value["demo_id"], "SCIENCE.MARKET.DEMO.001");
        assert!(value["sponsor_next_steps"]["ENS"].is_string());
        assert!(value["flows"][0]["flow"]["economic_settlement"]["settlement_release"].is_object());
    }

    #[test]
    fn settlement_validation_rejects_quote_above_request_max() {
        let mut flow = dft_flow();
        flow.quote.price_usdc_micros = flow.request.max_price_usdc_micros + 1;
        assert!(matches!(
            flow.validate_economic_settlement(),
            Err(MarketError::Settlement(_))
        ));
    }

    #[test]
    fn refund_requires_rejected_or_reviewed_result() {
        let flow = literature_flow();
        assert!(refund_after_rejection(
            "REFUND.LIT.FLOW.001",
            &flow.economic_settlement.escrow_authorization,
            &flow.economic_settlement.result_acknowledgement,
            ScienceRefundReason::ResultRejected,
        )
        .is_err());

        let mut acknowledgement = flow.economic_settlement.result_acknowledgement.clone();
        acknowledgement.status = ScienceResultAcknowledgementStatus::Rejected;
        let refund = refund_after_rejection(
            "REFUND.LIT.FLOW.001",
            &flow.economic_settlement.escrow_authorization,
            &acknowledgement,
            ScienceRefundReason::ResultRejected,
        )
        .expect("refund record");
        assert_eq!(refund.payer_agent, flow.request.requester_agent);
        assert_eq!(refund.amount_usdc_micros, flow.quote.price_usdc_micros);
    }
}
