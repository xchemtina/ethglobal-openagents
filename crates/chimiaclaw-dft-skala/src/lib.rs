//! Skala / PySCF DFT result crate.
//!
//! The Rust side defines the `chem.dft.result` schema and a signed-artifact
//! signer.  The actual SCF computation lives in an operator-managed uv worker
//! reachable through `CHIMIACLAW_DFT_COMMAND` (see
//! `skills/scienceclaw-port/workers/dft/`).  The worker contract is:
//!
//! - stdin: a `chem.dft.request` JSON payload (as produced by
//!   `chimiaclaw_moladt::dft_request_artifact`).
//! - stdout: a `chem.dft.result` JSON document matching `DftResult` below.
//! - non-zero exit with a stderr message on failure.
//!
//! Compile with `--features live` to enable the in-process command wrapper that
//! pipes a `DftRequest` to the worker and parses the response.  The default
//! build is types-only so downstream crates and tests can manipulate the schema
//! without depending on `Command`/`Stdio`.

use chimiaclaw_artifact::{
    Artifact, ArtifactDraft, ArtifactError, ArtifactId, ArtifactSigner, PayloadRef,
};
use chimiaclaw_moladt::{DftRequest, MoleculeAdt};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

pub const DFT_RESULT_TAG: &str = "chem.dft.result";
pub const DFT_RESULT_SKILL: &str = "chem.dft.result.v1";
pub const DFT_WORKER_COMMAND_ENV: &str = "CHIMIACLAW_DFT_COMMAND";

/// Canonical input wrapper sent to the duck-side uv worker on stdin.
///
/// The worker receives a single JSON document containing:
/// - `request`: the `chem.dft.request` payload (functional, basis, charge, ...).
/// - `molecule_adt`: the parent `chem.molecule.adt` payload (atoms with
///   coordinates).  This is what the worker actually feeds into PySCF.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DftWorkerInput {
    pub request: DftRequest,
    pub molecule_adt: MoleculeAdt,
}

impl DftWorkerInput {
    #[must_use]
    pub fn new(request: DftRequest, molecule_adt: MoleculeAdt) -> Self {
        Self {
            request,
            molecule_adt,
        }
    }
}

/// SCF convergence summary for a `DftResult`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DftConvergence {
    pub converged: bool,
    pub n_cycles: u32,
    pub final_gradient_norm: Option<f64>,
    pub scf_threshold: Option<f64>,
}

/// Frontier orbital block (HOMO/LUMO + gap), all in Hartree atomic units.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DftOrbitalEnergies {
    pub homo_hartree: f64,
    pub lumo_hartree: f64,
    pub gap_hartree: f64,
    pub gap_ev: f64,
}

/// Dipole moment as a 3-vector plus its magnitude in Debye.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DftDipole {
    pub x_debye: f64,
    pub y_debye: f64,
    pub z_debye: f64,
    pub magnitude_debye: f64,
}

impl DftDipole {
    #[must_use]
    pub fn from_components(x: f64, y: f64, z: f64) -> Self {
        let magnitude = (x * x + y * y + z * z).sqrt();
        Self {
            x_debye: x,
            y_debye: y,
            z_debye: z,
            magnitude_debye: magnitude,
        }
    }
}

/// Wall + CPU runtime for the SCF cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DftTimings {
    pub wall_seconds: f64,
    pub cpu_seconds: Option<f64>,
}

/// Provenance block tagging which DFT stack produced the result.
///
/// `source_kind` should be one of:
/// - `"pyscf-skala-1.1"` for the canonical Skala 1.1 / def2-tzvp path
/// - `"pyscf-classical-functional"` for any non-Skala PySCF SCF
/// - `"placeholder-result"` for synthetic test fixtures (never sign as real)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DftResultProvenance {
    pub source_kind: String,
    pub source_ref: String,
    pub host: Option<String>,
    pub pyscf_version: Option<String>,
    pub skala_version: Option<String>,
    pub dispersion: Option<String>,
    pub notes: Vec<String>,
}

/// Canonical `chem.dft.result` payload.
///
/// Mirrors the JSON the duck-side uv worker emits on stdout.  The schema_tag
/// is constant; the worker MUST include it so downstream Rust validation can
/// reject mis-tagged outputs early.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DftResult {
    pub schema_tag: String,
    pub request_id: String,
    pub molecule_id: String,
    pub functional: String,
    pub basis_set: String,
    pub backend: String,
    pub total_charge: i32,
    pub multiplicity: u8,
    pub energy_hartree: f64,
    pub orbitals: Option<DftOrbitalEnergies>,
    pub dipole: Option<DftDipole>,
    pub convergence: DftConvergence,
    pub timings: DftTimings,
    pub requested_properties: Vec<String>,
    pub provenance: DftResultProvenance,
}

