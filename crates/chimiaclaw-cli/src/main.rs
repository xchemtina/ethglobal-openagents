use chimiaclaw_artifact::{
    ArtifactDraft, ArtifactId, ArtifactSigner, ArtifactStore, InMemoryArtifactStore, PayloadRef,
};
use chimiaclaw_node::{NodeProfile, NodeRuntime};
use chimiaclaw_ord_adt::{
    adt_experiment_hash, demo_suzuki_ord_like, OrdToAdtSkill, OrdToAdtTranslator, ADT_REACTION_TAG,
    ORD_ADT_AGENT, ORD_REACTION_TAG,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use retroquoter::{
    ProcurementExecutionRequest, ProcurementExecutor, ReagentCatalog, ReagentRequirement,
    ReagentRole, RetroQuoter, RouteProposal, RouteStep, SupplierOffer, PLANNER_AGENT,
    PROCUREMENT_AGENT, ROUTE_PROPOSAL_TAG,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    match argv.as_slice() {
        [_, "demo-dag", ..] => {
            run_demo_dag();
            ExitCode::SUCCESS
        }
        [_, "demo-ord-adt", ..] => {
            run_demo_ord_adt();
            ExitCode::SUCCESS
        }
        [_, "node", "seed-ord", rest @ ..] => match run_node_seed_ord(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("node seed-ord failed: {error}");
                ExitCode::FAILURE
            }
        },
        [_, "node", "run-once", rest @ ..] => match run_node_run_once(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("node run-once failed: {error}");
                ExitCode::FAILURE
            }
        },
        [_, "artifact", "inspect", rest @ ..] => match run_artifact_inspect(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("artifact inspect failed: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            print_help();
            ExitCode::SUCCESS
        }
    }
}

fn run_demo_ord_adt() {
    let importer_signer = ArtifactSigner::from_seed([50; 32]);
    let translator_signer = ArtifactSigner::from_seed([51; 32]);
    let ord = demo_suzuki_ord_like();
    let ord_payload = PayloadRef::inline_json(&ord).expect("ord payload");
    let ord_artifact = ArtifactDraft {
        skill: SkillId("chem.ord.import.v1".to_string()),
        agent: AgentId("ord.importer.eth".to_string()),
        topic: "demo ORD-like Suzuki reaction".to_string(),
        input_fingerprint: "ord-like:suzuki".to_string(),
        output_cid: Some("inline://ord-like/suzuki".to_string()),
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(ORD_REACTION_TAG.to_string())]),
        payload: Some(ord_payload),
    }
    .seal(&importer_signer, 1)
    .expect("seal ORD artifact");

    let translator = OrdToAdtTranslator::new(AgentId(ORD_ADT_AGENT.to_string()), translator_signer);
    let signed_adt = translator
        .translate_and_seal(&ord_artifact, &ord, 2)
        .expect("translate and seal ADT");

    let mut store = InMemoryArtifactStore::new();
    store.put(ord_artifact.clone()).expect("store ORD artifact");
    store
        .put(signed_adt.artifact.clone())
        .expect("store ADT artifact");

    let children = store.children_of(&ord_artifact.id).expect("children");
    let report = serde_json::json!({
        "ord_schema_tag": ORD_REACTION_TAG,
        "adt_schema_tag": ADT_REACTION_TAG,
        "adt_hash": adt_experiment_hash(&signed_adt.experiment).expect("adt hash"),
        "ord_like": ord,
        "adt_experiment": signed_adt.experiment,
        "root": ord_artifact,
        "children_of_root": children,
        "artifact_count": store.all().expect("all artifacts").len(),
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
}

fn print_help() {
    println!("chimiaclaw-cli");
    println!("usage:");
    println!("  chimiaclaw-cli demo-dag");
    println!("      Build a deterministic in-memory route -> quote -> procured DAG.");
    println!("  chimiaclaw-cli demo-ord-adt");
    println!("      Translate the demo ORD-like reaction into a signed ADT artifact.");
    println!("  chimiaclaw-cli node seed-ord --store-dir <path>");
    println!("      Seed a file-backed store with a signed demo ORD reaction artifact.");
    println!("  chimiaclaw-cli node run-once --store-dir <path> [--agent <id>] [--profile-label <label>]");
    println!(
        "      Run one synchronous loop: scan store, invoke ORD->ADT skill, persist children."
    );
    println!("  chimiaclaw-cli artifact inspect --store-dir <path> [--id <artifact-id>]");
    println!("      List all stored artifacts (and lineage) or inspect one by id.");
}

fn parse_kv<'a>(args: &'a [&'a str], key: &str) -> Option<&'a str> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == key && i + 1 < args.len() {
            return Some(args[i + 1]);
        }
        i += 1;
    }
    None
}

