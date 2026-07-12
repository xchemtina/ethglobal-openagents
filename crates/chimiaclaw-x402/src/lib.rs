//! x402 payment primitives for ChimiaClaw science services.
//!
//! Maps HTTP 402 Payment Required flows onto signed, content-addressed
//! artifacts so agentic micropayments stay auditable in the artifact DAG.
//!
//! Live facilitator verification lives in `services/api-gateway` (TypeScript
//! x402 middleware). This crate owns:
//! - service catalog SKUs and USDC pricing
//! - CAIP-2 network identifiers
//! - challenge / payment / receipt payload shapes
//! - sealing those payloads as ChimiaClaw artifacts

use chimiaclaw_artifact::{Artifact, ArtifactDraft, ArtifactError, ArtifactSigner, PayloadRef};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Schema tags for the x402 settlement trail.
pub const X402_CHALLENGE_TAG: &str = "market.x402.challenge";
pub const X402_PAYMENT_TAG: &str = "market.x402.payment";
pub const X402_RECEIPT_TAG: &str = "market.x402.receipt";
pub const X402_CATALOG_TAG: &str = "market.x402.catalog";

pub const X402_CHALLENGE_SKILL: &str = "market.x402.challenge.v1";
pub const X402_PAYMENT_SKILL: &str = "market.x402.payment.v1";
pub const X402_RECEIPT_SKILL: &str = "market.x402.receipt.v1";
pub const X402_CATALOG_SKILL: &str = "market.x402.catalog.v1";

/// Base Sepolia (testnet) CAIP-2 network id used by the public x402 facilitator.
pub const NETWORK_BASE_SEPOLIA: &str = "eip155:84532";
/// Base mainnet CAIP-2 network id.
pub const NETWORK_BASE_MAINNET: &str = "eip155:8453";
/// Default test facilitator URL (testnet only — never use for mainnet).
pub const DEFAULT_TEST_FACILITATOR_URL: &str = "https://x402.org/facilitator";

/// DAO-facing service catalog agent (ENS-shaped).
pub const CATALOG_AGENT: &str = "market.chimiaclaw.eth";
/// Default MolADT geometry provider agent.
pub const MOLADT_AGENT: &str = "moladt.service.chimiaclaw.eth";

/// One sellable science capability advertised over x402.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct X402ServiceSku {
    pub sku_id: String,
    pub service_kind: String,
    pub title: String,
    pub description: String,
    /// Price in USDC micro-units (1 USDC = 1_000_000).
    pub price_usdc_micros: u64,
    /// Dollar string for x402 middleware, e.g. `"$0.01"`.
    pub price_display: String,
    pub http_method: String,
    pub path: String,
    pub produces_schema_tags: Vec<String>,
    pub estimated_latency_seconds: u64,
    pub network: String,
    pub mime_type: String,
}

/// Full catalog projected for agents and the public website.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct X402ServiceCatalog {
    pub catalog_id: String,
    pub version: String,
    pub provider_ens: String,
    pub pay_to_hint: String,
    pub network: String,
    pub facilitator_url: String,
    pub maturity: String,
    pub skus: Vec<X402ServiceSku>,
    pub notes: Vec<String>,
}

/// HTTP 402 challenge payload recorded before payment (optional audit parent).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct X402PaymentChallenge {
    pub challenge_id: String,
    pub sku_id: String,
    pub resource_path: String,
    pub scheme: String,
    pub network: String,
    pub pay_to: String,
    pub price_usdc_micros: u64,
    pub price_display: String,
    pub asset: String,
    pub expires_at_unix: u64,
    pub description: String,
    pub mode: X402Mode,
}

/// Envelope for a verified client payment proof (facilitator-backed or stub).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct X402PaymentProof {
    pub payment_id: String,
    pub challenge_id: String,
    pub sku_id: String,
    pub payer_hint: String,
    pub pay_to: String,
    pub network: String,
    pub amount_usdc_micros: u64,
    pub asset: String,
    /// Opaque facilitator / client payment signature header value (truncated for storage if huge).
    pub payment_signature_ref: String,
    pub verified_at_unix: u64,
    pub verification: X402VerificationStatus,
    pub transaction_ref: Option<String>,
    pub audit_notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum X402VerificationStatus {
    /// Local development: payment skipped, no funds moved.
    StubAccepted,
    /// Facilitator verified the PAYMENT-SIGNATURE header.
    FacilitatorVerified,
    /// Operator manually accepted after reviewing chain evidence.
    OperatorAccepted,
}

