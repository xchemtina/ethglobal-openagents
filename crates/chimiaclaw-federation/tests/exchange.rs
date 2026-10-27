use chimiaclaw_artifact::{ArtifactDraft, ArtifactSigner, PayloadRef};
use chimiaclaw_federation::*;
use chimiaclaw_posyn::{import_bundle, VerifiedBundle};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use std::collections::BTreeSet;
use std::path::PathBuf;

const NOW: u64 = 1_800_000_000;
const ROOT: &str = "blake3:a48f37a331a0949452b6b3535ff7da81cc28ab89e9b2860c13256369404411cd";
struct Fixture {
    humans: Vec<LocalIdentity>,
    ais: Vec<LocalIdentity>,
    policy: TrustPolicy,
    bundle: VerifiedBundle,
}
impl Fixture {
    fn new() -> Self {
        let humans: Vec<_> = ["coordinator", "producer", "reviewer"]
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                LocalIdentity::from_seed(
                    id.into(),
                    IdentityKind::Human,
                    [u8::try_from(i).unwrap() + 1; 32],
                )
                .unwrap()
            })
            .collect();
        let ais: Vec<_> = ["coordinator-ai", "producer-ai", "reviewer-ai"]
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                LocalIdentity::from_seed(
                    id.into(),
                    IdentityKind::Ai,
                    [u8::try_from(i).unwrap() + 10; 32],
                )
                .unwrap()
            })
            .collect();
        let policy = TrustPolicy {
            schema: POLICY_SCHEMA.into(),
            policy_id: "synthetic-three-person-pilot".into(),
            observed_at_unix: NOW - 10,
            expires_at_unix: NOW + 3600,
            current_scope: Scope {
                scope_id: "pox:local:inquiry:handoff1".into(),
                revision: 1,
                scope_sha256: format!("sha256:{}", "a".repeat(64)),
                coordinator_id: "coordinator".into(),
                producer_id: "producer".into(),
                reviewer_id: "reviewer".into(),
            },
            expected_posyn_root: ROOT.into(),
            trusted_identities: humans
                .iter()
                .chain(&ais)
                .map(|v| v.identity().clone())
                .collect(),
            revoked_keys: vec![],
            revoked_delegations: vec![],
        };
        let bundle = import_bundle(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../chimiaclaw-posyn/fixtures/synthetic/bundle"),
            ROOT,
        )
        .unwrap();
        Self {
            humans,
            ais,
            policy,
            bundle,
        }
    }
    fn bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(&self.policy).unwrap()
    }
    fn evidence(&self) -> Envelope {
        create_evidence(
            &self.humans[1],
            &self.policy,
            self.bundle.clone(),
            None,
            Audience::Synthetic,
            NOW,
        )
        .unwrap()
    }
    fn delegation(&self) -> Envelope {
        create_delegation(
            &self.humans[1],
            &self.policy,
            "producer-ai".into(),
            NOW + 100,
            NOW,
        )
        .unwrap()
    }
    fn ai_evidence(&self) -> Envelope {
        create_evidence(
            &self.ais[1],
            &self.policy,
            self.bundle.clone(),
            Some(self.delegation()),
            Audience::Synthetic,
            NOW,
        )
        .unwrap()
    }
}

fn raw_statement(package: &Envelope) -> Statement {
    serde_json::from_slice(&package.artifact.inline_payload_bytes().unwrap().unwrap()).unwrap()
}
fn sign_attack(statement: &Statement, seed: u8, id: &str) -> Envelope {
    let parents = if let Statement::Review {
        ref evidence_artifact_id,
        ..
    } = statement
    {
        vec![chimiaclaw_artifact::ArtifactId(
            evidence_artifact_id.clone(),
        )]
    } else {
        vec![]
    };
    let artifact = ArtifactDraft {
        skill: SkillId("federation.exchange.v1".into()),
        agent: AgentId(id.into()),
        topic: format!("federation.{}", statement.kind()),
        input_fingerprint: statement.scope().scope_sha256.clone(),
        output_cid: None,
        parent_artifact_ids: parents,
        schema_tags: BTreeSet::from([SchemaTag(format!("federation.{}.v1", statement.kind()))]),
        payload: Some(PayloadRef::inline_json(&statement).unwrap()),
    }
    .seal(&ArtifactSigner::from_seed([seed; 32]), NOW)
    .unwrap();
    Envelope {
        schema: PACKAGE_SCHEMA.into(),
        artifact,
    }
}

