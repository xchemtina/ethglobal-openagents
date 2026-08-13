use crate::model::{
    Claim, ClaimStatus, Conflict, DiscoveryDossier, DiscoveryError, Governance, Hypothesis,
    ProposedTest, Provenance, Source, SourceKind, Support, DOSSIER_SCHEMA, MAX_AGENT_IDS,
    MAX_AUTHORS, MAX_CLAIMS, MAX_CONFLICTS, MAX_EVIDENCE_BYTES, MAX_EVIDENCE_TOTAL, MAX_HYPOTHESES,
    MAX_ID_BYTES, MAX_MODEL_IDS, MAX_OPEN_QUESTIONS, MAX_PARENT_IDS, MAX_SOURCES,
    MAX_SUMMARY_BYTES, MAX_SUPPORT_PER_CLAIM, MAX_SUPPORT_TOTAL, MAX_TESTS, MAX_TEXT_BYTES,
};
use crate::strict_json::parse_strict_json;
use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

static LOCAL_PATH_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:file://|(?:^|[ \t\r\n\f:=,"'(\[;])(?:~[/\\]|\.\.?[/\\]|/(?:users|home|private|tmp|var|etc|opt|srv|root|volumes|mnt|run|dev|proc|sys)(?:[/\\]|$)|[a-z]:[/\\]|\\\\[^ \t\r\n\f]+))"#)
        .expect("fixed local-path regex")
});
static SECRET_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:-----begin[ a-z0-9_-]*private key-----|(?:^|[^a-z0-9_])[a-z0-9_-]*(?:api[_-]?key|access[_-]?token|authorization|password|private[_-]?key)[ \t\r\n\f]*[:=][ \t\r\n\f]*[^ \t\r\n\f]+|(?:^|[^a-z0-9_])bearer[ \t\r\n\f]+[^ \t\r\n\f]{8,}|(?:^|[^a-z0-9_])sk-[a-z0-9_-]{16,}|(?:^|[^a-z0-9_])gh[pousr]_[a-z0-9]{16,}|(?:^|[^a-z0-9_])akia[0-9a-z]{16}|(?:^|[^a-z0-9_])xox[baprs]-[a-z0-9-]{10,}|(?:^|[^a-z0-9_])hf_[a-z0-9]{20,}|(?:^|[^a-z0-9_])glpat-[a-z0-9_-]{20,})")
        .expect("fixed secret regex")
});
static HTTPS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https://(?:[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?)(?:\.(?:[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?))*(?::443)?(?:/[A-Za-z0-9._~/%-]*)?$")
        .expect("fixed HTTPS regex")
});
/// Decode and validate one discovery dossier without performing I/O.
///
/// # Errors
///
/// Returns a stable error for malformed JSON, a non-closed schema, or a failed
/// semantic invariant.
pub fn decode_dossier(bytes: &[u8]) -> Result<DiscoveryDossier, DiscoveryError> {
    let value = parse_strict_json(bytes)?;
    let dossier: DiscoveryDossier = serde_json::from_value(value)
        .map_err(|_| DiscoveryError::new("invalid-dossier-shape", "$"))?;
    validate_dossier(&dossier)?;
    Ok(dossier)
}

fn bounded(items: usize, minimum: usize, maximum: usize, at: &str) -> Result<(), DiscoveryError> {
    if (minimum..=maximum).contains(&items) {
        Ok(())
    } else {
        Err(DiscoveryError::new("invalid-count", at))
    }
}

fn validate_text(value: &str, maximum: usize, at: &str) -> Result<(), DiscoveryError> {
    if value.chars().all(is_wire_whitespace) {
        return Err(DiscoveryError::new("empty-string", at));
    }
    if value.len() > maximum {
        return Err(DiscoveryError::new("string-too-long", at));
    }
    if value.chars().any(char::is_control) {
        return Err(DiscoveryError::new("control-character-not-allowed", at));
    }
    if looks_like_local_path(value) {
        return Err(DiscoveryError::new("local-path-not-allowed", at));
    }
    if looks_like_secret(value) {
        return Err(DiscoveryError::new("secret-like-text-not-allowed", at));
    }
    Ok(())
}

