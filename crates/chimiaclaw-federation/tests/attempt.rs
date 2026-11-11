use chimiaclaw_artifact::{ArtifactDraft, ArtifactSigner, PayloadRef};
use chimiaclaw_federation::{attempt::*, *};
use chimiaclaw_posyn::{import_bundle, parse_manifest, Role};
use std::path::PathBuf;

const NOW: u64 = 1_800_000_000;
const ROOT: &str = "blake3:a48f37a331a0949452b6b3535ff7da81cc28ab89e9b2860c13256369404411cd";

struct Fixture {
    identities: Vec<LocalIdentity>,
    policy: TrustPolicy,
    policy_bytes: Vec<u8>,
    evidence: Envelope,
    expected: Expectation,
    receipt: TerminalReceipt,
    expectation_bytes: Vec<u8>,
    receipt_bytes: Vec<u8>,
    event_bytes: Vec<u8>,
}

fn identities() -> Vec<LocalIdentity> {
    [
        ("coordinator", IdentityKind::Human, 1),
        ("producer", IdentityKind::Human, 2),
        ("reviewer", IdentityKind::Human, 3),
        ("producer-ai", IdentityKind::Ai, 11),
    ]
    .into_iter()
    .map(|(id, kind, seed)| LocalIdentity::from_seed(id.into(), kind, [seed; 32]).unwrap())
    .collect()
}

impl Fixture {
    fn new(ai: bool) -> Self {
        let identities = identities();
        let policy = TrustPolicy {
            schema: chimiaclaw_federation::POLICY_SCHEMA.into(),
            policy_id: "synthetic-attempt-policy".into(),
            observed_at_unix: NOW - 20,
            expires_at_unix: NOW + 1000,
            current_scope: Scope {
                scope_id: "synthetic:inquiry:handoff-1".into(),
                revision: 1,
                scope_sha256: sha256(b"synthetic accepted scope"),
                coordinator_id: "coordinator".into(),
                producer_id: "producer".into(),
                reviewer_id: "reviewer".into(),
            },
            expected_posyn_root: ROOT.into(),
            trusted_identities: identities.iter().map(|v| v.identity().clone()).collect(),
            revoked_keys: vec![],
            revoked_delegations: vec![],
        };
        let bundle = import_bundle(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../chimiaclaw-posyn/fixtures/synthetic/bundle"),
            ROOT,
        )
        .unwrap();
        let result = parse_manifest(&bundle)
            .unwrap()
            .payload
            .artifacts
            .into_iter()
            .find(|v| v.role == Role::Result)
            .unwrap();
        let delegation = ai.then(|| {
            create_delegation(
                &identities[1],
                &policy,
                "producer-ai".into(),
                NOW + 500,
                NOW - 10,
            )
            .unwrap()
        });
        let evidence = create_evidence(
            &identities[if ai { 3 } else { 1 }],
            &policy,
            bundle,
            delegation,
            Audience::Synthetic,
            NOW,
        )
        .unwrap();
        let receipt = TerminalReceipt {
            schema: RECEIPT_SCHEMA.into(),
            scope: policy.current_scope.clone(),
            job_id: "synthetic-job".into(),
            attempt_id: "attempt-1".into(),
            authorization_sha256: sha256(b"synthetic work-order reference; not an execution grant"),
            inputs_sha256: sha256(b"synthetic input manifest"),
            environment_sha256: sha256(b"synthetic runtime identity"),
            started_at_unix: NOW - 5,
            finished_at_unix: NOW - 1,
            outcome: Outcome::Succeeded,
            failure_code: None,
            outputs: vec![OutputIdentity {
                name: result.name.clone(),
                blake3: result.digest,
                size_bytes: result.size_bytes,
            }],
            missing_outputs: vec![],
            retained_failure_record: None,
        };
        let event_bytes = observation(&receipt);
        let receipt_bytes = serde_json::to_vec_pretty(&receipt).unwrap();
        let expected = Expectation {
            schema: EXPECTATION_SCHEMA.into(),
            job_id: receipt.job_id.clone(),
            attempt_id: receipt.attempt_id.clone(),
            authorization_sha256: receipt.authorization_sha256.clone(),
            inputs_sha256: receipt.inputs_sha256.clone(),
            environment_sha256: receipt.environment_sha256.clone(),
            event_sha256: sha256(&event_bytes),
            receipt_sha256: sha256(&receipt_bytes),
            expected_outputs: vec![result.name],
            not_before_unix: NOW - 5,
            expires_at_unix: NOW + 500,
        };
        let expectation_bytes = serde_json::to_vec(&expected).unwrap();
        let policy_bytes = serde_json::to_vec(&policy).unwrap();
        Self {
            identities,
            policy,
            policy_bytes,
            evidence,
            expected,
            receipt,
            expectation_bytes,
            receipt_bytes,
            event_bytes,
        }
    }
    fn context(&self) -> Context<'_> {
        Context {
            policy_bytes: &self.policy_bytes,
            expectation_bytes: &self.expectation_bytes,
            receipt_bytes: &self.receipt_bytes,
            event_bytes: &self.event_bytes,
            evidence: &self.evidence,
            now: NOW,
        }
    }
    fn repin(&mut self) {
        self.receipt_bytes = serde_json::to_vec_pretty(&self.receipt).unwrap();
        self.expected.receipt_sha256 = sha256(&self.receipt_bytes);
        self.event_bytes = observation(&self.receipt);
        self.expected.event_sha256 = sha256(&self.event_bytes);
        self.expectation_bytes = serde_json::to_vec(&self.expected).unwrap();
        self.policy_bytes = serde_json::to_vec(&self.policy).unwrap();
    }
    fn package(&self) -> Package {
        create(
            &self.identities[if self.evidence.artifact.agent.0 == "producer-ai" {
                3
            } else {
                1
            }],
            &self.context(),
        )
        .unwrap()
    }
}