impl DftResult {
    /// Returns true if the schema tag is exactly `chem.dft.result` and the SCF
    /// converged.  Callers should always use this before signing.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        self.schema_tag == DFT_RESULT_TAG && self.convergence.converged
    }

    /// Human-readable one-liner summary suitable for CLI output.
    #[must_use]
    pub fn one_line_summary(&self) -> String {
        let homo_lumo = self
            .orbitals
            .as_ref()
            .map(|o| format!(" gap={:.3}eV", o.gap_ev))
            .unwrap_or_default();
        let dipole = self
            .dipole
            .as_ref()
            .map(|d| format!(" |mu|={:.3}D", d.magnitude_debye))
            .unwrap_or_default();
        format!(
            "{}/{} {} E={:.6}Ha cycles={}{}{} wall={:.2}s",
            self.functional,
            self.basis_set,
            self.molecule_id,
            self.energy_hartree,
            self.convergence.n_cycles,
            homo_lumo,
            dipole,
            self.timings.wall_seconds,
        )
    }

    /// Build a placeholder result for tests/fixtures.  `source_kind` is
    /// hard-coded to `placeholder-result` so signed artifacts can never be
    /// confused with a real SCF output.
    #[must_use]
    pub fn placeholder(request: &DftRequest) -> Self {
        Self {
            schema_tag: DFT_RESULT_TAG.to_string(),
            request_id: request.request_id.clone(),
            molecule_id: request.molecule.molecule_id.clone(),
            functional: request.method.functional.clone(),
            basis_set: request.method.basis_set.clone(),
            backend: format!("{:?}", request.method.backend),
            total_charge: request.total_charge,
            multiplicity: request.multiplicity,
            energy_hartree: 0.0,
            orbitals: None,
            dipole: None,
            convergence: DftConvergence {
                converged: false,
                n_cycles: 0,
                final_gradient_norm: None,
                scf_threshold: None,
            },
            timings: DftTimings {
                wall_seconds: 0.0,
                cpu_seconds: None,
            },
            requested_properties: request.requested_properties.clone(),
            provenance: DftResultProvenance {
                source_kind: "placeholder-result".to_string(),
                source_ref: "DftResult::placeholder".to_string(),
                host: None,
                pyscf_version: None,
                skala_version: None,
                dispersion: request.method.dispersion.clone(),
                notes: vec![
                    "Placeholder result; never anchor as a real on-chain DFT artifact".to_string(),
                ],
            },
        }
    }
}

#[derive(Debug)]
pub enum DftSkalaError {
    SchemaTagMismatch { expected: String, found: String },
    NotConverged,
    Artifact(ArtifactError),
    MissingEnv(String),
    Io(String),
    Command(String),
    Parse(String),
}

impl Display for DftSkalaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemaTagMismatch { expected, found } => write!(
                f,
                "DFT result schema_tag mismatch: expected {expected:?}, got {found:?}"
            ),
            Self::NotConverged => write!(f, "DFT SCF did not converge; refusing to sign"),
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
            Self::MissingEnv(name) => write!(f, "missing required environment variable {name}"),
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Command(error) => write!(f, "command error: {error}"),
            Self::Parse(error) => write!(f, "parse error: {error}"),
        }
    }
}

impl std::error::Error for DftSkalaError {}

impl From<ArtifactError> for DftSkalaError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

