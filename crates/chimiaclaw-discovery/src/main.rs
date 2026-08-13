use chimiaclaw_discovery::{verify_export_bytes, DiscoveryError, MAX_DOSSIER_BYTES};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

const USAGE: &str = "usage: chimiaclaw-discovery verify-export ARTIFACT.json --json";
const MAX_ARTIFACT_BYTES: usize = MAX_DOSSIER_BYTES * 3;

#[derive(Serialize)]
struct ErrorReport<'a> {
    schema: &'a str,
    operation: &'a str,
    ok: bool,
    report_only: bool,
    requires_human_decision: bool,
    authority: Authority,
    error: ErrorItem<'a>,
}

#[derive(Serialize)]
#[allow(clippy::struct_excessive_bools)] // Closed evidence envelope; each denied capability is explicit.
struct Authority {
    database_write: bool,
    dispatch: bool,
    execution: bool,
    human_decision: bool,
    mutation: bool,
    publication: bool,
}

#[derive(Serialize)]
struct ErrorItem<'a> {
    code: &'a str,
    at: &'a str,
}

fn error_report(error: &DiscoveryError) -> String {
    serde_json::to_string(&ErrorReport {
        schema: "openagents.discovery-export-report.v1",
        operation: "verify-export",
        ok: false,
        report_only: true,
        requires_human_decision: true,
        authority: Authority {
            database_write: false,
            dispatch: false,
            execution: false,
            human_decision: false,
            mutation: false,
            publication: false,
        },
        error: ErrorItem {
            code: error.code,
            at: &error.at,
        },
    })
    .unwrap_or_else(|_| {
        "{\"error\":{\"at\":\"$\",\"code\":\"error-report-failed\"},\"ok\":false}".to_owned()
    })
}

fn read_bounded(path: &str) -> Result<Vec<u8>, DiscoveryError> {
    if !path.to_ascii_lowercase().ends_with(".json") {
        return Err(DiscoveryError {
            code: "unsupported-input-format",
            at: "$".to_owned(),
        });
    }
    let path = Path::new(path);
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| DiscoveryError {
            code: "artifact-input-unavailable",
            at: "$".to_owned(),
        })?;
    let metadata = file.metadata().map_err(|_| DiscoveryError {
        code: "artifact-input-unavailable",
        at: "$".to_owned(),
    })?;
    if !metadata.file_type().is_file() {
        return Err(DiscoveryError {
            code: "artifact-input-not-regular-file",
            at: "$".to_owned(),
        });
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_ARTIFACT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| DiscoveryError {
            code: "artifact-input-read-failed",
            at: "$".to_owned(),
        })?;
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(DiscoveryError {
            code: "artifact-input-too-large",
            at: "$".to_owned(),
        });
    }
    Ok(bytes)
}

fn run(arguments: &[String]) -> Result<Vec<u8>, DiscoveryError> {
    if arguments.len() != 3 || arguments[0] != "verify-export" || arguments[2] != "--json" {
        return Err(DiscoveryError {
            code: "invalid-arguments",
            at: "$".to_owned(),
        });
    }
    verify_export_bytes(&read_bounded(&arguments[1])?)
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(bytes) => {
            let mut stdout = std::io::stdout().lock();
            if stdout
                .write_all(&bytes)
                .and_then(|()| stdout.write_all(b"\n"))
                .and_then(|()| stdout.flush())
                .is_err()
            {
                std::process::exit(1);
            }
        }
        Err(error) => {
            println!("{}", error_report(&error));
            eprintln!("{USAGE}");
            std::process::exit(if error.code == "invalid-arguments" {
                64
            } else {
                1
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TempPaths(Vec<PathBuf>);

    impl TempPaths {
        fn new(names: &[&str]) -> Self {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let paths = names
                .iter()
                .map(|name| {
                    std::env::temp_dir().join(format!(
                        "chimiaclaw-discovery-{}-{sequence}-{name}",
                        std::process::id()
                    ))
                })
                .collect();
            Self(paths)
        }
    }

    impl Drop for TempPaths {
        fn drop(&mut self) {
            for path in &self.0 {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    #[test]
    fn invalid_arguments_are_redacted_and_authority_free() {
        let error = run(&["/private/secret".to_owned()]).expect_err("invalid");
        let report = error_report(&error);
        assert!(!report.contains("/private/secret"));
        assert!(report.contains("\"human_decision\":false"));
        assert!(report.contains("\"mutation\":false"));
    }

    #[test]
    fn bounded_reader_rejects_symlinks_without_exposing_target() {
        let paths = TempPaths::new(&["target.json", "link.json"]);
        std::fs::write(&paths.0[0], b"{}").expect("write target");
        std::os::unix::fs::symlink(&paths.0[0], &paths.0[1]).expect("create symlink");

        let error = read_bounded(&paths.0[1].to_string_lossy()).expect_err("reject symlink");
        assert_eq!(error.code, "artifact-input-unavailable");
        assert_eq!(error.at, "$");
        assert!(!error_report(&error).contains(paths.0[0].to_string_lossy().as_ref()));
    }

    #[test]
    fn bounded_reader_accepts_exact_cap_and_rejects_plus_one() {
        let paths = TempPaths::new(&["exact.json", "oversize.json"]);
        std::fs::write(&paths.0[0], vec![b' '; MAX_ARTIFACT_BYTES]).expect("write exact file");
        assert_eq!(
            read_bounded(&paths.0[0].to_string_lossy())
                .expect("exact artifact byte cap")
                .len(),
            MAX_ARTIFACT_BYTES
        );

        std::fs::write(&paths.0[1], vec![b' '; MAX_ARTIFACT_BYTES + 1])
            .expect("write oversized file");
        let error =
            read_bounded(&paths.0[1].to_string_lossy()).expect_err("one byte over artifact cap");
        assert_eq!(error.code, "artifact-input-too-large");
        assert_eq!(error.at, "$");
    }

    #[test]
    fn bounded_reader_rejects_non_json_suffix_before_opening() {
        let path = std::env::temp_dir().join("chimiaclaw-discovery-does-not-exist.txt");
        let error = read_bounded(&path.to_string_lossy()).expect_err("unsupported suffix");
        assert_eq!(error.code, "unsupported-input-format");
    }
}