fn observation(receipt: &TerminalReceipt) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schema":"openagents.workstream.observation.v1", "workstream_id":receipt.job_id,
        "source_session":receipt.attempt_id, "receipt_sha256":sha256(&serde_json::to_vec_pretty(receipt).unwrap()),
        "source_host":"synthetic-producer", "owner_last_reported":"producer", "gate_reported":"paused",
        "next_step_proposed":"Review the synthetic terminal attempt", "recipients":["reviewer-ai"],
        "assurances":{
            "record_kind":"unsigned_local_observation", "receipt_claims":"unverified", "owner_status":"last_reported_only",
            "source_identity":"operator_reported_not_authenticated", "gate_authority":"existing_board", "human_awareness":"unknown",
            "independent_review":"absent", "execution_authorized":false,"lease_authority":false,"scientific_admission":false,"publication_authorized":false
        }
    })).unwrap()
}

fn binding(package: &Package) -> Binding {
    serde_json::from_slice(&package.artifact.inline_payload_bytes().unwrap().unwrap()).unwrap()
}

fn resign(package: &mut Package, payload: &Binding) {
    let a = &package.artifact;
    package.artifact = ArtifactDraft {
        skill: a.skill.clone(),
        agent: a.agent.clone(),
        topic: a.topic.clone(),
        input_fingerprint: a.input_fingerprint.clone(),
        output_cid: a.output_cid.clone(),
        parent_artifact_ids: a.parent_artifact_ids.clone(),
        schema_tags: a.schema_tags.clone(),
        payload: Some(PayloadRef::inline_json(payload).unwrap()),
    }
    .seal(&ArtifactSigner::from_seed([2; 32]), NOW)
    .unwrap();
}

