use serde_json::Value;
use std::process::Command;

#[test]
fn demo_dag_runs_route_quote_procurement_flow() {
    let output = Command::new(env!("CARGO_BIN_EXE_chimiaclaw-cli"))
        .arg("demo-dag")
        .output()
        .expect("run demo-dag");

    assert!(
        output.status.success(),
        "demo-dag failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    let report: Value = serde_json::from_str(&stdout).expect("demo-dag emits json");

    assert_eq!(report["artifact_count"], 3);
    assert_eq!(report["quote"]["total_cents"], 1_104);
    assert_eq!(
        report["procurement_receipt"]["total_cents"],
        report["quote"]["total_cents"]
    );
    assert_eq!(report["procurement_receipt"]["state"], "Procured");

    let root_id = report["root"]["id"].as_str().expect("root id");
    let root_children = report["children_of_root"]
        .as_array()
        .expect("children_of_root array");
    assert_eq!(root_children.len(), 1);
    assert!(contains_string(
        &root_children[0]["parent_artifact_ids"],
        root_id
    ));
    assert!(contains_string(
        &root_children[0]["schema_tags"],
        "chem.procurement.route_quote"
    ));

    let quote_artifact_id = root_children[0]["id"].as_str().expect("quote artifact id");
    let quote_children = report["children_of_quote"]
        .as_array()
        .expect("children_of_quote array");
    assert_eq!(quote_children.len(), 1);
    assert!(contains_string(
        &quote_children[0]["parent_artifact_ids"],
        quote_artifact_id
    ));
    assert!(contains_string(
        &quote_children[0]["schema_tags"],
        "chem.procurement.procured"
    ));
}

fn contains_string(value: &Value, expected: &str) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}