/// Sign a DFT result as a `chem.dft.result` artifact, parented to the request
/// artifact it answers.  Refuses to sign if the schema tag is wrong or the
/// SCF didn't converge — the goal is that every signed `chem.dft.result` in
/// the artifact graph corresponds to a real, converged SCF.
pub fn dft_result_artifact(
    result: &DftResult,
    request_artifact_id: ArtifactId,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, DftSkalaError> {
    if result.schema_tag != DFT_RESULT_TAG {
        return Err(DftSkalaError::SchemaTagMismatch {
            expected: DFT_RESULT_TAG.to_string(),
            found: result.schema_tag.clone(),
        });
    }
    if !result.convergence.converged {
        return Err(DftSkalaError::NotConverged);
    }
    ArtifactDraft {
        skill: SkillId(DFT_RESULT_SKILL.to_string()),
        agent,
        topic: format!(
            "DFT result {} ({}/{})",
            result.request_id, result.functional, result.basis_set
        ),
        input_fingerprint: format!(
            "dft-result:{}:{}:{:.6}",
            result.request_id, result.molecule_id, result.energy_hartree
        ),
        output_cid: None,
        parent_artifact_ids: vec![request_artifact_id],
        schema_tags: BTreeSet::from([SchemaTag(DFT_RESULT_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(result)?),
    }
    .seal(signer, created_at_unix)
    .map_err(DftSkalaError::Artifact)
}

#[cfg(feature = "live")]
mod live {
    use super::{DftResult, DftSkalaError, DftWorkerInput, DFT_RESULT_TAG, DFT_WORKER_COMMAND_ENV};
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    /// A whitespace-tokenized command line that invokes the duck-side DFT
    /// worker.  Read from `CHIMIACLAW_DFT_COMMAND`.  Multi-token wrappers like
    /// `ssh duck@olympus.local 'uv run --project ~/Documents/ChimiaDAO-QM/DFT
    /// python tools/dft_worker.py'` are supported.
    #[derive(Clone, Debug)]
    pub struct DftWorkerCommandConfig {
        program: PathBuf,
        program_args: Vec<String>,
    }

    impl DftWorkerCommandConfig {
        pub fn from_env() -> Result<Self, DftSkalaError> {
            let raw = std::env::var(DFT_WORKER_COMMAND_ENV)
                .map_err(|_| DftSkalaError::MissingEnv(DFT_WORKER_COMMAND_ENV.to_string()))?;
            let mut tokens = raw.split_whitespace();
            let program = tokens
                .next()
                .ok_or_else(|| DftSkalaError::MissingEnv(DFT_WORKER_COMMAND_ENV.to_string()))?;
            let program_args: Vec<String> = tokens.map(str::to_string).collect();
            Ok(Self {
                program: PathBuf::from(program),
                program_args,
            })
        }

        /// Pipe the `{request, molecule_adt}` wrapper as JSON on stdin, parse
        /// the response as a `DftResult`, validate its schema_tag.
        /// Convergence is checked at signing time, not here, so callers can
        /// still inspect failed attempts.
        pub fn invoke(&self, input: &DftWorkerInput) -> Result<DftResult, DftSkalaError> {
            let input_json = serde_json::to_vec(input)
                .map_err(|error| DftSkalaError::Parse(error.to_string()))?;
            let mut child = Command::new(&self.program)
                .args(&self.program_args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| DftSkalaError::Io(error.to_string()))?;
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| DftSkalaError::Io("could not open dft worker stdin".to_string()))?;
            stdin
                .write_all(&input_json)
                .map_err(|error| DftSkalaError::Io(error.to_string()))?;
            drop(child.stdin.take());
            let output = child
                .wait_with_output()
                .map_err(|error| DftSkalaError::Io(error.to_string()))?;
            if !output.status.success() {
                return Err(DftSkalaError::Command(
                    String::from_utf8_lossy(&output.stderr).into(),
                ));
            }
            let result: DftResult = serde_json::from_slice(&output.stdout)
                .map_err(|error| DftSkalaError::Parse(error.to_string()))?;
            if result.schema_tag != DFT_RESULT_TAG {
                return Err(DftSkalaError::SchemaTagMismatch {
                    expected: DFT_RESULT_TAG.to_string(),
                    found: result.schema_tag,
                });
            }
            Ok(result)
        }
    }
}

#[cfg(feature = "live")]
pub use live::DftWorkerCommandConfig;

#[cfg(test)]
mod tests {
    use super::*;
    use chimiaclaw_moladt::{
        demo_ferrocene_moladt, dft_request_artifact, molecule_artifact, DftBackend, DftJobKind,
        DftMethodSpec, DftMoleculeRef, DftRequest,
    };

    fn signer() -> ArtifactSigner {
        ArtifactSigner::from_seed([91; 32])
    }

    fn agent() -> AgentId {
        AgentId("dft.worker.chimiaclaw.eth".to_string())
    }

    fn fixture_request() -> (Artifact, DftRequest) {
        let molecule = demo_ferrocene_moladt();
        let mol_artifact =
            molecule_artifact(&molecule, agent(), &signer(), 1).expect("molecule artifact");
        let request = DftRequest {
            request_id: "REQ.DFT.SKALA.FERROCENE.001".to_string(),
            molecule: DftMoleculeRef::unbound(&molecule).with_artifact(&mol_artifact),
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
            worker_hint: Some(DFT_WORKER_COMMAND_ENV.to_string()),
        };
        let req_artifact =
            dft_request_artifact(&request, agent(), &signer(), 2).expect("request artifact");
        (req_artifact, request)
    }

    fn fixture_converged_result(request: &DftRequest) -> DftResult {
        DftResult {
            schema_tag: DFT_RESULT_TAG.to_string(),
            request_id: request.request_id.clone(),
            molecule_id: request.molecule.molecule_id.clone(),
            functional: request.method.functional.clone(),
            basis_set: request.method.basis_set.clone(),
            backend: format!("{:?}", request.method.backend),
            total_charge: request.total_charge,
            multiplicity: request.multiplicity,
            energy_hartree: -1648.123_456,
            orbitals: Some(DftOrbitalEnergies {
                homo_hartree: -0.214_5,
                lumo_hartree: -0.025_1,
                gap_hartree: 0.189_4,
                gap_ev: 5.154,
            }),
            dipole: Some(DftDipole::from_components(0.0, 0.0, 0.0)),
            convergence: DftConvergence {
                converged: true,
                n_cycles: 18,
                final_gradient_norm: Some(1.2e-7),
                scf_threshold: Some(1.0e-8),
            },
            timings: DftTimings {
                wall_seconds: 41.7,
                cpu_seconds: Some(165.2),
            },
            requested_properties: request.requested_properties.clone(),
            provenance: DftResultProvenance {
                source_kind: "pyscf-skala-1.1".to_string(),
                source_ref: "duck@olympus.local:pyscf-2.11.0+skala-1.1".to_string(),
                host: Some("duck@olympus.local".to_string()),
                pyscf_version: Some("2.11.0".to_string()),
                skala_version: Some("1.1".to_string()),
                dispersion: Some("dftd3".to_string()),
                notes: vec!["fixture test result; not from a real SCF".to_string()],
            },
        }
    }

    #[test]
    fn placeholder_is_not_consistent() {
        let (_, request) = fixture_request();
        let placeholder = DftResult::placeholder(&request);
        assert_eq!(placeholder.schema_tag, DFT_RESULT_TAG);
        assert!(!placeholder.is_consistent());
        assert_eq!(placeholder.provenance.source_kind, "placeholder-result");
    }

    #[test]
    fn dft_result_artifact_links_to_request_artifact() {
        let (req_artifact, request) = fixture_request();
        let result = fixture_converged_result(&request);
        assert!(result.is_consistent());
        let artifact = dft_result_artifact(&result, req_artifact.id.clone(), agent(), &signer(), 3)
            .expect("result artifact");
        artifact.verify().expect("artifact verifies");
        artifact
            .verify_payload_value(&result)
            .expect("payload binding");
        assert!(artifact.has_parent(&req_artifact.id));
        assert!(artifact
            .schema_tags
            .contains(&SchemaTag(DFT_RESULT_TAG.to_string())));
    }

    #[test]
    fn dft_result_artifact_refuses_unconverged() {
        let (req_artifact, request) = fixture_request();
        let mut result = fixture_converged_result(&request);
        result.convergence.converged = false;
        match dft_result_artifact(&result, req_artifact.id, agent(), &signer(), 3) {
            Err(DftSkalaError::NotConverged) => {}
            other => panic!("expected NotConverged, got {other:?}"),
        }
    }

    #[test]
    fn dft_result_artifact_refuses_wrong_schema_tag() {
        let (req_artifact, request) = fixture_request();
        let mut result = fixture_converged_result(&request);
        result.schema_tag = "chem.dft.something_else".to_string();
        match dft_result_artifact(&result, req_artifact.id, agent(), &signer(), 3) {
            Err(DftSkalaError::SchemaTagMismatch { expected, found }) => {
                assert_eq!(expected, DFT_RESULT_TAG);
                assert_eq!(found, "chem.dft.something_else");
            }
            other => panic!("expected SchemaTagMismatch, got {other:?}"),
        }
    }

    #[test]
    fn dft_result_round_trips_through_json() {
        let (_, request) = fixture_request();
        let result = fixture_converged_result(&request);
        let json = serde_json::to_string(&result).expect("serialize");
        let parsed: DftResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, result);
    }

    #[test]
    fn dft_result_one_line_summary_is_human_readable() {
        let (_, request) = fixture_request();
        let result = fixture_converged_result(&request);
        let summary = result.one_line_summary();
        assert!(summary.contains("skala-1.1/def2-tzvp"));
        assert!(summary.contains("E=-1648.123456Ha"));
        assert!(summary.contains("cycles=18"));
        assert!(summary.contains("gap=5.154eV"));
    }
}