#[test]
fn verifies_exact_terminal_chain_without_promoting_execution_or_authority() {
    let f = Fixture::new(false);
    let p = decode_package(&serde_json::to_vec(&f.package()).unwrap()).unwrap();
    let v = verify(&p, &f.context()).unwrap();
    assert_eq!(v.outcome, Outcome::Succeeded);
    assert_eq!(v.receipt_sha256, sha256(&f.receipt_bytes));
    assert_eq!(v.event_sha256, sha256(&f.event_bytes));
    assert_eq!(v.expected_posyn_root, ROOT);
    assert_eq!(v.execution, "not_verified");
    assert_eq!(v.scientific_review, "not_established");
    assert_eq!(v.human_awareness, "unknown");
    assert!(
        !v.execution_authorized && !v.publication_authorized && !v.authorization_reference_verified
    );
    assert_eq!(
        v.content_hash,
        verify(&p, &f.context()).unwrap().content_hash
    );
}

#[test]
fn delegated_ai_may_sign_evidence_binding_but_cannot_claim_human_review() {
    let f = Fixture::new(true);
    let v = verify(&f.package(), &f.context()).unwrap();
    assert_eq!(v.signer_id, "producer-ai");
    assert_eq!(v.scientific_review, "not_established");
    assert!(create(&f.identities[1], &f.context()).is_err());
    assert!(create(&f.identities[2], &f.context()).is_err());
}

#[test]
fn timeout_preserves_partial_outputs_and_missing_outputs() {
    let mut f = Fixture::new(false);
    f.expected.expected_outputs.push("complete.json".into());
    f.receipt.missing_outputs.push("complete.json".into());
    f.receipt.outcome = Outcome::TimedOut;
    f.receipt.failure_code = Some(FailureCode::TaskTimeout);
    f.repin();
    let p = f.package();
    assert_eq!(verify(&p, &f.context()).unwrap().outcome, Outcome::TimedOut);
    assert_eq!(binding(&p).receipt.missing_outputs, vec!["complete.json"]);
    f.receipt.outcome = Outcome::Succeeded;
    f.receipt.failure_code = None;
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
}

#[test]
fn cancelled_and_worker_failure_remain_failures_even_with_retained_outputs() {
    for (outcome, code) in [
        (Outcome::Cancelled, FailureCode::Cancelled),
        (Outcome::Failed, FailureCode::WorkerFailed),
    ] {
        let mut f = Fixture::new(false);
        f.receipt.outcome = outcome.clone();
        f.receipt.failure_code = Some(code);
        f.repin();
        assert_eq!(verify(&f.package(), &f.context()).unwrap().outcome, outcome);
    }
}

#[test]
fn exact_event_and_receipt_bytes_not_just_parsed_fields_are_required() {
    let mut f = Fixture::new(false);
    let p = f.package();
    f.event_bytes.push(b' ');
    assert!(verify(&p, &f.context()).is_err());
    f.event_bytes.pop();
    f.receipt_bytes.push(b' ');
    assert!(verify(&p, &f.context()).is_err());
    f.receipt_bytes.pop();
    assert!(verify(&p, &f.context()).is_ok());
}

#[test]
fn changed_attempt_cannot_replay_old_signed_binding_even_after_repinning() {
    let mut f = Fixture::new(false);
    let p = f.package();
    f.expected.attempt_id = "attempt-2".into();
    f.receipt.attempt_id = "attempt-2".into();
    f.repin();
    assert_eq!(
        verify(&p, &f.context()).unwrap_err().0,
        "attempt_binding_mismatch"
    );
    assert!(verify(&f.package(), &f.context()).is_ok());
}

#[test]
fn wrong_result_hash_size_and_missing_members_are_refused_after_repinning() {
    let mut f = Fixture::new(false);
    f.receipt.outputs[0].blake3 = format!("blake3:{}", "a".repeat(64));
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    let mut f = Fixture::new(false);
    f.receipt.outputs[0].size_bytes += 1;
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    let mut f = Fixture::new(false);
    f.receipt.outputs.clear();
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    let mut f = Fixture::new(false);
    f.receipt.missing_outputs.push("result.json".into());
    f.receipt.outcome = Outcome::Failed;
    f.receipt.failure_code = Some(FailureCode::MissingOutput);
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
}