fn is_wire_whitespace(value: char) -> bool {
    matches!(
        value,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
    )
}

fn looks_like_local_path(value: &str) -> bool {
    LOCAL_PATH_PATTERN.is_match(&value.to_ascii_lowercase())
}

fn looks_like_secret(value: &str) -> bool {
    SECRET_PATTERN.is_match(&value.to_ascii_lowercase())
}

fn validate_identifier(value: &str, at: &str) -> Result<(), DiscoveryError> {
    validate_text(value, MAX_ID_BYTES, at)?;
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(DiscoveryError::new("invalid-identifier", at));
    };
    if !first.is_ascii_alphanumeric()
        || !bytes.all(|item| item.is_ascii_alphanumeric() || b"._:-".contains(&item))
    {
        return Err(DiscoveryError::new("invalid-identifier", at));
    }
    Ok(())
}

pub(crate) fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|item| item.is_ascii_digit() || (b'a'..=b'f').contains(&item))
}

fn validate_sha256(value: &str, at: &str) -> Result<(), DiscoveryError> {
    if is_lower_hex(value, 64) {
        Ok(())
    } else {
        Err(DiscoveryError::new("invalid-sha256", at))
    }
}

pub(crate) fn validate_artifact_id(value: &str, at: &str) -> Result<(), DiscoveryError> {
    if value
        .strip_prefix("art_")
        .is_some_and(|suffix| is_lower_hex(suffix, 16))
    {
        Ok(())
    } else {
        Err(DiscoveryError::new("invalid-artifact-id", at))
    }
}

fn sorted_unique(values: &[String], at: &str) -> Result<(), DiscoveryError> {
    if values
        .windows(2)
        .any(|pair| utf16_compare(&pair[0], &pair[1]) != Ordering::Less)
    {
        return Err(DiscoveryError::new("unsorted-or-duplicate", at));
    }
    Ok(())
}

