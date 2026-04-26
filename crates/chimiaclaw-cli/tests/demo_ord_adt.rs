use serde_json::Value;
use std::process::Command;

#[test]
fn demo_ord_adt_runs_signed_translation_flow() {
    let output = Command::new(env!("CARGO_BIN_EXE_chimiaclaw-cli"))
        .arg("demo-ord-adt")
        .output()
        .expect("run demo-ord-adt");

    assert!(
        output.status.success(),
        "demo-ord-adt failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let report: Value = serde_json::from_str(&stdout).expect("demo-ord-adt emits json");

    assert_eq!(report["artifact_count"], 2);
    assert_eq!(report["ord_schema_tag"], "chem.ord.reaction");
    assert_eq!(report["adt_schema_tag"], "chem.adt.reaction");
    assert_eq!(
        report["adt_experiment"]["metadata"]["title"],
        "Suzuki coupling — biphenyl (hackathon v0)"
    );
    assert_eq!(
        report["adt_experiment"]["reaction"]["conditions"]["temperature_C"],
        80.0
    );
    assert_eq!(
        report["adt_experiment"]["samples"]
            .as_array()
            .unwrap()
            .len(),
        6
    );

    let root_id = report["root"]["id"].as_str().expect("root id");
    let children = report["children_of_root"]
        .as_array()
        .expect("children_of_root array");
    assert_eq!(children.len(), 1);
    assert!(contains_string(
        &children[0]["parent_artifact_ids"],
        root_id
    ));
    assert!(contains_string(
        &children[0]["schema_tags"],
        "chem.adt.reaction"
    ));
}

fn contains_string(value: &Value, expected: &str) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}