#[test]
fn signatures_and_signed_bindings_are_both_checked() {
    let f = Fixture::new(false);
    let mut p = f.package();
    p.artifact.signature = "00".repeat(64);
    assert!(verify(&p, &f.context()).is_err());
    let mut p = f.package();
    let mut b = binding(&p);
    b.event_sha256 = sha256(b"another event");
    resign(&mut p, &b);
    assert_eq!(
        verify(&p, &f.context()).unwrap_err().0,
        "attempt_binding_mismatch"
    );
}

#[test]
fn replacement_original_evidence_requires_new_binding() {
    let mut f = Fixture::new(false);
    let p = f.package();
    let Statement::Evidence { bundle, .. } =
        serde_json::from_slice(&f.evidence.artifact.inline_payload_bytes().unwrap().unwrap())
            .unwrap()
    else {
        panic!()
    };
    f.evidence = create_evidence(
        &f.identities[1],
        &f.policy,
        bundle,
        None,
        Audience::Synthetic,
        NOW - 1,
    )
    .unwrap();
    assert!(verify(&p, &f.context()).is_err());
}

#[test]
fn changed_scope_and_root_refuse() {
    let mut f = Fixture::new(false);
    let p = f.package();
    f.policy.current_scope.revision += 1;
    f.repin();
    assert!(verify(&p, &f.context()).is_err());
    let mut f = Fixture::new(false);
    let p = f.package();
    f.policy.expected_posyn_root = format!("blake3:{}", "b".repeat(64));
    f.repin();
    assert!(verify(&p, &f.context()).is_err());
}

#[test]
fn revoked_expired_or_untrusted_signer_is_refused() {
    let mut f = Fixture::new(true);
    let p = f.package();
    f.policy
        .revoked_keys
        .push(f.identities[3].identity().public_key.clone());
    f.repin();
    assert!(verify(&p, &f.context()).is_err());
    let mut f = Fixture::new(true);
    let p = f.package();
    let Statement::Evidence {
        delegation: Some(d),
        ..
    } = serde_json::from_slice(&f.evidence.artifact.inline_payload_bytes().unwrap().unwrap())
        .unwrap()
    else {
        panic!()
    };
    f.policy
        .revoked_delegations
        .push(format!("blake3:{}", d.artifact.content_hash));
    f.repin();
    assert!(verify(&p, &f.context()).is_err());
    let f = Fixture::new(true);
    let p = f.package();
    let mut context = f.context();
    context.now = NOW + 500;
    assert!(verify(&p, &context).is_err());
    let mut f = Fixture::new(true);
    let p = f.package();
    f.policy.trusted_identities.pop();
    f.repin();
    assert!(verify(&p, &f.context()).is_err());
}

#[test]
fn timing_must_fit_held_window_and_evidence_must_follow_terminal_receipt() {
    let mut f = Fixture::new(false);
    f.receipt.finished_at_unix = NOW + 1;
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    let mut f = Fixture::new(false);
    f.receipt.started_at_unix = NOW - 6;
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    let mut f = Fixture::new(false);
    f.expected.expires_at_unix = NOW;
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
}

#[test]
fn input_environment_authorization_and_job_id_must_match_held_expectation() {
    for change in 0..4 {
        let mut f = Fixture::new(false);
        match change {
            0 => f.receipt.inputs_sha256 = sha256(b"changed"),
            1 => f.receipt.environment_sha256 = sha256(b"changed"),
            2 => f.receipt.authorization_sha256 = sha256(b"changed"),
            _ => f.receipt.job_id = "different-job".into(),
        }
        f.repin();
        assert!(create(&f.identities[1], &f.context()).is_err());
    }
}

