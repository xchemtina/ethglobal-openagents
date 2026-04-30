//! External SMILES → [`MoleculeAdt`] worker boundary.
//!
//! ChimiaClaw does not embed a SMILES parser or a 3D embedder. Instead, this
//! module shells out to a user-managed worker (typically a uv-managed RDKit
//! script under `skills/scienceclaw-port/workers/cheminformatics`) whose
//! contract is:
//!
//! - the worker is invoked through the `CHIMIACLAW_SMILES_TO_MOLADT_COMMAND`
//!   environment variable (whitespace-separated argv); if unset the function
//!   returns [`WorkerError::NotConfigured`];
//! - the worker receives the SMILES on stdin (UTF-8, no trailing newline
//!   required);
//! - on success it writes a JSON document on stdout that deserializes into a
//!   [`MoleculeAdt`];
//! - on failure it exits with a non-zero status and writes a human-readable
//!   message to stderr.
//!
//! The returned molecule is validated and its `provenance.source_kind` is
//! preserved so callers can distinguish a curated entry from an
//! RDKit-MMFF-generated geometry without inspecting the worker output.

use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::{MolAdtError, MoleculeAdt};

/// Environment variable that points at the SMILES worker command line.
pub const SMILES_WORKER_ENV: &str = "CHIMIACLAW_SMILES_TO_MOLADT_COMMAND";

/// Errors that arise specifically from the worker boundary.
#[derive(Debug)]
pub enum WorkerError {
    /// `CHIMIACLAW_SMILES_TO_MOLADT_COMMAND` is unset or empty.
    NotConfigured,
    /// The configured command could not be spawned.
    Spawn(String),
    /// Stdin could not be opened or written.
    Stdin(String),
    /// The worker exited non-zero; the captured stderr is included.
    NonZeroExit {
        status_code: Option<i32>,
        stderr: String,
    },
    /// Stdout was not valid UTF-8.
    NonUtf8Output(String),
    /// Stdout was not a valid `MoleculeAdt` JSON document.
    Json(String),
    /// The decoded molecule failed `MoleculeAdt::validate`.
    InvalidMolecule(MolAdtError),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(
                f,
                "{SMILES_WORKER_ENV} is not set; cannot resolve SMILES via worker"
            ),
            Self::Spawn(message) => write!(f, "spawn worker: {message}"),
            Self::Stdin(message) => write!(f, "write SMILES to worker stdin: {message}"),
            Self::NonZeroExit {
                status_code,
                stderr,
            } => write!(
                f,
                "worker exited with status {:?}: {}",
                status_code,
                stderr.trim()
            ),
            Self::NonUtf8Output(message) => write!(f, "worker stdout not utf-8: {message}"),
            Self::Json(message) => write!(f, "worker stdout not valid MoleculeAdt JSON: {message}"),
            Self::InvalidMolecule(error) => {
                write!(f, "worker emitted invalid MoleculeAdt: {error}")
            }
        }
    }
}

impl std::error::Error for WorkerError {}

impl From<WorkerError> for MolAdtError {
    fn from(value: WorkerError) -> Self {
        match value {
            WorkerError::InvalidMolecule(error) => error,
            other => MolAdtError::Worker(other.to_string()),
        }
    }
}

/// Try to resolve `smiles` to a `MoleculeAdt` via the curated library first,
/// falling back to the configured external worker. Returns `None` only when
/// neither path produced a molecule.
pub fn resolve_with_worker(smiles: &str) -> Result<Option<MoleculeAdt>, MolAdtError> {
    if let Some(molecule) = crate::library::resolve_smiles(smiles) {
        return Ok(Some(molecule));
    }
    if crate::library::is_known_unsafe_for_dft(smiles) {
        return Ok(None);
    }
    match invoke_worker(smiles) {
        Ok(molecule) => Ok(Some(molecule)),
        Err(WorkerError::NotConfigured) => Ok(None),
        Err(other) => Err(MolAdtError::from(other)),
    }
}

/// Invoke the configured worker without consulting the curated library.
pub fn invoke_worker(smiles: &str) -> Result<MoleculeAdt, WorkerError> {
    let command_line = std::env::var(SMILES_WORKER_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or(WorkerError::NotConfigured)?;
    let mut tokens = command_line.split_whitespace();
    let program = tokens.next().ok_or(WorkerError::NotConfigured)?;
    let args: Vec<&str> = tokens.collect();
    invoke_worker_command(program, &args, smiles)
}

/// Lower-level entry point used by tests; runs `program` with `args` and
/// pipes `smiles` through stdin.
pub fn invoke_worker_command<S: AsRef<OsStr>>(
    program: S,
    args: &[&str],
    smiles: &str,
) -> Result<MoleculeAdt, WorkerError> {
    let mut child = Command::new(program.as_ref())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| WorkerError::Spawn(error.to_string()))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| WorkerError::Stdin("worker stdin not available".to_string()))?;
        stdin
            .write_all(smiles.trim().as_bytes())
            .map_err(|error| WorkerError::Stdin(error.to_string()))?;
        if !smiles.ends_with('\n') {
            let _ = stdin.write_all(b"\n");
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|error| WorkerError::Spawn(error.to_string()))?;
    if !output.status.success() {
        return Err(WorkerError::NonZeroExit {
            status_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| WorkerError::NonUtf8Output(error.to_string()))?;
    let molecule: MoleculeAdt = serde_json::from_str(stdout.trim())
        .map_err(|error| WorkerError::Json(error.to_string()))?;
    molecule.validate().map_err(WorkerError::InvalidMolecule)?;
    Ok(molecule)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_env_returns_not_configured() {
        // SAFETY: tests in this crate run single-threaded by default; we
        // restore the previous value after the test exits.
        let previous = std::env::var(SMILES_WORKER_ENV).ok();
        std::env::remove_var(SMILES_WORKER_ENV);
        let err = invoke_worker("Cc1ccccc1").expect_err("worker should not be configured");
        assert!(matches!(err, WorkerError::NotConfigured));
        if let Some(previous) = previous {
            std::env::set_var(SMILES_WORKER_ENV, previous);
        }
    }

    #[test]
    fn falls_back_to_curated_library_first() {
        let result = resolve_with_worker("Cc1ccccc1").expect("toluene resolves");
        let molecule = result.expect("library hit");
        assert_eq!(molecule.formula(), "C7H8");
    }

    #[test]
    fn refuses_to_invoke_worker_for_unsafe_smiles() {
        let result =
            resolve_with_worker("P(c1ccccc1)(c1ccccc1)c1ccccc1.Pd").expect("must not error");
        assert!(result.is_none());
    }
}