fn require_kv<'a>(args: &'a [&'a str], key: &str) -> Result<&'a str, String> {
    parse_kv(args, key).ok_or_else(|| format!("missing required argument: {key}"))
}

fn run_node_seed_ord(args: &[&str]) -> Result<(), String> {
    let store_dir = PathBuf::from(require_kv(args, "--store-dir")?);
    let agent_id = parse_kv(args, "--agent").unwrap_or("ord.importer.eth");
    let label = parse_kv(args, "--profile-label").unwrap_or("chimiaclaw-dev");

    let signer = NodeProfile::dev_signer_from_seed_label(&format!("importer:{label}"));
    let ord = demo_suzuki_ord_like();
    let payload = PayloadRef::inline_json(&ord).map_err(|error| format!("{error:?}"))?;
    let ord_artifact = ArtifactDraft {
        skill: SkillId("chem.ord.import.v1".to_string()),
        agent: AgentId(agent_id.to_string()),
        topic: "demo ORD-like Suzuki reaction".to_string(),
        input_fingerprint: "ord-like:suzuki".to_string(),
        output_cid: Some("inline://ord-like/suzuki".to_string()),
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(ORD_REACTION_TAG.to_string())]),
        payload: Some(payload),
    }
    .seal(&signer, 1)
    .map_err(|error| format!("seal: {error:?}"))?;

    let profile = NodeProfile {
        agent: AgentId(agent_id.to_string()),
        signer,
        store_dir,
    };
    let mut runtime = NodeRuntime::open(profile).map_err(|error| format!("{error:?}"))?;
    let id = ord_artifact.id.clone();
    match runtime.put_artifact(ord_artifact) {
        Ok(())
        | Err(chimiaclaw_node::NodeError::Store(
            chimiaclaw_artifact::ArtifactStoreError::Conflict(_),
        )) => {}
        Err(other) => return Err(format!("put: {other:?}")),
    }
    println!(
        "{}",
        serde_json::json!({
            "seeded_artifact_id": id.0,
            "schema_tag": ORD_REACTION_TAG,
        })
    );
    Ok(())
}

