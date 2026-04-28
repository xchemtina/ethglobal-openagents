use chimiaclaw_artifact::{
    ArtifactDraft, ArtifactId, ArtifactSigner, ArtifactStore, InMemoryArtifactStore, PayloadRef,
};
use chimiaclaw_market::demo_science_market;
use chimiaclaw_node::{NodeProfile, NodeRuntime, RunCycleReport};
use chimiaclaw_ord_adt::{
    adt_experiment_hash, demo_suzuki_ord_like, OrdToAdtSkill, OrdToAdtTranslator, ADT_REACTION_TAG,
    ORD_ADT_AGENT, ORD_REACTION_TAG,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use retroquoter::{
    demo_execution_request, demo_reagent_catalog, demo_route_proposal, ProcurementExecutor,
    RetroQuoter, RouteQuoteSkill, PLANNER_AGENT, PROCUREMENT_AGENT, ROUTE_PROPOSAL_TAG,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_NODE_AGENT: &str = "node.local.chimiaclaw.eth";

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
        [_, "world-model", ..] => match run_world_model() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("world-model failed: {error}");
                ExitCode::FAILURE
            }
        },
        [_, "science-market-demo", ..] => match run_science_market_demo() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("science-market-demo failed: {error}");
                ExitCode::FAILURE
            }
        },
        [_, "node", "seed-ord", rest @ ..] => match run_node_seed_ord(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("node seed-ord failed: {error}");
                ExitCode::FAILURE
            }
        },
        [_, "node", "seed-route", rest @ ..] => match run_node_seed_route(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("node seed-route failed: {error}");
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
        [_, "node", "run", rest @ ..] => match run_node_run(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("node run failed: {error}");
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

fn run_science_market_demo() -> Result<(), String> {
    let demo = demo_science_market();
    println!(
        "{}",
        serde_json::to_string_pretty(&demo)
            .map_err(|error| format!("serialize science market demo: {error}"))?
    );
    Ok(())
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
    println!("  chimiaclaw-cli world-model");
    println!("      Print the frontend-facing abstract lab network model as JSON.");
    println!("  chimiaclaw-cli science-market-demo");
    println!(
        "      Print deterministic signed ENS-shaped service transactions for DFT, retrosynthesis, and literature."
    );
    println!("  chimiaclaw-cli node seed-ord --store-dir <path>");
    println!("      Seed a file-backed store with a signed demo ORD reaction artifact.");
    println!("  chimiaclaw-cli node seed-route --store-dir <path>");
    println!("      Seed a file-backed store with a signed demo route proposal artifact.");
    println!("  chimiaclaw-cli node run-once --store-dir <path> [--agent <id>] [--profile-label <label>] [--skills all|ord-adt|retroquoter] [--created-at <unix>]");
    println!(
        "      Run one synchronous loop: scan store, invoke registered demo skills, persist children."
    );
    println!("  chimiaclaw-cli node run --store-dir <path> [--agent <id>] [--profile-label <label>] [--skills all|ord-adt|retroquoter] [--interval-ms <ms>] [--max-cycles <n>]");
    println!(
        "      Poll continuously as JSONL until Ctrl+C, or stop after --max-cycles for scripted demos."
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

fn parse_optional_u64(args: &[&str], key: &str) -> Result<Option<u64>, String> {
    parse_kv(args, key)
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid {key} value {value:?}: {error}"))
        })
        .transpose()
}

fn parse_u64_or(args: &[&str], key: &str, default: u64) -> Result<u64, String> {
    Ok(parse_optional_u64(args, key)?.unwrap_or(default))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn run_world_model() -> Result<(), String> {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../demo/world-model.json"))
            .map_err(|error| format!("invalid world model fixture: {error}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value)
            .map_err(|error| format!("serialize world model: {error}"))?
    );
    Ok(())
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
fn run_node_seed_route(args: &[&str]) -> Result<(), String> {
    let store_dir = PathBuf::from(require_kv(args, "--store-dir")?);
    let agent_id = parse_kv(args, "--agent").unwrap_or(PLANNER_AGENT);
    let label = parse_kv(args, "--profile-label").unwrap_or("chimiaclaw-dev");

    let signer = NodeProfile::dev_signer_from_seed_label(&format!("planner:{label}"));
    let proposal = demo_route_proposal();
    let payload = PayloadRef::inline_json(&proposal).map_err(|error| format!("{error:?}"))?;
    let route_artifact = ArtifactDraft {
        skill: SkillId("chem.retrosynth.aizynth.v1".to_string()),
        agent: AgentId(agent_id.to_string()),
        topic: "demo aspirin route proposal".to_string(),
        input_fingerprint: format!("smiles:{}", proposal.target_smiles),
        output_cid: Some("inline://retroquoter/route/aspirin-demo".to_string()),
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(ROUTE_PROPOSAL_TAG.to_string())]),
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
    let id = route_artifact.id.clone();
    match runtime.put_artifact(route_artifact) {
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
            "schema_tag": ROUTE_PROPOSAL_TAG,
        })
    );
    Ok(())
}

fn register_node_skills(runtime: &mut NodeRuntime, skills: &str) -> Result<(), String> {
    match skills {
        "all" => {
            runtime.register_skill(Box::new(OrdToAdtSkill::new()));
            runtime.register_skill(Box::new(RouteQuoteSkill::demo()));
            Ok(())
        }
        "ord-adt" => {
            runtime.register_skill(Box::new(OrdToAdtSkill::new()));
            Ok(())
        }
        "retroquoter" | "route-quote" => {
            runtime.register_skill(Box::new(RouteQuoteSkill::demo()));
            Ok(())
        }
        other => Err(format!(
            "unknown --skills value {other:?}; expected all, ord-adt, or retroquoter"
        )),
    }
}

fn run_node_run_once(args: &[&str]) -> Result<(), String> {
    let store_dir = PathBuf::from(require_kv(args, "--store-dir")?);
    let agent_id = parse_kv(args, "--agent").unwrap_or(DEFAULT_NODE_AGENT);
    let label = parse_kv(args, "--profile-label").unwrap_or("chimiaclaw-dev");
    let skills = parse_kv(args, "--skills").unwrap_or("all");
    let created_at_unix = parse_optional_u64(args, "--created-at")?.unwrap_or_else(unix_now);

    let signer = NodeProfile::dev_signer_from_seed_label(&format!("node:{label}"));
    let profile = NodeProfile {
        agent: AgentId(agent_id.to_string()),
        signer,
        store_dir,
    };
    let mut runtime = NodeRuntime::open(profile).map_err(|error| format!("{error:?}"))?;
    register_node_skills(&mut runtime, skills)?;
    let report = runtime
        .run_once(created_at_unix)
        .map_err(|error| format!("{error:?}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .unwrap_or_else(|error| format!("<failed to serialize report: {error}>"))
    );
    Ok(())
}

fn run_node_run(args: &[&str]) -> Result<(), String> {
    let store_dir = PathBuf::from(require_kv(args, "--store-dir")?);
    let agent_id = parse_kv(args, "--agent").unwrap_or(DEFAULT_NODE_AGENT);
    let label = parse_kv(args, "--profile-label").unwrap_or("chimiaclaw-dev");
    let skills = parse_kv(args, "--skills").unwrap_or("all");
    let interval_ms = parse_u64_or(args, "--interval-ms", 5_000)?;
    let max_cycles = parse_optional_u64(args, "--max-cycles")?;
    if max_cycles == Some(0) {
        return Ok(());
    }

    let signer = NodeProfile::dev_signer_from_seed_label(&format!("node:{label}"));
    let profile = NodeProfile {
        agent: AgentId(agent_id.to_string()),
        signer,
        store_dir,
    };
    let mut runtime = NodeRuntime::open(profile).map_err(|error| format!("{error:?}"))?;
    register_node_skills(&mut runtime, skills)?;

    let interval = Duration::from_millis(interval_ms);
    let mut cycle_index = 0;
    loop {
        let created_at_unix = unix_now();
        let report = runtime
            .run_once(created_at_unix)
            .map_err(|error| format!("{error:?}"))?;
        let cycle = RunCycleReport {
            cycle_index,
            created_at_unix,
            report,
        };
        println!(
            "{}",
            serde_json::to_string(&cycle)
                .unwrap_or_else(|error| format!("<failed to serialize cycle: {error}>"))
        );

        cycle_index += 1;
        if max_cycles.is_some_and(|limit| cycle_index >= limit) {
            break;
        }
        std::thread::sleep(interval);
    }
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
        demo_reagent_catalog(),
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