#[test]
fn three_people_and_their_ai_systems_exchange_and_review() {
    let f = Fixture::new();
    assert_eq!(f.policy.trusted_identities.len(), 6);
    let evidence = f.ai_evidence();
    let received = decode_envelope(&serde_json::to_vec(&evidence).unwrap()).unwrap();
    let receipt = verify_package(&received, &f.bytes(), NOW, None).unwrap();
    assert_eq!(receipt.signer_id, "producer-ai");
    assert_eq!(receipt.policy_sha256, sha256(&f.bytes()));
    assert_eq!(receipt.trust_state, "local_snapshot");
    assert!(!receipt.publication_authorized);
    assert!(!receipt.encrypted);
    let review = create_review(
        &f.humans[2],
        &f.bytes(),
        &received,
        ReviewDecision::Reviewed,
        NOW,
    )
    .unwrap();
    let receipt = verify_package(&review, &f.bytes(), NOW, Some(&received)).unwrap();
    assert_eq!(receipt.review_decision, Some(ReviewDecision::Reviewed));
    assert_eq!(receipt.scientific_review, "not_established");
    let Statement::Evidence { bundle, .. } = raw_statement(&received) else {
        panic!()
    };
    assert_eq!(bundle, f.bundle);
}

#[test]
fn correction_required_is_preserved_in_receipt() {
    let f = Fixture::new();
    let evidence = f.evidence();
    let review = create_review(
        &f.humans[2],
        &f.bytes(),
        &evidence,
        ReviewDecision::CorrectionsRequired,
        NOW,
    )
    .unwrap();
    assert_eq!(
        verify_package(&review, &f.bytes(), NOW, Some(&evidence))
            .unwrap()
            .review_decision,
        Some(ReviewDecision::CorrectionsRequired)
    );
}

#[test]
fn embedded_key_substitution_and_signature_tampering_are_rejected() {
    let f = Fixture::new();
    let mut package = f.evidence();
    package.artifact.signing_public_key = f.humans[0].identity().public_key.clone();
    assert_eq!(
        verify_package(&package, &f.bytes(), NOW, None)
            .unwrap_err()
            .0,
        "signer_key_mismatch"
    );
    let mut package = f.evidence();
    package.artifact.signature = "00".repeat(64);
    assert!(verify_package(&package, &f.bytes(), NOW, None).is_err());
}

#[test]
fn object_tampering_fails_even_when_producer_resigns() {
    let f = Fixture::new();
    let mut statement = raw_statement(&f.evidence());
    if let Statement::Evidence { ref mut bundle, .. } = statement {
        bundle.objects[0].bytes_hex.push_str("00");
    }
    assert_eq!(
        verify_package(
            &sign_attack(&statement, 2, "producer"),
            &f.bytes(),
            NOW,
            None
        )
        .unwrap_err()
        .0,
        "invalid_posyn_bundle"
    );
}

#[test]
fn root_is_pinned_outside_the_received_package() {
    let mut f = Fixture::new();
    let package = f.evidence();
    f.policy.expected_posyn_root = format!("blake3:{}", "b".repeat(64));
    assert_eq!(
        verify_package(&package, &f.bytes(), NOW, None)
            .unwrap_err()
            .0,
        "expected_root_mismatch"
    );
}

#[test]
fn each_scope_identity_dimension_is_binding() {
    let f = Fixture::new();
    let evidence = f.evidence();
    for changed in ["revision", "handoff", "digest", "reviewer"] {
        let mut policy = f.policy.clone();
        match changed {
            "revision" => policy.current_scope.revision += 1,
            "handoff" => policy.current_scope.scope_id.push('2'),
            "digest" => policy.current_scope.scope_sha256 = format!("sha256:{}", "b".repeat(64)),
            _ => {
                std::mem::swap(
                    &mut policy.current_scope.reviewer_id,
                    &mut policy.current_scope.coordinator_id,
                );
            }
        }
        assert_eq!(
            verify_package(&evidence, &serde_json::to_vec(&policy).unwrap(), NOW, None)
                .unwrap_err()
                .0,
            "stale_or_wrong_scope"
        );
    }
}

#[test]
fn revoked_human_and_ai_keys_invalidate_existing_packages() {
    for revoke_ai in [false, true] {
        let mut f = Fixture::new();
        let evidence = f.ai_evidence();
        f.policy.revoked_keys.push(if revoke_ai {
            f.ais[1].identity().public_key.clone()
        } else {
            f.humans[1].identity().public_key.clone()
        });
        assert_eq!(
            verify_package(&evidence, &f.bytes(), NOW, None)
                .unwrap_err()
                .0,
            "revoked_key"
        );
    }
}

#[test]
fn revoked_or_expired_delegation_cannot_be_replayed() {
    let mut f = Fixture::new();
    let evidence = f.ai_evidence();
    f.policy
        .revoked_delegations
        .push(format!("blake3:{}", f.delegation().artifact.content_hash));
    assert_eq!(
        verify_package(&evidence, &f.bytes(), NOW, None)
            .unwrap_err()
            .0,
        "revoked_delegation"
    );
    f.policy.revoked_delegations.clear();
    assert_eq!(
        verify_package(&evidence, &f.bytes(), NOW + 101, None)
            .unwrap_err()
            .0,
        "invalid_or_expired_delegation"
    );
}