#[test]
fn unknown_duplicate_and_unsafe_fields_fail_closed() {
    let mut f = Fixture::new(false);
    let mut value = serde_json::to_value(&f.receipt).unwrap();
    value["execution_authorized"] = true.into();
    f.receipt_bytes = serde_json::to_vec(&value).unwrap();
    f.expected.receipt_sha256 = sha256(&f.receipt_bytes);
    let mut event: serde_json::Value = serde_json::from_slice(&f.event_bytes).unwrap();
    event["receipt_sha256"] = f.expected.receipt_sha256.clone().into();
    f.event_bytes = serde_json::to_vec(&event).unwrap();
    f.expected.event_sha256 = sha256(&f.event_bytes);
    f.expectation_bytes = serde_json::to_vec(&f.expected).unwrap();
    assert_eq!(
        create(&f.identities[1], &f.context()).unwrap_err().0,
        "invalid_fields"
    );
    let mut f = Fixture::new(false);
    f.expected.expected_outputs.push("result.json".into());
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    let mut f = Fixture::new(false);
    f.expected.expected_outputs = vec!["../result.json".into()];
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    let f = Fixture::new(false);
    let bytes = serde_json::to_string(&f.package()).unwrap();
    let duplicate = bytes.replacen('{', "{\"schema\":\"bad\",", 1);
    assert!(decode_package(duplicate.as_bytes()).is_err());
}

#[test]
fn zero_output_failure_is_valid_but_cannot_be_promoted_to_success() {
    let mut f = Fixture::new(false);
    let root = "blake3:f8bd1eee8d1820c91a92a82d8f54c7227992fac822076f75bd833ebc18d04790";
    let bundle = import_bundle(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../demo/federation/fixtures/terminal-no-results"),
        root,
    )
    .unwrap();
    let failure = parse_manifest(&bundle)
        .unwrap()
        .payload
        .artifacts
        .into_iter()
        .find(|a| a.role == Role::Result)
        .unwrap();
    f.receipt.retained_failure_record = Some(OutputIdentity {
        name: failure.name,
        blake3: failure.digest,
        size_bytes: failure.size_bytes,
    });
    f.policy.expected_posyn_root = root.into();
    f.evidence = create_evidence(
        &f.identities[1],
        &f.policy,
        bundle,
        None,
        Audience::Synthetic,
        NOW,
    )
    .unwrap();
    f.receipt.outcome = Outcome::Failed;
    f.receipt.failure_code = Some(FailureCode::MissingOutput);
    f.receipt.outputs.clear();
    f.receipt.missing_outputs = vec!["result.json".into()];
    f.repin();
    let v = verify(&f.package(), &f.context()).unwrap();
    assert_eq!(v.outcome, Outcome::Failed);
    assert_eq!(v.failure_code, Some(FailureCode::MissingOutput));
    f.receipt.failure_code = Some(FailureCode::WorkerFailed);
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    f.receipt.failure_code = Some(FailureCode::MissingOutput);
    let retained = f.receipt.retained_failure_record.take();
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    f.receipt.retained_failure_record = retained;
    f.receipt.outcome = Outcome::Succeeded;
    f.receipt.failure_code = None;
    f.receipt.missing_outputs.clear();
    f.repin();
    assert!(create(&f.identities[1], &f.context()).is_err());
    // A fresh expectation cannot reclassify the reserved failed report as the
    // successful scientific output, even with all identities repinned.
    let retained = f.receipt.retained_failure_record.take().unwrap();
    f.expected.expected_outputs = vec![retained.name.clone()];
    f.receipt.outputs = vec![retained];
    f.repin();
    assert_eq!(
        create(&f.identities[1], &f.context()).unwrap_err().0,
        "failure_record_is_not_a_produced_output"
    );
}

#[test]
fn repinned_observation_cannot_claim_another_receipt_or_execution_authority() {
    for mutation in 0..3 {
        let mut f = Fixture::new(false);
        let mut event: serde_json::Value = serde_json::from_slice(&f.event_bytes).unwrap();
        match mutation {
            0 => event["receipt_sha256"] = sha256(b"another receipt").into(),
            1 => event["source_session"] = "another-attempt".into(),
            _ => event["assurances"]["execution_authorized"] = true.into(),
        }
        f.event_bytes = serde_json::to_vec(&event).unwrap();
        f.expected.event_sha256 = sha256(&f.event_bytes);
        f.expectation_bytes = serde_json::to_vec(&f.expected).unwrap();
        assert!(create(&f.identities[1], &f.context()).is_err());
    }
}
