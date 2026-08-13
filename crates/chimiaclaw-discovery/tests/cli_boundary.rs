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
