use serde_json::Value;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::process::Command;

#[test]
fn process_boundary_rejects_non_utf8_argv_without_panicking_or_echoing() {
    let output = Command::new(env!("CARGO_BIN_EXE_chimiaclaw-discovery"))
        .arg(OsString::from_vec(b"secret-\xff.json".to_vec()))
        .output()
        .expect("run discovery CLI");

    assert_eq!(output.status.code(), Some(64));
    let report: Value = serde_json::from_slice(&output.stdout).expect("stable JSON error report");
    assert_eq!(report["error"]["code"], "invalid-arguments");
    assert_eq!(report["error"]["at"], "$");
    assert_eq!(report["authority"]["human_decision"], false);
    assert_eq!(report["authority"]["mutation"], false);

    let combined = [output.stdout, output.stderr].concat();
    assert!(!combined.windows(6).any(|window| window == b"secret"));
    assert!(!combined.windows(5).any(|window| window == b"panic"));
    assert!(!combined.contains(&0xff));
}

#[test]
fn process_boundary_rejects_weak_key_forgery_without_exporting_dossier() {
    let mut artifact: Value =
        serde_json::from_str(include_str!("../fixtures/discovery-artifact.valid.json"))
            .expect("fixture");
    artifact["signing_public_key"] = Value::String(format!("01{}", "00".repeat(31)));
    artifact["signature"] = Value::String(format!("01{}", "00".repeat(63)));
    let path = std::env::temp_dir().join(format!("discovery-forgery-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&artifact).expect("serialize")).expect("write");
    let output = Command::new(env!("CARGO_BIN_EXE_chimiaclaw-discovery"))
        .arg("verify-export")
        .arg(&path)
        .arg("--json")
        .output()
        .expect("run CLI");
    std::fs::remove_file(&path).expect("cleanup");
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["error"]["code"], "artifact-integrity-failed");
    assert!(report.get("dossier").is_none());
    assert_eq!(report["authority"]["mutation"], false);
}