/// Receipt binding payment proof to the delivered science artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct X402SettlementReceipt {
    pub receipt_id: String,
    pub payment_id: String,
    pub challenge_id: String,
    pub sku_id: String,
    pub result_artifact_id: String,
    pub amount_usdc_micros: u64,
    pub asset: String,
    pub network: String,
    pub pay_to: String,
    pub settled_at_unix: u64,
    pub status: X402SettlementStatus,
    pub audit_notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum X402SettlementStatus {
    Delivered,
    DeliveredStubMode,
    FailedNoCharge,
}

/// Runtime mode for the HTTP gateway.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum X402Mode {
    /// No payment required; useful for local UI wiring.
    Free,
    /// Emit realistic 402-shaped challenges and accept a stub signature without chain settlement.
    Stub,
    /// Real facilitator verification + USDC settlement.
    Live,
}

#[derive(Debug)]
pub enum X402Error {
    Artifact(ArtifactError),
    Invalid(String),
}

impl std::fmt::Display for X402Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
            Self::Invalid(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for X402Error {}

impl From<ArtifactError> for X402Error {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

/// Convert USDC micros to the x402 dollar price string (`"$0.01"`).
#[must_use]
pub fn micros_to_price_display(micros: u64) -> String {
    let whole = micros / 1_000_000;
    let frac = micros % 1_000_000;
    if frac == 0 {
        format!("${whole}")
    } else if frac % 10_000 == 0 {
        // Exact cent amounts: always two decimal places ("$0.10", not "$0.1").
        format!("${whole}.{:02}", frac / 10_000)
    } else {
        // Sub-cent precision: keep micros, strip only pure trailing zeros after two digits.
        let frac_str = format!("{frac:06}");
        let mut trimmed = frac_str.as_str();
        while trimmed.len() > 2 && trimmed.ends_with('0') {
            trimmed = &trimmed[..trimmed.len() - 1];
        }
        format!("${whole}.{trimmed}")
    }
}

/// Parse a `$0.01`-style string into USDC micros. Returns `None` on invalid input.
#[must_use]
pub fn price_display_to_micros(price: &str) -> Option<u64> {
    let stripped = price.trim().trim_start_matches('$').trim();
    if stripped.is_empty() {
        return None;
    }
    let parts: Vec<&str> = stripped.split('.').collect();
    match parts.as_slice() {
        [whole] => whole.parse::<u64>().ok().map(|w| w.saturating_mul(1_000_000)),
        [whole, frac] => {
            let w = whole.parse::<u64>().ok()?;
            if frac.len() > 6 || !frac.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let mut frac_padded = frac.to_string();
            while frac_padded.len() < 6 {
                frac_padded.push('0');
            }
            let f = frac_padded.parse::<u64>().ok()?;
            Some(w.saturating_mul(1_000_000).saturating_add(f))
        }
        _ => None,
    }
}

/// Default ChimiaDAO science SKU catalog for the first revenue surface.
#[must_use]
pub fn default_catalog(pay_to_hint: &str, network: &str) -> X402ServiceCatalog {
    let skus = vec![
        X402ServiceSku {
            sku_id: "moladt.geometry".to_string(),
            service_kind: "moladt".to_string(),
            title: "MolADT geometry".to_string(),
            description: "SMILES → signed chem.molecule.adt with schematic or RDKit geometry"
                .to_string(),
            price_usdc_micros: 10_000, // $0.01
            price_display: "$0.01".to_string(),
            http_method: "POST".to_string(),
            path: "/v1/moladt".to_string(),
            produces_schema_tags: vec!["chem.molecule.adt".to_string()],
            estimated_latency_seconds: 5,
            network: network.to_string(),
            mime_type: "application/json".to_string(),
        },
        X402ServiceSku {
            sku_id: "literature.synthesis".to_string(),
            service_kind: "literature".to_string(),
            title: "Literature synthesis".to_string(),
            description: "Query → signed science.literature.synthesis artifact".to_string(),
            price_usdc_micros: 100_000, // $0.10
            price_display: "$0.10".to_string(),
            http_method: "POST".to_string(),
            path: "/v1/literature".to_string(),
            produces_schema_tags: vec!["science.literature.synthesis".to_string()],
            estimated_latency_seconds: 60,
            network: network.to_string(),
            mime_type: "application/json".to_string(),
        },
        X402ServiceSku {
            sku_id: "dft.cached_result".to_string(),
            service_kind: "dft".to_string(),
            title: "Cached DFT result".to_string(),
            description: "Retrieve a previously computed signed chem.dft.result by molecule label"
                .to_string(),
            price_usdc_micros: 50_000, // $0.05
            price_display: "$0.05".to_string(),
            http_method: "GET".to_string(),
            path: "/v1/dft/cached".to_string(),
            produces_schema_tags: vec!["chem.dft.result".to_string()],
            estimated_latency_seconds: 2,
            network: network.to_string(),
            mime_type: "application/json".to_string(),
        },
        X402ServiceSku {
            sku_id: "dft.live_small".to_string(),
            service_kind: "dft".to_string(),
            title: "Live small-molecule DFT".to_string(),
            description: "Operator-capped live PySCF DFT for small systems (gated, not free-run)"
                .to_string(),
            price_usdc_micros: 2_500_000, // $2.50
            price_display: "$2.50".to_string(),
            http_method: "POST".to_string(),
            path: "/v1/dft/live".to_string(),
            produces_schema_tags: vec!["chem.dft.result".to_string()],
            estimated_latency_seconds: 600,
            network: network.to_string(),
            mime_type: "application/json".to_string(),
        },
    ];

    X402ServiceCatalog {
        catalog_id: "chimia.x402.catalog.v1".to_string(),
        version: "0.1.0".to_string(),
        provider_ens: CATALOG_AGENT.to_string(),
        pay_to_hint: pay_to_hint.to_string(),
        network: network.to_string(),
        facilitator_url: DEFAULT_TEST_FACILITATOR_URL.to_string(),
        maturity: "scaffold-ready".to_string(),
        skus,
        notes: vec![
            "Prices are initial DAO micro-SKU defaults; governance may revise.".to_string(),
            "Live mode requires CHIMIA_X402_PAY_TO and a funded facilitator path.".to_string(),
            "Stub mode never moves funds; Free mode skips the 402 challenge.".to_string(),
            "Every paid response must be a signed ChimiaClaw artifact or bundle.".to_string(),
        ],
    }
}

/// Build a payment challenge for a catalog SKU.
pub fn challenge_for_sku(
    sku: &X402ServiceSku,
    pay_to: &str,
    mode: X402Mode,
    now_unix: u64,
) -> Result<X402PaymentChallenge, X402Error> {
    if pay_to.trim().is_empty() && mode == X402Mode::Live {
        return Err(X402Error::Invalid(
            "live x402 mode requires a non-empty pay_to treasury address".to_string(),
        ));
    }
    Ok(X402PaymentChallenge {
        challenge_id: format!("x402ch_{}_{}", sku.sku_id.replace('.', "_"), now_unix),
        sku_id: sku.sku_id.clone(),
        resource_path: sku.path.clone(),
        scheme: "exact".to_string(),
        network: sku.network.clone(),
        pay_to: pay_to.to_string(),
        price_usdc_micros: sku.price_usdc_micros,
        price_display: sku.price_display.clone(),
        asset: "USDC".to_string(),
        expires_at_unix: now_unix.saturating_add(600),
        description: sku.description.clone(),
        mode,
    })
}

/// Seal a catalog as a signed artifact for ENS/0G/publication.
pub fn catalog_artifact(
    catalog: &X402ServiceCatalog,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, X402Error> {
    Ok(ArtifactDraft {
        skill: SkillId(X402_CATALOG_SKILL.to_string()),
        agent,
        topic: format!("x402 catalog {}", catalog.catalog_id),
        input_fingerprint: format!("x402:catalog:{}:{}", catalog.catalog_id, catalog.version),
        output_cid: None,
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(X402_CATALOG_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(catalog)?),
    }
    .seal(signer, created_at_unix)?)
}

/// Seal an HTTP 402 challenge as a signed artifact.
pub fn challenge_artifact(
    challenge: &X402PaymentChallenge,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, X402Error> {
    Ok(ArtifactDraft {
        skill: SkillId(X402_CHALLENGE_SKILL.to_string()),
        agent,
        topic: format!("x402 challenge {}", challenge.challenge_id),
        input_fingerprint: format!(
            "x402:challenge:{}:{}",
            challenge.sku_id, challenge.challenge_id
        ),
        output_cid: None,
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(X402_CHALLENGE_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(challenge)?),
    }
    .seal(signer, created_at_unix)?)
}

/// Seal a verified payment proof as a signed artifact.
pub fn payment_artifact(
    payment: &X402PaymentProof,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
    parent_ids: Vec<chimiaclaw_artifact::ArtifactId>,
) -> Result<Artifact, X402Error> {
    Ok(ArtifactDraft {
        skill: SkillId(X402_PAYMENT_SKILL.to_string()),
        agent,
        topic: format!("x402 payment {}", payment.payment_id),
        input_fingerprint: format!(
            "x402:payment:{}:{}:{}",
            payment.sku_id, payment.payment_id, payment.amount_usdc_micros
        ),
        output_cid: None,
        parent_artifact_ids: parent_ids,
        schema_tags: BTreeSet::from([SchemaTag(X402_PAYMENT_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(payment)?),
    }
    .seal(signer, created_at_unix)?)
}

/// Seal a settlement receipt binding payment → science result.
pub fn receipt_artifact(
    receipt: &X402SettlementReceipt,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
    parent_ids: Vec<chimiaclaw_artifact::ArtifactId>,
) -> Result<Artifact, X402Error> {
    Ok(ArtifactDraft {
        skill: SkillId(X402_RECEIPT_SKILL.to_string()),
        agent,
        topic: format!("x402 receipt {}", receipt.receipt_id),
        input_fingerprint: format!(
            "x402:receipt:{}:{}:{}",
            receipt.sku_id, receipt.receipt_id, receipt.result_artifact_id
        ),
        output_cid: None,
        parent_artifact_ids: parent_ids,
        schema_tags: BTreeSet::from([SchemaTag(X402_RECEIPT_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(receipt)?),
    }
    .seal(signer, created_at_unix)?)
}

/// Deterministic demo bundle for CLI smoke tests (no fund movement).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct X402DemoBundle {
    pub maturity: String,
    pub catalog: X402ServiceCatalog,
    pub sample_challenge: X402PaymentChallenge,
    pub sample_payment: X402PaymentProof,
    pub sample_receipt: X402SettlementReceipt,
    pub catalog_artifact: Artifact,
    pub challenge_artifact: Artifact,
    pub payment_artifact: Artifact,
    pub receipt_artifact: Artifact,
    pub truth: Vec<String>,
}

/// Build a fully signed stub-mode demo for `chimiaclaw-cli x402-demo`.
pub fn demo_x402_bundle(signer: &ArtifactSigner, now_unix: u64) -> Result<X402DemoBundle, X402Error> {
    let pay_to = "0xChimiaDAOTreasuryPlaceholder0000000000";
    let catalog = default_catalog(pay_to, NETWORK_BASE_SEPOLIA);
    let sku = catalog
        .skus
        .iter()
        .find(|s| s.sku_id == "moladt.geometry")
        .cloned()
        .ok_or_else(|| X402Error::Invalid("missing moladt.geometry sku".to_string()))?;

    let challenge = challenge_for_sku(&sku, pay_to, X402Mode::Stub, now_unix)?;
    let payment = X402PaymentProof {
        payment_id: format!("x402pay_stub_{now_unix}"),
        challenge_id: challenge.challenge_id.clone(),
        sku_id: sku.sku_id.clone(),
        payer_hint: "agent.buyer.example".to_string(),
        pay_to: pay_to.to_string(),
        network: NETWORK_BASE_SEPOLIA.to_string(),
        amount_usdc_micros: sku.price_usdc_micros,
        asset: "USDC".to_string(),
        payment_signature_ref: "stub:PAYMENT-SIGNATURE".to_string(),
        verified_at_unix: now_unix.saturating_add(1),
        verification: X402VerificationStatus::StubAccepted,
        transaction_ref: None,
        audit_notes: vec![
            "stub mode: no on-chain transfer".to_string(),
            "facilitator not contacted".to_string(),
        ],
    };
    let receipt = X402SettlementReceipt {
        receipt_id: format!("x402rcpt_stub_{now_unix}"),
        payment_id: payment.payment_id.clone(),
        challenge_id: challenge.challenge_id.clone(),
        sku_id: sku.sku_id.clone(),
        result_artifact_id: "art_demo_moladt_result".to_string(),
        amount_usdc_micros: sku.price_usdc_micros,
        asset: "USDC".to_string(),
        network: NETWORK_BASE_SEPOLIA.to_string(),
        pay_to: pay_to.to_string(),
        settled_at_unix: now_unix.saturating_add(2),
        status: X402SettlementStatus::DeliveredStubMode,
        audit_notes: vec![
            "result delivery is simulated in the demo bundle".to_string(),
            "live gateway seals a real chem.molecule.adt as the result parent".to_string(),
        ],
    };

    let agent = AgentId(CATALOG_AGENT.to_string());
    let catalog_art = catalog_artifact(&catalog, agent.clone(), signer, now_unix)?;
    let challenge_art = challenge_artifact(&challenge, agent.clone(), signer, now_unix)?;
    let payment_art = payment_artifact(
        &payment,
        agent.clone(),
        signer,
        now_unix.saturating_add(1),
        vec![challenge_art.id.clone()],
    )?;
    let receipt_art = receipt_artifact(
        &receipt,
        agent,
        signer,
        now_unix.saturating_add(2),
        vec![payment_art.id.clone()],
    )?;

    Ok(X402DemoBundle {
        maturity: "deterministic-stub-demo".to_string(),
        catalog,
        sample_challenge: challenge,
        sample_payment: payment,
        sample_receipt: receipt,
        catalog_artifact: catalog_art,
        challenge_artifact: challenge_art,
        payment_artifact: payment_art,
        receipt_artifact: receipt_art,
        truth: vec![
            "x402 catalog, challenge, payment, and receipt are signed artifacts".to_string(),
            "stub demo does not contact a facilitator or move USDC".to_string(),
            "services/api-gateway owns live HTTP 402 middleware".to_string(),
            "pay_to must be a DAO-controlled Base address before Live mode".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micros_price_roundtrip() {
        assert_eq!(micros_to_price_display(10_000), "$0.01");
        assert_eq!(micros_to_price_display(100_000), "$0.10");
        assert_eq!(micros_to_price_display(2_500_000), "$2.50");
        assert_eq!(micros_to_price_display(1_000_000), "$1");
        assert_eq!(price_display_to_micros("$0.01"), Some(10_000));
        assert_eq!(price_display_to_micros("$2.50"), Some(2_500_000));
        assert_eq!(price_display_to_micros("$1"), Some(1_000_000));
    }

    #[test]
    fn default_catalog_has_moladt() {
        let catalog = default_catalog("0xabc", NETWORK_BASE_SEPOLIA);
        assert_eq!(catalog.skus.len(), 4);
        assert!(catalog.skus.iter().any(|s| s.sku_id == "moladt.geometry"));
        assert_eq!(catalog.network, NETWORK_BASE_SEPOLIA);
    }

    #[test]
    fn demo_bundle_signs_and_lineage() {
        let signer = ArtifactSigner::from_seed([42; 32]);
        let bundle = demo_x402_bundle(&signer, 1_700_000_000).expect("demo bundle");
        assert_eq!(
            bundle.payment_artifact.parent_artifact_ids,
            vec![bundle.challenge_artifact.id.clone()]
        );
        assert_eq!(
            bundle.receipt_artifact.parent_artifact_ids,
            vec![bundle.payment_artifact.id.clone()]
        );
        assert!(bundle
            .catalog_artifact
            .schema_tags
            .iter()
            .any(|t| t.0 == X402_CATALOG_TAG));
    }

    #[test]
    fn live_mode_requires_pay_to() {
        let catalog = default_catalog("", NETWORK_BASE_SEPOLIA);
        let sku = &catalog.skus[0];
        let err = challenge_for_sku(sku, "", X402Mode::Live, 1).unwrap_err();
        assert!(matches!(err, X402Error::Invalid(_)));
    }
}