fn utf16_compare(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn support_compare(left: &Support, right: &Support) -> Ordering {
    utf16_compare(&left.source_id, &right.source_id)
        .then_with(|| utf16_compare(&left.locator, &right.locator))
        .then_with(|| utf16_compare(&left.evidence_span, &right.evidence_span))
}

fn sorted_unique_ids<T>(
    items: &[T],
    id: impl Fn(&T) -> &str,
    at: &str,
) -> Result<(), DiscoveryError> {
    if items.windows(2).any(|pair| id(&pair[0]) >= id(&pair[1])) {
        return Err(DiscoveryError::new("unsorted-or-duplicate-id", at));
    }
    Ok(())
}

fn validate_locator(kind: &SourceKind, value: &str, at: &str) -> Result<(), DiscoveryError> {
    validate_text(value, 2048, at)?;
    let valid = match kind {
        SourceKind::Doi => {
            let mut parts = value.splitn(2, '/');
            let prefix = parts.next().unwrap_or_default();
            let suffix = parts.next().unwrap_or_default();
            prefix.strip_prefix("10.").is_some_and(|digits| {
                (4..=9).contains(&digits.len()) && digits.bytes().all(|c| c.is_ascii_digit())
            }) && !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-._;()/:".contains(&c))
        }
        SourceKind::Arxiv => {
            let body = value.strip_prefix("arXiv:").unwrap_or(value);
            let versionless = body
                .rsplit_once('v')
                .filter(|(_, version)| {
                    !version.is_empty() && version.bytes().all(|c| c.is_ascii_digit())
                })
                .map_or(body, |(base, _)| base);
            versionless.len() >= 9
                && versionless.as_bytes().get(4) == Some(&b'.')
                && versionless[..4].bytes().all(|c| c.is_ascii_digit())
                && versionless[5..].bytes().all(|c| c.is_ascii_digit())
                && (4..=5).contains(&versionless[5..].len())
        }
        SourceKind::Https => HTTPS_PATTERN.is_match(value) && complete_percent_escapes(value),
        SourceKind::OpenagentsArtifact => value
            .strip_prefix("art_")
            .is_some_and(|suffix| is_lower_hex(suffix, 16)),
    };
    if valid {
        Ok(())
    } else {
        Err(DiscoveryError::new("invalid-locator", at))
    }
}

fn complete_percent_escapes(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

fn validate_header(dossier: &DiscoveryDossier) -> Result<(), DiscoveryError> {
    if dossier.schema != DOSSIER_SCHEMA {
        return Err(DiscoveryError::new("invalid-schema", "$.schema"));
    }
    validate_identifier(&dossier.dossier_id, "$.dossier_id")?;
    validate_text(&dossier.title, 1024, "$.title")?;
    validate_text(&dossier.research_question, 1024, "$.research_question")?;
    validate_text(&dossier.summary, MAX_SUMMARY_BYTES, "$.summary")?;
    Ok(())
}

fn validate_source(source: &Source, index: usize) -> Result<(), DiscoveryError> {
    let at = format!("$.sources[{index}]");
    validate_identifier(&source.id, &format!("{at}.id"))?;
    validate_locator(&source.kind, &source.locator, &format!("{at}.locator"))?;
    validate_text(&source.title, 1024, &format!("{at}.title"))?;
    bounded(
        source.authors.len(),
        0,
        MAX_AUTHORS,
        &format!("{at}.authors"),
    )?;
    for (author_index, author) in source.authors.iter().enumerate() {
        validate_text(author, 512, &format!("{at}.authors[{author_index}]"))?;
    }
    if !(0..=9999).contains(&source.year) || source.retrieved_at_unix < 0 {
        return Err(DiscoveryError::new("invalid-integer", at));
    }
    validate_text(&source.license, 256, &format!("{at}.license"))?;
    if let Some(hash) = &source.content_sha256 {
        validate_sha256(hash, &format!("{at}.content_sha256"))?;
    }
    Ok(())
}

fn validate_sources(sources: &[Source]) -> Result<BTreeSet<&str>, DiscoveryError> {
    bounded(sources.len(), 1, MAX_SOURCES, "$.sources")?;
    sorted_unique_ids(sources, |item| &item.id, "$.sources")?;
    let mut source_ids = BTreeSet::new();
    for (index, source) in sources.iter().enumerate() {
        validate_source(source, index)?;
        source_ids.insert(source.id.as_str());
    }
    Ok(source_ids)
}

fn validate_support(
    support: &Support,
    at: &str,
    source_ids: &BTreeSet<&str>,
) -> Result<usize, DiscoveryError> {
    validate_identifier(&support.source_id, &format!("{at}.source_id"))?;
    if !source_ids.contains(support.source_id.as_str()) {
        return Err(DiscoveryError::new(
            "unknown-reference",
            format!("{at}.source_id"),
        ));
    }
    validate_text(&support.locator, 512, &format!("{at}.locator"))?;
    validate_text(
        &support.evidence_span,
        MAX_EVIDENCE_BYTES,
        &format!("{at}.evidence_span"),
    )?;
    Ok(support.evidence_span.len())
}

fn validate_claim(
    claim: &Claim,
    index: usize,
    source_ids: &BTreeSet<&str>,
) -> Result<usize, DiscoveryError> {
    let at = format!("$.claims[{index}]");
    validate_identifier(&claim.id, &format!("{at}.id"))?;
    validate_text(&claim.statement, MAX_TEXT_BYTES, &format!("{at}.statement"))?;
    validate_text(
        &claim.uncertainty,
        MAX_TEXT_BYTES,
        &format!("{at}.uncertainty"),
    )?;
    bounded(
        claim.support.len(),
        1,
        MAX_SUPPORT_PER_CLAIM,
        &format!("{at}.support"),
    )?;
    if claim
        .support
        .windows(2)
        .any(|pair| support_compare(&pair[0], &pair[1]) != Ordering::Less)
    {
        return Err(DiscoveryError::new(
            "unsorted-or-duplicate",
            format!("{at}.support"),
        ));
    }
    let mut evidence_total = 0_usize;
    for (support_index, support) in claim.support.iter().enumerate() {
        evidence_total = evidence_total.saturating_add(validate_support(
            support,
            &format!("{at}.support[{support_index}]"),
            source_ids,
        )?);
    }
    Ok(evidence_total)
}

fn validate_claims<'a>(
    claims: &'a [Claim],
    source_ids: &BTreeSet<&str>,
) -> Result<BTreeSet<&'a str>, DiscoveryError> {
    bounded(claims.len(), 1, MAX_CLAIMS, "$.claims")?;
    sorted_unique_ids(claims, |item| &item.id, "$.claims")?;
    let mut claim_ids = BTreeSet::new();
    let mut support_total = 0_usize;
    let mut evidence_total = 0_usize;
    for (index, claim) in claims.iter().enumerate() {
        evidence_total = evidence_total.saturating_add(validate_claim(claim, index, source_ids)?);
        support_total += claim.support.len();
        claim_ids.insert(claim.id.as_str());
    }
    if support_total > MAX_SUPPORT_TOTAL {
        return Err(DiscoveryError::new("invalid-count", "$.claims[*].support"));
    }
    if evidence_total > MAX_EVIDENCE_TOTAL {
        return Err(DiscoveryError::new("evidence-byte-limit", "$.claims"));
    }
    Ok(claim_ids)
}