fn run_node_run_once(args: &[&str]) -> Result<(), String> {
    let store_dir = PathBuf::from(require_kv(args, "--store-dir")?);
    let agent_id = parse_kv(args, "--agent").unwrap_or(ORD_ADT_AGENT);
    let label = parse_kv(args, "--profile-label").unwrap_or("chimiaclaw-dev");

    let signer = NodeProfile::dev_signer_from_seed_label(&format!("node:{label}"));
    let profile = NodeProfile {
        agent: AgentId(agent_id.to_string()),
        signer,
        store_dir,
    };
    let mut runtime = NodeRuntime::open(profile).map_err(|error| format!("{error:?}"))?;
    runtime.register_skill(Box::new(OrdToAdtSkill::new()));
    let report = runtime.run_once(2).map_err(|error| format!("{error:?}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .unwrap_or_else(|error| format!("<failed to serialize report: {error}>"))
    );
    Ok(())
}

fn run_artifact_inspect(args: &[&str]) -> Result<(), String> {
    let store_dir = PathBuf::from(require_kv(args, "--store-dir")?);
    let id = parse_kv(args, "--id");
    let signer = NodeProfile::dev_signer_from_seed_label("inspect");
    let profile = NodeProfile {
        agent: AgentId("inspector.local.eth".to_string()),
        signer,
        store_dir,
    };
    let runtime = NodeRuntime::open(profile).map_err(|error| format!("{error:?}"))?;
    if let Some(id) = id {
        let artifact = runtime
            .get_artifact(&ArtifactId(id.to_string()))
            .map_err(|error| format!("{error:?}"))?;
        match artifact {
            Some(artifact) => {
                let value = serde_json::to_value(&artifact)
                    .map_err(|error| format!("serialize: {error}"))?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value)
                        .unwrap_or_else(|error| format!("<failed to serialize: {error}>"))
                );
            }
            None => return Err(format!("no artifact with id {id} in store")),
        }
    } else {
        let all = runtime
            .all_artifacts()
            .map_err(|error| format!("{error:?}"))?;
        let summary: Vec<_> = all
            .iter()
            .map(|artifact| {
                serde_json::json!({
                    "id": artifact.id.0,
                    "skill": artifact.skill.0,
                    "agent": artifact.agent.0,
                    "schema_tags": artifact.schema_tags.iter().map(|tag| tag.0.clone()).collect::<Vec<_>>(),
                    "parents": artifact.parent_artifact_ids.iter().map(|id| id.0.clone()).collect::<Vec<_>>(),
                    "payload_hash": artifact.payload.as_ref().map(|p| p.hash.clone()),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&summary)
                .unwrap_or_else(|error| format!("<failed to serialize summary: {error}>"))
        );
    }
    Ok(())
}

fn run_demo_dag() {
    let planner_signer = ArtifactSigner::from_seed([41; 32]);
    let procurement_signer = ArtifactSigner::from_seed([42; 32]);
    let execution_signer = ArtifactSigner::from_seed([43; 32]);
    let proposal = demo_route_proposal();
    let route_payload = PayloadRef::inline_json(&proposal).expect("route payload");
    let route = ArtifactDraft {
        skill: SkillId("chem.retrosynth.aizynth.v1".to_string()),
        agent: AgentId(PLANNER_AGENT.to_string()),
        topic: "retrosynthesis route for demo target".to_string(),
        input_fingerprint: "smiles:CC(=O)Oc1ccccc1C(=O)O".to_string(),
        output_cid: Some("zg://retroquoter/route/001".to_string()),
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(ROUTE_PROPOSAL_TAG.to_string())]),
        payload: Some(route_payload),
    }
    .seal(&planner_signer, 1)
    .expect("seal route artifact");

    let mut store = InMemoryArtifactStore::new();
    store.put(route.clone()).expect("store route");

    let quoter = RetroQuoter::new(
        AgentId(PROCUREMENT_AGENT.to_string()),
        procurement_signer,
        demo_catalog(),
    );
    let signed_quote = quoter
        .quote_route_from_store(&store, &route.id, &proposal, 2)
        .expect("generate signed quote");
    store
        .put(signed_quote.artifact.clone())
        .expect("store quote artifact");
    let executor =
        ProcurementExecutor::new(AgentId(PROCUREMENT_AGENT.to_string()), execution_signer);
    let procured = executor
        .execute_from_store(
            &store,
            &signed_quote.artifact.id,
            &signed_quote.quote,
            &demo_execution_request(),
            3,
        )
        .expect("execute procurement");
    store
        .put(procured.artifact.clone())
        .expect("store procured artifact");

    let children = store.children_of(&route.id).expect("children");
    let quote_children = store
        .children_of(&signed_quote.artifact.id)
        .expect("quote children");
    let report = serde_json::json!({
        "planner_public_key": route.signing_public_key,
        "procurement_public_key": signed_quote.artifact.signing_public_key,
        "execution_public_key": procured.artifact.signing_public_key,
        "quote": signed_quote.quote,
        "procurement_receipt": procured.receipt,
        "root": route,
        "children_of_root": children,
        "children_of_quote": quote_children,
        "artifact_count": store.all().expect("all artifacts").len(),
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
}

fn demo_route_proposal() -> RouteProposal {
    RouteProposal {
        route_id: "route-aspirin-demo".to_string(),
        target_smiles: "CC(=O)Oc1ccccc1C(=O)O".to_string(),
        target_scale_milligrams: 5_000,
        steps: vec![RouteStep {
            step_id: "step-1".to_string(),
            description: "Acetylate salicylic acid with acetic anhydride".to_string(),
            requirements: vec![
                ReagentRequirement {
                    reagent_id: "salicylic-acid".to_string(),
                    display_name: "Salicylic acid".to_string(),
                    quantity_milligrams: 4_000,
                    role: ReagentRole::StartingMaterial,
                },
                ReagentRequirement {
                    reagent_id: "acetic-anhydride".to_string(),
                    display_name: "Acetic anhydride".to_string(),
                    quantity_milligrams: 6_000,
                    role: ReagentRole::Reagent,
                },
            ],
        }],
    }
}

fn demo_execution_request() -> ProcurementExecutionRequest {
    ProcurementExecutionRequest {
        buyer_agent: AgentId("buyer.chimiaclaw.eth".to_string()),
        payment_reference: "uniswap-swap-demo-001".to_string(),
        destination_profile_id: "sofia-lab-default".to_string(),
    }
}

fn demo_catalog() -> ReagentCatalog {
    ReagentCatalog::new()
        .with_offer(
            "salicylic-acid",
            SupplierOffer {
                supplier: "DemoChem".to_string(),
                sku: "SAL-001".to_string(),
                unit_price_cents_per_gram: 120,
                available_grams: 100,
            },
        )
        .with_offer(
            "acetic-anhydride",
            SupplierOffer {
                supplier: "DemoChem".to_string(),
                sku: "ACE-002".to_string(),
                unit_price_cents_per_gram: 80,
                available_grams: 250,
            },
        )
}