#[test]
fn ai_cannot_produce_without_its_own_delegation_or_review_as_a_human() {
    let f = Fixture::new();
    assert!(create_evidence(
        &f.ais[1],
        &f.policy,
        f.bundle.clone(),
        None,
        Audience::Private,
        NOW
    )
    .is_err());
    assert!(create_evidence(
        &f.ais[0],
        &f.policy,
        f.bundle.clone(),
        Some(f.delegation()),
        Audience::Private,
        NOW
    )
    .is_err());
    assert!(create_review(
        &f.ais[2],
        &f.bytes(),
        &f.evidence(),
        ReviewDecision::Reviewed,
        NOW
    )
    .is_err());
    assert!(create_delegation(
        &f.humans[0],
        &f.policy,
        "producer-ai".into(),
        NOW + 100,
        NOW
    )
    .is_err());
}

#[test]
fn human_cannot_impersonate_another_role() {
    let f = Fixture::new();
    for index in [0, 2] {
        assert!(create_evidence(
            &f.humans[index],
            &f.policy,
            f.bundle.clone(),
            None,
            Audience::Private,
            NOW
        )
        .is_err());
    }
    for index in [0, 1] {
        assert!(create_review(
            &f.humans[index],
            &f.bytes(),
            &f.evidence(),
            ReviewDecision::Reviewed,
            NOW
        )
        .is_err());
    }
}

#[test]
fn review_requires_exact_evidence_and_never_admits_scientific_truth() {
    let f = Fixture::new();
    let evidence = f.evidence();
    let review = create_review(
        &f.humans[2],
        &f.bytes(),
        &evidence,
        ReviewDecision::Reviewed,
        NOW,
    )
    .unwrap();
    assert_eq!(
        verify_package(&review, &f.bytes(), NOW, None)
            .unwrap_err()
            .0,
        "review_requires_evidence"
    );
    assert_eq!(
        verify_package(&review, &f.bytes(), NOW, Some(&f.ai_evidence()))
            .unwrap_err()
            .0,
        "review_target_mismatch"
    );
    let mut attack = raw_statement(&review);
    if let Statement::Review {
        ref mut scientific_review,
        ..
    } = attack
    {
        *scientific_review = "verified".into();
    }
    assert!(verify_package(
        &sign_attack(&attack, 3, "reviewer"),
        &f.bytes(),
        NOW,
        Some(&evidence)
    )
    .is_err());
}

#[test]
fn unknown_and_duplicate_fields_and_public_audience_are_rejected() {
    let f = Fixture::new();
    let policy = String::from_utf8(f.bytes()).unwrap();
    assert!(decode_policy(
        policy
            .replacen('{', "{\"schema\":\"duplicate\",", 1)
            .as_bytes()
    )
    .is_err());
    let mut package = serde_json::to_value(f.evidence()).unwrap();
    package["artifact"]["publication_authorized"] = true.into();
    assert!(decode_envelope(&serde_json::to_vec(&package).unwrap()).is_err());
    let mut statement = serde_json::to_value(raw_statement(&f.evidence())).unwrap();
    statement["audience"] = "public".into();
    assert!(serde_json::from_value::<Statement>(statement).is_err());
}

#[test]
fn expired_or_future_policies_and_ambiguous_identities_are_rejected() {
    let f = Fixture::new();
    let evidence = f.evidence();
    assert!(verify_package(&evidence, &f.bytes(), NOW + 3600, None).is_err());
    assert!(verify_package(&evidence, &f.bytes(), NOW - 11, None).is_err());
    let mut policy = f.policy.clone();
    policy
        .trusted_identities
        .push(policy.trusted_identities[0].clone());
    assert!(policy.validate(NOW).is_err());
    let mut policy = f.policy.clone();
    policy.current_scope.reviewer_id = policy.current_scope.producer_id.clone();
    assert!(policy.validate(NOW).is_err());
}

#[test]
fn signed_future_packages_and_metadata_smuggling_are_rejected() {
    let f = Fixture::new();
    let future = create_evidence(
        &f.humans[1],
        &f.policy,
        f.bundle.clone(),
        None,
        Audience::Private,
        NOW + 1,
    )
    .unwrap();
    assert!(verify_package(&future, &f.bytes(), NOW, None).is_err());
    let mut metadata = f.evidence();
    metadata.artifact.output_cid = Some("file:///private/data".into());
    assert!(verify_package(&metadata, &f.bytes(), NOW, None).is_err());
}

#[test]
fn oversized_json_structure_is_rejected_before_allocation() {
    let large = format!("[{}]", vec!["null"; 25_000].join(","));
    assert_eq!(
        decode_envelope(large.as_bytes()).unwrap_err().0,
        "json_structure_limit"
    );
}