fn validate_hypothesis(
    hypothesis: &Hypothesis,
    index: usize,
    claim_ids: &BTreeSet<&str>,
) -> Result<(), DiscoveryError> {
    let at = format!("$.hypotheses[{index}]");
    validate_identifier(&hypothesis.id, &format!("{at}.id"))?;
    validate_text(
        &hypothesis.statement,
        MAX_TEXT_BYTES,
        &format!("{at}.statement"),
    )?;
    validate_text(
        &hypothesis.prediction,
        MAX_TEXT_BYTES,
        &format!("{at}.prediction"),
    )?;
    validate_text(
        &hypothesis.falsifier,
        MAX_TEXT_BYTES,
        &format!("{at}.falsifier"),
    )?;
    validate_text(
        &hypothesis.uncertainty,
        MAX_TEXT_BYTES,
        &format!("{at}.uncertainty"),
    )?;
    bounded(
        hypothesis.rationale_claim_ids.len(),
        1,
        MAX_CLAIMS,
        &format!("{at}.rationale_claim_ids"),
    )?;
    sorted_unique(
        &hypothesis.rationale_claim_ids,
        &format!("{at}.rationale_claim_ids"),
    )?;
    if hypothesis
        .rationale_claim_ids
        .iter()
        .any(|id| !claim_ids.contains(id.as_str()))
    {
        return Err(DiscoveryError::new(
            "unknown-reference",
            format!("{at}.rationale_claim_ids"),
        ));
    }
    Ok(())
}

fn validate_hypotheses<'a>(
    hypotheses: &'a [Hypothesis],
    claim_ids: &BTreeSet<&str>,
) -> Result<BTreeSet<&'a str>, DiscoveryError> {
    bounded(hypotheses.len(), 1, MAX_HYPOTHESES, "$.hypotheses")?;
    sorted_unique_ids(hypotheses, |item| &item.id, "$.hypotheses")?;
    let mut hypothesis_ids = BTreeSet::new();
    for (index, hypothesis) in hypotheses.iter().enumerate() {
        validate_hypothesis(hypothesis, index, claim_ids)?;
        hypothesis_ids.insert(hypothesis.id.as_str());
    }
    Ok(hypothesis_ids)
}

fn validate_proposed_test(
    test: &ProposedTest,
    index: usize,
    hypothesis_ids: &BTreeSet<&str>,
) -> Result<(), DiscoveryError> {
    let at = format!("$.proposed_tests[{index}]");
    validate_identifier(&test.id, &format!("{at}.id"))?;
    validate_identifier(&test.hypothesis_id, &format!("{at}.hypothesis_id"))?;
    if !hypothesis_ids.contains(test.hypothesis_id.as_str()) {
        return Err(DiscoveryError::new(
            "unknown-reference",
            format!("{at}.hypothesis_id"),
        ));
    }
    for (field, value) in [
        ("objective", &test.objective),
        ("method_summary", &test.method_summary),
        ("positive_signal", &test.positive_signal),
        ("falsifying_signal", &test.falsifying_signal),
    ] {
        validate_text(value, MAX_TEXT_BYTES, &format!("{at}.{field}"))?;
    }
    Ok(())
}

