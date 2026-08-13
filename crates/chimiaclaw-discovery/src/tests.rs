use crate::model::{
    MAX_AGENT_IDS, MAX_AUTHORS, MAX_CLAIMS, MAX_CONFLICTS, MAX_EVIDENCE_BYTES, MAX_EVIDENCE_TOTAL,
    MAX_HYPOTHESES, MAX_MODEL_IDS, MAX_OPEN_QUESTIONS, MAX_PARENT_IDS, MAX_SOURCES,
    MAX_SUPPORT_PER_CLAIM, MAX_SUPPORT_TOTAL, MAX_TESTS,
};
use crate::*;
use chimiaclaw_artifact::{
    ArtifactDraft, ArtifactId, ArtifactSigner, PayloadEncoding, PayloadLocation, PayloadRef,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const GOLDEN: &[u8] = include_bytes!("../fixtures/discovery-dossier.valid.json");
const STRING_SAFETY: &[u8] = include_bytes!("../fixtures/discovery-string-safety-parity.v1.json");

fn dossier() -> DiscoveryDossier {
    decode_dossier(GOLDEN).expect("golden dossier")
}

fn claims_with_support(claim_count: usize, support_count: usize) -> Vec<Claim> {
    (0..claim_count)
        .map(|claim_index| {
            let mut claim = dossier().claims[0].clone();
            claim.id = format!("claim-{claim_index:03}");
            claim.support = (0..support_count)
                .map(|support_index| Support {
                    source_id: "src-001".to_owned(),
                    locator: format!("span-{support_index:03}"),
                    evidence_span: "e".to_owned(),
                })
                .collect();
            claim
        })
        .collect()
}

fn seal_mutated_draft(
    dossier: &DiscoveryDossier,
    signer: &ArtifactSigner,
    mutate: impl FnOnce(&mut ArtifactDraft),
) -> chimiaclaw_artifact::Artifact {
    let mut draft = dossier_artifact_draft(dossier).expect("draft");
    mutate(&mut draft);
    draft
        .seal(
            signer,
            u64::try_from(dossier.provenance.generated_at_unix).expect("non-negative timestamp"),
        )
        .expect("seal mutated draft")
}

#[test]
fn golden_fixture_validates_and_has_cross_host_digest() {
    let dossier = dossier();
    assert_eq!(
        normalized_sha256(&dossier).expect("digest"),
        "c52bd4a48717fde7340f4a3545a85b4013f85c60d8a3dd1d5fef30e7cb57f067"
    );
    assert_eq!(
        decode_dossier(&canonical_dossier_bytes(&dossier).expect("canonical")).expect("decode"),
        dossier
    );
}

#[test]
fn signed_artifact_round_trip_is_payload_bound() {
    let signer = ArtifactSigner::from_seed([23; 32]);
    let dossier = dossier();
    let artifact = seal_dossier(&dossier, &signer).expect("seal");
    assert_eq!(verify_export(&artifact).expect("verify"), dossier);
    let encoded = serde_json::to_vec(&artifact).expect("artifact JSON");
    assert_eq!(
        verify_export_bytes(&encoded).expect("export"),
        canonical_dossier_bytes(&dossier).expect("canonical")
    );
}

#[test]
fn signature_and_payload_tampering_fail_closed() {
    let signer = ArtifactSigner::from_seed([23; 32]);
    let mut artifact = seal_dossier(&dossier(), &signer).expect("seal");
    artifact.topic.push_str(" tampered");
    assert_eq!(
        verify_export(&artifact).expect_err("tamper").code,
        "artifact-integrity-failed"
    );

    let mut artifact = seal_dossier(&dossier(), &signer).expect("seal");
    if let Some(PayloadRef {
        location: PayloadLocation::Inline { bytes_hex },
        ..
    }) = artifact.payload.as_mut()
    {
        bytes_hex.push_str("00");
    }
    assert_eq!(
        verify_export(&artifact).expect_err("payload tamper").code,
        "artifact-integrity-failed"
    );
}

#[test]
fn strict_json_rejects_duplicate_trailing_float_range_depth_and_nodes() {
    let cases = [
        (br#"{"a":1,"a":2}"#.as_slice(), "duplicate-json-key"),
        (br"{} x".as_slice(), "trailing-json-data"),
        (br"1.5".as_slice(), "floating-number-not-allowed"),
        (br"9223372036854775808".as_slice(), "integer-out-of-range"),
    ];
    for (bytes, code) in cases {
        assert_eq!(parse_strict_json(bytes).expect_err(code).code, code);
    }
    let deep = format!(
        "{}0{}",
        "[".repeat(MAX_JSON_DEPTH + 1),
        "]".repeat(MAX_JSON_DEPTH + 1)
    );
    assert_eq!(
        parse_strict_json(deep.as_bytes()).expect_err("depth").code,
        "json-depth-limit"
    );
    let many = format!(
        "[{}]",
        std::iter::repeat_n("0", MAX_JSON_NODES)
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(
        parse_strict_json(many.as_bytes()).expect_err("nodes").code,
        "json-node-limit"
    );
    let exact_depth = format!(
        "{}0{}",
        "[".repeat(MAX_JSON_DEPTH),
        "]".repeat(MAX_JSON_DEPTH)
    );
    assert!(parse_strict_json(exact_depth.as_bytes()).is_ok());

    let exact_nodes = format!(
        "[{}]",
        std::iter::repeat_n("0", MAX_JSON_NODES - 1)
            .collect::<Vec<_>>()
            .join(",")
    );
    assert!(parse_strict_json(exact_nodes.as_bytes()).is_ok());
}

#[test]
fn strict_json_accepts_exact_byte_limit_and_rejects_plus_one() {
    let mut exact = vec![b' '; MAX_DOSSIER_BYTES];
    exact[0] = b'{';
    exact[1] = b'}';
    assert_eq!(
        parse_strict_json(&exact).expect("exact byte limit"),
        Value::Object(serde_json::Map::default())
    );

    exact.push(b' ');
    assert_eq!(
        parse_strict_json(&exact)
            .expect_err("one byte over limit")
            .code,
        "input-too-large"
    );
}

#[test]
fn semantic_mutations_never_gain_authority() {
    let mut dossier = dossier();
    dossier.governance.authority.dispatch = true;
    assert_eq!(
        validate_dossier(&dossier).expect_err("dispatch").code,
        "governance-mismatch"
    );

    let mut dossier = self::dossier();
    dossier.claims[0].support[0].source_id = "src-missing".to_owned();
    assert_eq!(
        validate_dossier(&dossier).expect_err("reference").code,
        "unknown-reference"
    );

    let mut dossier = self::dossier();
    dossier.summary = "/Users/operator/private".to_owned();
    assert_eq!(
        validate_dossier(&dossier).expect_err("path").code,
        "local-path-not-allowed"
    );

    let mut dossier = self::dossier();
    dossier.summary = "api_key=SUPERSECRET".to_owned();
    assert_eq!(
        validate_dossier(&dossier).expect_err("secret").code,
        "secret-like-text-not-allowed"
    );
}

#[test]
fn every_governance_escape_hatch_fails_closed() {
    for mutation in 0..7 {
        let mut dossier = dossier();
        match mutation {
            0 => dossier.governance.report_only = false,
            1 => dossier.governance.requires_human_decision = false,
            2 => dossier.governance.decision = "approved".to_owned(),
            3 => dossier.governance.authority.database_write = true,
            4 => dossier.governance.authority.dispatch = true,
            5 => dossier.governance.authority.execution = true,
            6 => dossier.governance.authority.publication = true,
            _ => unreachable!(),
        }
        let error = validate_dossier(&dossier).expect_err("governance mutation");
        assert_eq!(error.code, "governance-mismatch", "mutation {mutation}");
        assert_eq!(error.at, "$.governance", "mutation {mutation}");
    }
}

#[test]
fn primary_collection_counts_accept_exact_caps_and_reject_plus_one() {
    let mut dossier = dossier();
    dossier.sources = (0..MAX_SOURCES)
        .map(|index| {
            let mut source = dossier.sources[0].clone();
            source.id = format!("src-{index:03}");
            source
        })
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    let mut extra = dossier.sources[0].clone();
    extra.id = format!("src-{MAX_SOURCES:03}");
    dossier.sources.push(extra);
    assert_eq!(
        validate_dossier(&dossier).expect_err("source cap + 1").code,
        "invalid-count"
    );

    let mut dossier = self::dossier();
    dossier.claims = claims_with_support(MAX_CLAIMS, 1);
    assert!(validate_dossier(&dossier).is_ok());
    dossier.claims.push({
        let mut claim = dossier.claims[0].clone();
        claim.id = format!("claim-{MAX_CLAIMS:03}");
        claim
    });
    assert_eq!(
        validate_dossier(&dossier).expect_err("claim cap + 1").code,
        "invalid-count"
    );

    let mut dossier = self::dossier();
    dossier.hypotheses = (0..MAX_HYPOTHESES)
        .map(|index| {
            let mut hypothesis = dossier.hypotheses[0].clone();
            hypothesis.id = format!("hyp-{index:03}");
            hypothesis
        })
        .collect();
    dossier.proposed_tests = (0..MAX_HYPOTHESES)
        .map(|index| {
            let mut proposed = dossier.proposed_tests[0].clone();
            proposed.id = format!("test-{index:03}");
            proposed.hypothesis_id = format!("hyp-{index:03}");
            proposed
        })
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    dossier.hypotheses.push({
        let mut hypothesis = dossier.hypotheses[0].clone();
        hypothesis.id = format!("hyp-{MAX_HYPOTHESES:03}");
        hypothesis
    });
    dossier.proposed_tests.push({
        let mut proposed = dossier.proposed_tests[0].clone();
        proposed.id = format!("test-{MAX_HYPOTHESES:03}");
        proposed.hypothesis_id = format!("hyp-{MAX_HYPOTHESES:03}");
        proposed
    });
    assert_eq!(
        validate_dossier(&dossier)
            .expect_err("hypothesis cap + 1")
            .code,
        "invalid-count"
    );

    let mut dossier = self::dossier();
    dossier.proposed_tests = (0..MAX_TESTS)
        .map(|index| {
            let mut proposed = dossier.proposed_tests[0].clone();
            proposed.id = format!("test-{index:03}");
            proposed
        })
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    dossier.proposed_tests.push({
        let mut proposed = dossier.proposed_tests[0].clone();
        proposed.id = format!("test-{MAX_TESTS:03}");
        proposed
    });
    assert_eq!(
        validate_dossier(&dossier).expect_err("test cap + 1").code,
        "invalid-count"
    );
}

#[test]
fn nested_collection_counts_accept_exact_caps_and_reject_plus_one() {
    let mut dossier = dossier();
    dossier.sources[0].authors = (0..MAX_AUTHORS)
        .map(|index| format!("Observer {index:03}"))
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    dossier.sources[0]
        .authors
        .push(format!("Observer {MAX_AUTHORS:03}"));
    assert_eq!(
        validate_dossier(&dossier).expect_err("author cap + 1").code,
        "invalid-count"
    );

    let mut dossier = self::dossier();
    dossier.claims = claims_with_support(1, MAX_SUPPORT_PER_CLAIM);
    dossier.hypotheses[0].rationale_claim_ids = vec!["claim-000".to_owned()];
    assert!(validate_dossier(&dossier).is_ok());
    dossier.claims[0].support.push(Support {
        source_id: "src-001".to_owned(),
        locator: format!("span-{MAX_SUPPORT_PER_CLAIM:03}"),
        evidence_span: "e".to_owned(),
    });
    assert_eq!(
        validate_dossier(&dossier)
            .expect_err("support-per-claim cap + 1")
            .code,
        "invalid-count"
    );

    let mut dossier = self::dossier();
    dossier.claims = claims_with_support(MAX_CLAIMS, MAX_SUPPORT_TOTAL / MAX_CLAIMS);
    assert_eq!(
        dossier
            .claims
            .iter()
            .map(|claim| claim.support.len())
            .sum::<usize>(),
        MAX_SUPPORT_TOTAL
    );
    assert!(validate_dossier(&dossier).is_ok());
    dossier.claims[0].support.push(Support {
        source_id: "src-001".to_owned(),
        locator: "span-004".to_owned(),
        evidence_span: "e".to_owned(),
    });
    assert_eq!(
        validate_dossier(&dossier)
            .expect_err("total support cap + 1")
            .code,
        "invalid-count"
    );

    let mut dossier = self::dossier();
    dossier.open_questions = (0..MAX_OPEN_QUESTIONS)
        .map(|index| format!("Question {index:03}"))
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    dossier
        .open_questions
        .push(format!("Question {MAX_OPEN_QUESTIONS:03}"));
    assert_eq!(
        validate_dossier(&dossier)
            .expect_err("open-question cap + 1")
            .code,
        "invalid-count"
    );
}

#[test]
fn provenance_counts_accept_exact_caps_and_reject_plus_one() {
    let mut dossier = dossier();
    dossier.provenance.parent_artifact_ids = (0..MAX_PARENT_IDS)
        .map(|index| format!("art_{index:016x}"))
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    dossier
        .provenance
        .parent_artifact_ids
        .push(format!("art_{MAX_PARENT_IDS:016x}"));
    assert_eq!(
        validate_dossier(&dossier).expect_err("parent cap + 1").code,
        "invalid-count"
    );

    let mut dossier = self::dossier();
    dossier.provenance.agent_ids = (0..MAX_AGENT_IDS)
        .map(|index| format!("agent-{index:03}"))
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    dossier
        .provenance
        .agent_ids
        .push(format!("agent-{MAX_AGENT_IDS:03}"));
    assert_eq!(
        validate_dossier(&dossier).expect_err("agent cap + 1").code,
        "invalid-count"
    );

    let mut dossier = self::dossier();
    dossier.provenance.model_ids = (0..MAX_MODEL_IDS)
        .map(|index| format!("model-{index:03}"))
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    dossier
        .provenance
        .model_ids
        .push(format!("model-{MAX_MODEL_IDS:03}"));
    assert_eq!(
        validate_dossier(&dossier).expect_err("model cap + 1").code,
        "invalid-count"
    );
}

#[test]
fn provenance_agent_and_model_ids_reject_hostile_text() {
    let mut dossier = dossier();
    dossier.provenance.agent_ids = vec!["observer-α".to_owned()];
    let error = validate_dossier(&dossier).expect_err("non-ASCII agent ID");
    assert_eq!(error.code, "invalid-identifier");
    assert_eq!(error.at, "$.provenance.agent_ids[0]");

    let mut dossier = self::dossier();
    dossier.provenance.model_ids = vec!["m".repeat(129)];
    let error = validate_dossier(&dossier).expect_err("oversized model ID");
    assert_eq!(error.code, "string-too-long");
    assert_eq!(error.at, "$.provenance.model_ids[0]");
}

#[test]
fn conflict_count_accepts_exact_cap_and_rejects_plus_one() {
    let mut dossier = dossier();
    dossier.claims = claims_with_support(2, 1);
    dossier.hypotheses[0].rationale_claim_ids = vec!["claim-000".to_owned()];
    dossier.claims[0].status = ClaimStatus::Conflicting;
    dossier.claims[1].status = ClaimStatus::Conflicting;
    dossier.conflicts = (0..MAX_CONFLICTS)
        .map(|index| Conflict {
            id: format!("conflict-{index:03}"),
            claim_ids: vec!["claim-000".to_owned(), "claim-001".to_owned()],
            description: "Synthetic disagreement queued for human review.".to_owned(),
        })
        .collect();
    assert!(validate_dossier(&dossier).is_ok());
    dossier.conflicts.push(Conflict {
        id: format!("conflict-{MAX_CONFLICTS:03}"),
        claim_ids: vec!["claim-000".to_owned(), "claim-001".to_owned()],
        description: "Synthetic disagreement queued for human review.".to_owned(),
    });
    assert_eq!(
        validate_dossier(&dossier)
            .expect_err("conflict cap + 1")
            .code,
        "invalid-count"
    );
}

#[test]
fn evidence_span_and_total_byte_limits_are_exact() {
    let mut dossier = dossier();
    dossier.claims[0].support[0].evidence_span = "e".repeat(MAX_EVIDENCE_BYTES);
    assert!(validate_dossier(&dossier).is_ok());
    dossier.claims[0].support[0].evidence_span.push('e');
    assert_eq!(
        validate_dossier(&dossier)
            .expect_err("evidence span cap + 1")
            .code,
        "string-too-long"
    );

    let mut dossier = self::dossier();
    dossier.claims = claims_with_support(9, 1);
    dossier.hypotheses[0].rationale_claim_ids = vec!["claim-001".to_owned()];
    let mut evidence_index = 0_usize;
    for claim in &mut dossier.claims {
        let support_count = if evidence_index < 64 {
            MAX_SUPPORT_PER_CLAIM
        } else if evidence_index < 65 {
            2
        } else {
            0
        };
        claim.support = (0..support_count)
            .map(|support_index| {
                let length = if evidence_index < 65 {
                    MAX_EVIDENCE_BYTES
                } else {
                    MAX_EVIDENCE_TOTAL - (65 * MAX_EVIDENCE_BYTES)
                };
                evidence_index += 1;
                Support {
                    source_id: "src-001".to_owned(),
                    locator: format!("span-{support_index:03}"),
                    evidence_span: "e".repeat(length),
                }
            })
            .collect();
    }
    assert_eq!(evidence_index, 66);
    assert_eq!(
        dossier
            .claims
            .iter()
            .flat_map(|claim| &claim.support)
            .map(|support| support.evidence_span.len())
            .sum::<usize>(),
        MAX_EVIDENCE_TOTAL
    );
    assert!(validate_dossier(&dossier).is_ok());
    dossier.claims[8].support[1].evidence_span.push('e');
    assert_eq!(
        validate_dossier(&dossier)
            .expect_err("evidence total cap + 1")
            .code,
        "evidence-byte-limit"
    );
}

#[test]
fn extra_and_missing_fields_are_rejected() {
    let mut value = parse_strict_json(GOLDEN).expect("json");
    value
        .as_object_mut()
        .expect("object")
        .insert("confidence".to_owned(), Value::from(1));
    let bytes = serde_json::to_vec(&value).expect("json");
    assert_eq!(
        decode_dossier(&bytes).expect_err("extra").code,
        "invalid-dossier-shape"
    );

    value.as_object_mut().expect("object").remove("title");
    let bytes = serde_json::to_vec(&value).expect("json");
    assert_eq!(
        decode_dossier(&bytes).expect_err("missing").code,
        "invalid-dossier-shape"
    );

    let mut value = parse_strict_json(GOLDEN).expect("json");
    value["sources"][0]
        .as_object_mut()
        .expect("source")
        .remove("content_sha256");
    let bytes = serde_json::to_vec(&value).expect("json");
    assert_eq!(
        decode_dossier(&bytes)
            .expect_err("missing nullable field")
            .code,
        "invalid-dossier-shape"
    );
}

#[test]
fn raw_artifact_shape_and_schema_tag_set_fail_closed() {
    let signer = ArtifactSigner::from_seed([23; 32]);
    let artifact = seal_dossier(&dossier(), &signer).expect("seal");

    let mut value = serde_json::to_value(&artifact).expect("artifact value");
    value
        .as_object_mut()
        .expect("artifact object")
        .insert("unexpected".to_owned(), Value::Bool(true));
    assert_eq!(
        decode_artifact(&serde_json::to_vec(&value).expect("artifact bytes"))
            .expect_err("unknown artifact key")
            .code,
        "invalid-artifact-shape"
    );

    let mut value = serde_json::to_value(&artifact).expect("artifact value");
    value["schema_tags"] = Value::Array(vec![
        Value::String(DOSSIER_SCHEMA.to_owned()),
        Value::String(DOSSIER_SCHEMA.to_owned()),
    ]);
    assert_eq!(
        decode_artifact(&serde_json::to_vec(&value).expect("artifact bytes"))
            .expect_err("duplicate schema tags")
            .code,
        "artifact-contract-mismatch"
    );
}

#[test]
fn independently_signed_wrong_and_external_payloads_are_rejected() {
    let signer = ArtifactSigner::from_seed([23; 32]);
    let dossier = dossier();

    let artifact = seal_mutated_draft(&dossier, &signer, |draft| {
        draft.payload.as_mut().expect("payload").encoding = PayloadEncoding::Bytes;
    });
    assert_eq!(
        verify_export(&artifact).expect_err("non-JSON payload").code,
        "artifact-payload-not-json"
    );

    let artifact = seal_mutated_draft(&dossier, &signer, |draft| {
        draft.payload.as_mut().expect("payload").location = PayloadLocation::External {
            cid: "zg://synthetic-unresolved-payload".to_owned(),
        };
    });
    assert_eq!(
        verify_export(&artifact).expect_err("external payload").code,
        "artifact-payload-not-inline"
    );
}

#[test]
fn independently_signed_metadata_must_match_dossier_provenance() {
    let signer = ArtifactSigner::from_seed([23; 32]);
    let dossier = dossier();

    let cases = [
        seal_mutated_draft(&dossier, &signer, |draft| {
            draft
                .parent_artifact_ids
                .push(ArtifactId("art_0000000000000001".to_owned()));
        }),
        seal_mutated_draft(&dossier, &signer, |draft| {
            draft.agent.0 = "different-agent".to_owned();
        }),
        seal_mutated_draft(&dossier, &signer, |draft| {
            draft.topic = "Different topic".to_owned();
        }),
        seal_mutated_draft(&dossier, &signer, |draft| {
            draft.input_fingerprint = "1".repeat(64);
        }),
    ];
    for artifact in cases {
        assert_eq!(
            verify_export(&artifact)
                .expect_err("metadata binding mismatch")
                .code,
            "artifact-dossier-binding-mismatch"
        );
    }

    let draft = dossier_artifact_draft(&dossier).expect("draft");
    let artifact = draft
        .seal(
            &signer,
            u64::try_from(dossier.provenance.generated_at_unix).expect("timestamp") + 1,
        )
        .expect("seal timestamp mismatch");
    assert_eq!(
        verify_export(&artifact)
            .expect_err("timestamp binding mismatch")
            .code,
        "artifact-dossier-binding-mismatch"
    );
}

#[test]
fn safety_review_flag_is_preserved_without_policy_inference() {
    let signer = ArtifactSigner::from_seed([23; 32]);
    for safety_review_required in [false, true] {
        let mut dossier = dossier();
        dossier.proposed_tests[0].safety_review_required = safety_review_required;
        let artifact = seal_dossier(&dossier, &signer).expect("seal");
        let verified = verify_export(&artifact).expect("verify");
        assert_eq!(
            verified.proposed_tests[0].safety_review_required,
            safety_review_required
        );
    }
}

#[test]
fn shared_string_safety_and_ordering_corpus_is_the_wire_contract() {
    assert_eq!(
        hex::encode(Sha256::digest(STRING_SAFETY)),
        "30bddc1f4adb05d06a92355b5dd6656ad7d6b6c382ca68290c39a1b6ce72ef4c"
    );
    let corpus: Value = serde_json::from_slice(STRING_SAFETY).expect("shared corpus");
    for text in corpus["text"]["accepted"].as_array().expect("accepted") {
        let mut dossier = dossier();
        dossier.summary = text.as_str().expect("text").to_owned();
        assert!(validate_dossier(&dossier).is_ok(), "{text}");
    }
    for text in corpus["text"]["rejected"].as_array().expect("rejected") {
        let mut dossier = dossier();
        dossier.summary = text.as_str().expect("text").to_owned();
        assert!(validate_dossier(&dossier).is_err(), "{text}");
    }
    for locator in corpus["https"]["accepted"].as_array().expect("accepted") {
        let mut dossier = dossier();
        dossier.sources[0].kind = SourceKind::Https;
        dossier.sources[0].locator = locator.as_str().expect("locator").to_owned();
        assert!(validate_dossier(&dossier).is_ok(), "{locator}");
    }
    for locator in corpus["https"]["rejected"].as_array().expect("rejected") {
        let mut dossier = dossier();
        dossier.sources[0].kind = SourceKind::Https;
        dossier.sources[0].locator = locator.as_str().expect("locator").to_owned();
        assert_eq!(
            validate_dossier(&dossier)
                .expect_err("invalid locator")
                .code,
            "invalid-locator",
            "{locator}"
        );
    }
}

#[test]
fn conflicting_claims_require_an_explicit_conflict_record() {
    let mut dossier = dossier();
    let mut second = dossier.claims[0].clone();
    second.id = "claim-002".to_owned();
    second.status = ClaimStatus::Conflicting;
    dossier.claims.push(second);
    dossier.conflicts.push(Conflict {
        id: "conflict-001".to_owned(),
        claim_ids: vec!["claim-001".to_owned(), "claim-002".to_owned()],
        description: "The synthetic claims disagree and require human review.".to_owned(),
    });
    assert!(validate_dossier(&dossier).is_ok());
    dossier.conflicts.clear();
    assert_eq!(
        validate_dossier(&dossier)
            .expect_err("unqueued conflict")
            .code,
        "missing-conflict"
    );
}

#[test]
fn sorted_strings_use_the_jvm_utf16_wire_order() {
    let mut dossier = dossier();
    dossier.open_questions = vec!["𐀀".to_owned(), "".to_owned()];
    assert!(validate_dossier(&dossier).is_ok());
    dossier.open_questions.reverse();
    assert_eq!(
        validate_dossier(&dossier).expect_err("wire order").code,
        "unsorted-or-duplicate"
    );
}