fn validate_proposed_tests(
    tests: &[ProposedTest],
    hypothesis_ids: &BTreeSet<&str>,
) -> Result<(), DiscoveryError> {
    bounded(tests.len(), 1, MAX_TESTS, "$.proposed_tests")?;
    sorted_unique_ids(tests, |item| &item.id, "$.proposed_tests")?;
    let mut covered = BTreeSet::new();
    for (index, test) in tests.iter().enumerate() {
        validate_proposed_test(test, index, hypothesis_ids)?;
        covered.insert(test.hypothesis_id.as_str());
    }
    if !hypothesis_ids.is_subset(&covered) {
        return Err(DiscoveryError::new(
            "missing-proposed-test",
            "$.proposed_tests",
        ));
    }
    Ok(())
}

fn validate_conflict<'a>(
    conflict: &'a Conflict,
    index: usize,
    claim_ids: &BTreeSet<&str>,
) -> Result<BTreeSet<&'a str>, DiscoveryError> {
    let at = format!("$.conflicts[{index}]");
    validate_identifier(&conflict.id, &format!("{at}.id"))?;
    bounded(
        conflict.claim_ids.len(),
        2,
        MAX_CLAIMS,
        &format!("{at}.claim_ids"),
    )?;
    sorted_unique(&conflict.claim_ids, &format!("{at}.claim_ids"))?;
    let mut conflicting = BTreeSet::new();
    for claim_id in &conflict.claim_ids {
        if !claim_ids.contains(claim_id.as_str()) {
            return Err(DiscoveryError::new(
                "unknown-reference",
                format!("{at}.claim_ids"),
            ));
        }
        conflicting.insert(claim_id.as_str());
    }
    validate_text(
        &conflict.description,
        MAX_TEXT_BYTES,
        &format!("{at}.description"),
    )?;
    Ok(conflicting)
}

fn validate_conflicts(
    conflicts: &[Conflict],
    claims: &[Claim],
    claim_ids: &BTreeSet<&str>,
) -> Result<(), DiscoveryError> {
    bounded(conflicts.len(), 0, MAX_CONFLICTS, "$.conflicts")?;
    sorted_unique_ids(conflicts, |item| &item.id, "$.conflicts")?;
    let mut conflicting = BTreeSet::new();
    for (index, conflict) in conflicts.iter().enumerate() {
        conflicting.extend(validate_conflict(conflict, index, claim_ids)?);
    }
    for (index, claim) in claims.iter().enumerate() {
        if claim.status == ClaimStatus::Conflicting && !conflicting.contains(claim.id.as_str()) {
            return Err(DiscoveryError::new(
                "missing-conflict",
                format!("$.claims[{index}].status"),
            ));
        }
    }
    Ok(())
}

fn validate_open_questions(open_questions: &[String]) -> Result<(), DiscoveryError> {
    bounded(
        open_questions.len(),
        0,
        MAX_OPEN_QUESTIONS,
        "$.open_questions",
    )?;
    sorted_unique(open_questions, "$.open_questions")?;
    for (index, question) in open_questions.iter().enumerate() {
        validate_text(
            question,
            MAX_TEXT_BYTES,
            &format!("$.open_questions[{index}]"),
        )?;
    }
    Ok(())
}

fn validate_provenance(provenance: &Provenance) -> Result<(), DiscoveryError> {
    if provenance.producer != "openagents-rust" {
        return Err(DiscoveryError::new(
            "provenance-mismatch",
            "$.provenance.producer",
        ));
    }
    validate_text(
        &provenance.producer_version,
        128,
        "$.provenance.producer_version",
    )?;
    bounded(
        provenance.parent_artifact_ids.len(),
        0,
        MAX_PARENT_IDS,
        "$.provenance.parent_artifact_ids",
    )?;
    sorted_unique(
        &provenance.parent_artifact_ids,
        "$.provenance.parent_artifact_ids",
    )?;
    for (index, id) in provenance.parent_artifact_ids.iter().enumerate() {
        validate_artifact_id(id, &format!("$.provenance.parent_artifact_ids[{index}]"))?;
    }
    for (field, values, maximum) in [
        ("agent_ids", &provenance.agent_ids, MAX_AGENT_IDS),
        ("model_ids", &provenance.model_ids, MAX_MODEL_IDS),
    ] {
        bounded(values.len(), 1, maximum, &format!("$.provenance.{field}"))?;
        sorted_unique(values, &format!("$.provenance.{field}"))?;
        for (index, value) in values.iter().enumerate() {
            validate_identifier(value, &format!("$.provenance.{field}[{index}]"))?;
        }
    }
    validate_sha256(&provenance.prompt_sha256, "$.provenance.prompt_sha256")?;
    if provenance.generated_at_unix < 0 {
        return Err(DiscoveryError::new(
            "invalid-integer",
            "$.provenance.generated_at_unix",
        ));
    }
    Ok(())
}

fn validate_governance(governance: &Governance) -> Result<(), DiscoveryError> {
    if !governance.report_only
        || !governance.requires_human_decision
        || governance.decision != "pending"
        || governance.authority.database_write
        || governance.authority.dispatch
        || governance.authority.execution
        || governance.authority.publication
    {
        return Err(DiscoveryError::new("governance-mismatch", "$.governance"));
    }
    Ok(())
}

/// Validate the closed semantic contract shared with Zopf.
///
/// # Errors
///
/// Returns the first stable invariant failure and its schema location.
pub fn validate_dossier(dossier: &DiscoveryDossier) -> Result<(), DiscoveryError> {
    validate_header(dossier)?;
    let source_ids = validate_sources(&dossier.sources)?;
    let claim_ids = validate_claims(&dossier.claims, &source_ids)?;
    let hypothesis_ids = validate_hypotheses(&dossier.hypotheses, &claim_ids)?;
    validate_proposed_tests(&dossier.proposed_tests, &hypothesis_ids)?;
    validate_conflicts(&dossier.conflicts, &dossier.claims, &claim_ids)?;
    validate_open_questions(&dossier.open_questions)?;
    validate_provenance(&dossier.provenance)?;
    validate_governance(&dossier.governance)
}

fn write_canonical(value: &Value, output: &mut String) -> Result<(), DiscoveryError> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => {
            if value.as_i64().is_none() {
                return Err(DiscoveryError::new("floating-number-not-allowed", "$"));
            }
            output.push_str(&value.to_string());
        }
        Value::String(value) => output.push_str(
            &serde_json::to_string(value)
                .map_err(|_| DiscoveryError::new("json-serialization-failed", "$"))?,
        ),
        Value::Array(values) => {
            output.push('[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical(item, output)?;
            }
            output.push(']');
        }
        Value::Object(values) => {
            output.push('{');
            let sorted = values.iter().collect::<BTreeMap<_, _>>();
            for (index, (key, item)) in sorted.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key)
                        .map_err(|_| DiscoveryError::new("json-serialization-failed", "$"))?,
                );
                output.push(':');
                write_canonical(item, output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}

/// Lexical-key, whitespace-free JSON bytes used by the Rust/Zopf wire fixture.
///
/// # Errors
///
/// Returns an error if validation or serialization fails.
pub fn canonical_dossier_bytes(dossier: &DiscoveryDossier) -> Result<Vec<u8>, DiscoveryError> {
    validate_dossier(dossier)?;
    let value = serde_json::to_value(dossier)
        .map_err(|_| DiscoveryError::new("json-serialization-failed", "$"))?;
    let mut encoded = String::new();
    write_canonical(&value, &mut encoded)?;
    Ok(encoded.into_bytes())
}

/// Compute the cross-language SHA-256 of the canonical dossier bytes.
///
/// # Errors
///
/// Returns an error if the dossier is invalid or cannot be serialized.
pub fn normalized_sha256(dossier: &DiscoveryDossier) -> Result<String, DiscoveryError> {
    let bytes = canonical_dossier_bytes(dossier)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
