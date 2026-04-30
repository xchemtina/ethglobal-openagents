use chimiaclaw_artifact::{
    ArtifactDraft, ArtifactId, ArtifactSigner, ArtifactStore, InMemoryArtifactStore, PayloadRef,
};
use chimiaclaw_market::demo_science_market;
use chimiaclaw_moladt::{
    demo_ferrocene_moladt, dft_request_artifact, library, molecule_artifact, render,
    worker as moladt_worker, DftBackend, DftJobKind, DftMethodSpec, DftMoleculeRef, DftRequest,
    MoleculeAdt,
};
use chimiaclaw_node::{NodeProfile, NodeRuntime, RunCycleReport};
use chimiaclaw_ord_adt::{
    adt_experiment_hash, demo_suzuki_ord_like, translate_reaction, OrdLikeReaction, OrdToAdtSkill,
    OrdToAdtTranslator, ADT_REACTION_TAG, ORD_ADT_AGENT, ORD_REACTION_TAG,
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
        [_, "moladt-dft-demo", ..] => match run_moladt_dft_demo() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("moladt-dft-demo failed: {error}");
                ExitCode::FAILURE
            }
        },
        [_, "ord-moladt-demo", rest @ ..] => match run_ord_moladt_demo(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("ord-moladt-demo failed: {error}");
                ExitCode::FAILURE
            }
        },
        [_, "moladt-render", rest @ ..] => match run_moladt_render(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("moladt-render failed: {error}");
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
        #[cfg(feature = "live-sponsors")]
        [_, "live", "ens-verify", rest @ ..] => match run_live_ens_verify(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("live ens-verify failed: {error}");
                ExitCode::FAILURE
            }
        },
        #[cfg(feature = "live-sponsors")]
        [_, "live", "ens-publish", rest @ ..] => match run_live_ens_publish(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("live ens-publish failed: {error}");
                ExitCode::FAILURE
            }
        },
        #[cfg(feature = "live-sponsors")]
        [_, "live", "zerog-anchor", rest @ ..] => match run_live_zerog_anchor(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("live zerog-anchor failed: {error}");
                ExitCode::FAILURE
            }
        },
        #[cfg(feature = "live-sponsors")]
        [_, "live", "keeperhub-schedule", rest @ ..] => match run_live_keeperhub_schedule(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("live keeperhub-schedule failed: {error}");
                ExitCode::FAILURE
            }
        },
        #[cfg(feature = "live-sponsors")]
        [_, "live", "keeperhub-status", rest @ ..] => match run_live_keeperhub_status(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("live keeperhub-status failed: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            print_help();
            ExitCode::SUCCESS
        }
    }
}

#[cfg(feature = "live-sponsors")]
fn parse_all_kv<'a>(args: &'a [&'a str], key: &str) -> Vec<&'a str> {
    let mut values = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == key && i + 1 < args.len() {
            values.push(args[i + 1]);
            i += 1;
        }
        i += 1;
    }
    values
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

fn run_ord_moladt_demo(args: &[&str]) -> Result<(), String> {
    let ord = if let Some(path) = parse_kv(args, "--ord-json") {
        let body = std::fs::read_to_string(path)
            .map_err(|error| format!("read ORD-like JSON {path}: {error}"))?;
        OrdLikeReaction::from_json_str(&body)
            .map_err(|error| format!("parse ORD-like JSON: {error:?}"))?
    } else if let Some(path) = parse_kv(args, "--official-ord-json") {
        let body = std::fs::read_to_string(path)
            .map_err(|error| format!("read official ORD JSON {path}: {error}"))?;
        OrdLikeReaction::from_official_ord_json_str(&body)
            .map_err(|error| format!("parse official ORD JSON: {error:?}"))?
    } else {
        demo_suzuki_ord_like()
    };
    let translation = translate_reaction(&ord);
    let agent = AgentId("operator.chimiaclaw.eth".to_string());
    let signer = NodeProfile::dev_signer_from_seed_label("ord-moladt-demo");
    let output_dir = parse_kv(args, "--output-dir").map(PathBuf::from);
    if let Some(dir) = output_dir.as_ref() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("create output dir {}: {error}", dir.display()))?;
    }
    let mut artifacts = Vec::new();
    for (index, entry) in translation.resolved.iter().enumerate() {
        let created_at_unix = 1_u64 + index as u64;
        let artifact = molecule_artifact(&entry.molecule, agent.clone(), &signer, created_at_unix)
            .map_err(|error| format!("sign molecule artifact for {}: {error}", entry.smiles))?;
        let mut entry_record = serde_json::json!({
            "label": entry.label,
            "smiles": entry.smiles,
            "role": entry.role,
            "roles_in_reaction": entry.roles_in_reaction,
            "molecule_id": entry.molecule.molecule_id,
            "formula": entry.molecule.formula(),
            "artifact_id": artifact.id.0.clone(),
            "payload_hash": artifact.payload.as_ref().map(|payload| payload.hash.clone()),
            "artifact": artifact,
        });
        if let Some(dir) = output_dir.as_ref() {
            let stem = sanitize_filename_stem(&entry.molecule.molecule_id);
            let xyz_path = dir.join(format!("{stem}.xyz"));
            let svg_path = dir.join(format!("{stem}.svg"));
            entry.molecule.write_xyz_to(&xyz_path).map_err(|error| {
                format!("write xyz for {}: {error}", entry.molecule.molecule_id)
            })?;
            render::write_svg_to(
                &entry.molecule,
                &render::SvgRenderOptions::default(),
                &svg_path,
            )
            .map_err(|error| format!("write svg for {}: {error}", entry.molecule.molecule_id))?;
            if let Some(map) = entry_record.as_object_mut() {
                map.insert(
                    "xyz_path".to_string(),
                    serde_json::Value::String(xyz_path.display().to_string()),
                );
                map.insert(
                    "svg_path".to_string(),
                    serde_json::Value::String(svg_path.display().to_string()),
                );
            }
        }
        artifacts.push(entry_record);
    }
    let report = serde_json::json!({
        "reaction": translation.reaction_name,
        "resolved_count": translation.resolved.len(),
        "skipped": translation.skipped,
        "resolved": artifacts,
        "dft_ready": translation.dft_ready(),
        "output_dir": output_dir.as_ref().map(|dir| dir.display().to_string()),
        "note": "Resolved entries carry curated schematic geometries; a live DFT worker must run a geometry pass before trusting energies.",
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("serialize ord moladt demo: {error}"))?
    );
    Ok(())
}

fn run_moladt_render(args: &[&str]) -> Result<(), String> {
    let library_name = parse_kv(args, "--library");
    let smiles = parse_kv(args, "--smiles");
    let xyz_path = parse_kv(args, "--xyz").map(PathBuf::from);
    let svg_path = parse_kv(args, "--svg").map(PathBuf::from);
    let allow_worker = !args.iter().any(|arg| *arg == "--no-worker");
    let molecule = match (library_name, smiles) {
        (Some(name), None) => resolve_library_by_name(name).ok_or_else(|| {
            format!("unknown library molecule {name:?}; try water/ammonia/methanol/ethanol/acetic-acid/benzene/toluene/bromobenzene/phenylboronic-acid/biphenyl/ferrocene")
        })?,
        (None, Some(smiles)) => {
            if let Some(molecule) = library::resolve_smiles(smiles) {
                molecule
            } else if library::is_known_unsafe_for_dft(smiles) {
                return Err(format!(
                    "SMILES {smiles:?} is flagged unsafe-for-direct-DFT (multi-component or metal complex); refusing to render"
                ));
            } else if allow_worker {
                match moladt_worker::resolve_with_worker(smiles) {
                    Ok(Some(molecule)) => molecule,
                    Ok(None) => {
                        return Err(format!(
                            "SMILES {smiles:?} is not in the curated library and {} is not configured",
                            moladt_worker::SMILES_WORKER_ENV
                        ));
                    }
                    Err(error) => return Err(format!("smiles worker failed: {error}")),
                }
            } else {
                return Err(format!(
                    "SMILES {smiles:?} is not in the curated library (--no-worker disabled the external worker)"
                ));
            }
        }
        (Some(_), Some(_)) => {
            return Err("--library and --smiles are mutually exclusive".to_string());
        }
        (None, None) => demo_ferrocene_moladt(),
    };
    if let Some(path) = xyz_path.as_ref() {
        molecule
            .write_xyz_to(path)
            .map_err(|error| format!("write xyz {}: {error}", path.display()))?;
    }
    if let Some(path) = svg_path.as_ref() {
        render::write_svg_to(&molecule, &render::SvgRenderOptions::default(), path)
            .map_err(|error| format!("write svg {}: {error}", path.display()))?;
    }
    let report = serde_json::json!({
        "molecule_id": molecule.molecule_id,
        "name": molecule.name,
        "formula": molecule.formula(),
        "source_kind": molecule.provenance.source_kind,
        "atom_count": molecule.atoms.len(),
        "bond_count": molecule.local_bonds.len(),
        "projections": molecule.projections,
        "xyz_path": xyz_path.as_ref().map(|path| path.display().to_string()),
        "svg_path": svg_path.as_ref().map(|path| path.display().to_string()),
        "xyz": molecule.to_xyz().unwrap_or_default(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("serialize moladt render report: {error}"))?
    );
    Ok(())
}

fn resolve_library_by_name(name: &str) -> Option<MoleculeAdt> {
    match name.to_ascii_lowercase().replace('_', "-").as_str() {
        "water" | "h2o" => Some(library::water()),
        "ammonia" | "nh3" => Some(library::ammonia()),
        "methanol" | "methyl-alcohol" => Some(library::methanol()),
        "ethanol" | "ethyl-alcohol" => Some(library::ethanol()),
        "acetic-acid" | "acetic" => Some(library::acetic_acid()),
        "benzene" => Some(library::benzene()),
        "toluene" | "methylbenzene" => Some(library::toluene()),
        "bromobenzene" => Some(library::bromobenzene()),
        "phenylboronic-acid" | "phenylboronicacid" => Some(library::phenylboronic_acid()),
        "biphenyl" => Some(library::biphenyl()),
        "ferrocene" => Some(demo_ferrocene_moladt()),
        _ => None,
    }
}

fn sanitize_filename_stem(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "molecule".to_string()
    } else {
        out
    }
}

fn run_moladt_dft_demo() -> Result<(), String> {
    let molecule = demo_ferrocene_moladt();
    let agent = AgentId("operator.chimiaclaw.eth".to_string());
    let signer = NodeProfile::dev_signer_from_seed_label("moladt-dft-demo");
    let molecule_artifact = molecule_artifact(&molecule, agent.clone(), &signer, 1)
        .map_err(|error| format!("sign molecule artifact: {error}"))?;
    let molecule_ref = DftMoleculeRef::unbound(&molecule).with_artifact(&molecule_artifact);
    let request = DftRequest {
        request_id: "REQ.MOLADT.DFT.FERROCENE.001".to_string(),
        molecule: molecule_ref,
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
        worker_hint: Some("CHIMIACLAW_DFT_COMMAND".to_string()),
    };
    let request_artifact = dft_request_artifact(&request, agent, &signer, 2)
        .map_err(|error| format!("sign dft request artifact: {error}"))?;
    let xyz = molecule
        .to_xyz()
        .map_err(|error| format!("render xyz projection: {error}"))?;
    let pyscf = molecule
        .to_pyscf_atom_block()
        .map_err(|error| format!("render pyscf atom block: {error}"))?;
    let report = serde_json::json!({
        "molecule": molecule,
        "projections": {
            "xyz": xyz,
            "pyscf_atom_block": pyscf,
        },
        "molecule_artifact": molecule_artifact,
        "dft_request": request,
        "dft_request_artifact": request_artifact,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("serialize moladt dft demo: {error}"))?
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
    println!("  chimiaclaw-cli moladt-dft-demo");
    println!(
        "      Print a signed MolADT molecule artifact, XYZ/PySCF projections, and a signed DFT request artifact."
    );
    println!("  chimiaclaw-cli ord-moladt-demo [--ord-json <path>] [--official-ord-json <path>] [--output-dir <dir>]");
    println!(
        "      Translate every substrate in an ORD-like or official ORD reaction into signed MolADT artifacts via the curated library, listing skipped entries that still need an external geometry pre-pass. With --output-dir, writes one .xyz and one .svg per resolved substrate."
    );
    println!("  chimiaclaw-cli moladt-render [--library <name>|--smiles <smiles>] [--xyz <path>] [--svg <path>] [--no-worker]");
    println!(
        "      Render a curated library molecule (or SMILES via the optional CHIMIACLAW_SMILES_TO_MOLADT_COMMAND worker) to XYZ and pure-Rust SVG, printing a JSON summary that includes the inline XYZ block."
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
    println!("  chimiaclaw-cli live ens-verify --agent <id> --ens <name> [--expect-address <0x..>] [--expect-text key=value]");
    println!("      Feature-gated live ENS text/address resolution; compile with --features live-sponsors.");
    println!("  chimiaclaw-cli live ens-publish --agent <id> --ens <name> --record key=value ... [--out-dir <dir>] [--dry-run] [--no-verify]");
    println!("      Publish ChimiaClaw text records via the configured ENS publisher worker, then sign publication+resolution+verification artifacts (round-trip).");
    println!("  chimiaclaw-cli live zerog-anchor --source-artifact-json <path> --payload-file <path> [--agent <id>]");
    println!(
        "      Feature-gated 0G upload through ZEROG_UPLOAD_COMMAND and signed anchor artifact."
    );
    println!("  chimiaclaw-cli live keeperhub-schedule --workflow-id <id> [--input-json <json>] [--parent-artifact-id <id>]");
    println!("      Feature-gated KeeperHub workflow execution and signed scheduled artifact.");
    println!(
        "  chimiaclaw-cli live keeperhub-status --execution-id <id> [--scheduled-artifact-id <id>]"
    );
    println!("      Feature-gated KeeperHub execution status and signed status artifact.");
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

#[cfg(feature = "live-sponsors")]
fn run_live_ens_verify(args: &[&str]) -> Result<(), String> {
    use chimiaclaw_identity_ens::{
        default_text_keys, resolution_artifact, verification_artifact, verify_resolution,
        EnsVerificationExpectation, LiveEnsResolver,
    };
    use std::collections::BTreeMap;

    let agent = AgentId(require_kv(args, "--agent")?.to_string());
    let ens_name = require_kv(args, "--ens")?;
    let created_at_unix = parse_optional_u64(args, "--created-at")?.unwrap_or_else(unix_now);
    let expected_address = parse_kv(args, "--expect-address").map(str::to_string);
    let expected_axl_peer_id = parse_kv(args, "--expect-axl-peer-id").map(str::to_string);
    let expected_head_artifact_cid = parse_kv(args, "--expect-head-cid").map(str::to_string);
    let mut required_text_records = BTreeMap::new();
    for spec in parse_all_kv(args, "--expect-text") {
        let (key, value) = spec
            .split_once('=')
            .ok_or_else(|| format!("--expect-text must be key=value, got {spec:?}"))?;
        required_text_records.insert(key.to_string(), value.to_string());
    }
    let mut text_keys = default_text_keys();
    for key in required_text_records.keys() {
        if !text_keys.iter().any(|candidate| candidate == key) {
            text_keys.push(key.clone());
        }
    }
    for key in parse_all_kv(args, "--text-key") {
        if !text_keys.iter().any(|candidate| candidate == key) {
            text_keys.push(key.to_string());
        }
    }

    let resolver = LiveEnsResolver::from_env().map_err(|error| error.to_string())?;
    let resolution = resolver
        .resolve(agent.clone(), ens_name, &text_keys, created_at_unix)
        .map_err(|error| error.to_string())?;
    let expectation = EnsVerificationExpectation {
        agent,
        ens_name: ens_name.to_string(),
        expected_address,
        expected_axl_peer_id,
        expected_head_artifact_cid,
        required_text_records,
    };
    let report = verify_resolution(&expectation, &resolution);
    let signer = NodeProfile::dev_signer_from_seed_label("live:ens");
    let resolution_artifact = resolution_artifact(&resolution, &signer, created_at_unix)
        .map_err(|error| error.to_string())?;
    let verification_artifact = verification_artifact(
        &report,
        &signer,
        Some(resolution_artifact.id.clone()),
        created_at_unix + 1,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "resolution": resolution,
            "verification": report,
            "artifacts": [resolution_artifact, verification_artifact],
        }))
        .map_err(|error| format!("serialize: {error}"))?
    );
    Ok(())
}

#[cfg(feature = "live-sponsors")]
fn run_live_ens_publish(args: &[&str]) -> Result<(), String> {
    use chimiaclaw_identity_ens::{
        default_text_keys, publication_artifact, resolution_artifact, verification_artifact,
        verify_resolution, EnsPublication, EnsPublisherCommandConfig, ENS_PUBLISHER_COMMAND_ENV,
    };

    let agent = AgentId(require_kv(args, "--agent")?.to_string());
    let ens_name = require_kv(args, "--ens")?.to_string();
    let out_dir = parse_kv(args, "--out-dir").map(PathBuf::from);
    let created_at_unix = parse_optional_u64(args, "--created-at")?.unwrap_or_else(unix_now);
    let dry_run = args.iter().any(|arg| *arg == "--dry-run");
    let allow_mainnet = args.iter().any(|arg| *arg == "--allow-mainnet");
    let skip_verify = args.iter().any(|arg| *arg == "--no-verify");
    let mut record_specs: Vec<(String, String)> = Vec::new();
    for spec in parse_all_kv(args, "--record") {
        let (key, value) = spec
            .split_once('=')
            .ok_or_else(|| format!("--record must be key=value, got {spec:?}"))?;
        record_specs.push((key.to_string(), value.to_string()));
    }
    if record_specs.is_empty() {
        return Err("at least one --record key=value is required".to_string());
    }

    let publisher = EnsPublisherCommandConfig::from_env()
        .map_err(|error| format!("{ENS_PUBLISHER_COMMAND_ENV} not configured: {error}"))?;
    let publication: EnsPublication = publisher
        .invoke(&ens_name, &record_specs, dry_run, allow_mainnet)
        .map_err(|error| format!("ens publisher failed: {error}"))?;

    let signer = NodeProfile::dev_signer_from_seed_label("live:ens-publish");
    let publication_art =
        publication_artifact(&publication, agent.clone(), &signer, created_at_unix)
            .map_err(|error| format!("sign publication artifact: {error}"))?;

    let mut artifacts = vec![publication_art.clone()];
    let mut resolution_artifact_opt = None;
    let mut verification_artifact_opt = None;
    if !skip_verify {
        let resolver =
            chimiaclaw_identity_ens::LiveEnsResolver::from_env().map_err(|e| e.to_string())?;
        let mut text_keys = default_text_keys();
        for (key, _) in &record_specs {
            if !text_keys.iter().any(|candidate| candidate == key) {
                text_keys.push(key.clone());
            }
        }
        let resolution = resolver
            .resolve(agent.clone(), &ens_name, &text_keys, created_at_unix + 1)
            .map_err(|error| format!("live ens resolve: {error}"))?;
        let resolution_art = resolution_artifact(&resolution, &signer, created_at_unix + 2)
            .map_err(|error| format!("sign resolution artifact: {error}"))?;
        let expectation = publication.verification_expectation(agent.clone());
        let report = verify_resolution(&expectation, &resolution);
        let verification_art = verification_artifact(
            &report,
            &signer,
            Some(resolution_art.id.clone()),
            created_at_unix + 3,
        )
        .map_err(|error| format!("sign verification artifact: {error}"))?;
        resolution_artifact_opt = Some(resolution_art.clone());
        verification_artifact_opt = Some(verification_art.clone());
        artifacts.push(resolution_art);
        artifacts.push(verification_art);
    }

    if let Some(dir) = out_dir.as_ref() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("create out-dir {}: {error}", dir.display()))?;
        for artifact in &artifacts {
            let stem = artifact
                .schema_tags
                .iter()
                .next()
                .map(|tag| tag.0.replace('.', "_"))
                .unwrap_or_else(|| "artifact".to_string());
            let path = dir.join(format!("{stem}.{}.json", artifact.id.0));
            let serialized = serde_json::to_string_pretty(artifact)
                .map_err(|error| format!("serialize artifact: {error}"))?;
            std::fs::write(&path, serialized)
                .map_err(|error| format!("write {}: {error}", path.display()))?;
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication": publication,
            "publication_artifact": publication_art,
            "resolution_artifact": resolution_artifact_opt,
            "verification_artifact": verification_artifact_opt,
            "out_dir": out_dir.as_ref().map(|p| p.display().to_string()),
        }))
        .map_err(|error| format!("serialize report: {error}"))?
    );
    Ok(())
}

#[cfg(feature = "live-sponsors")]
fn run_live_zerog_anchor(args: &[&str]) -> Result<(), String> {
    use chimiaclaw_storage_0g::{upload_anchor_artifact, ZeroGCommandUploader};
    use std::fs;

    let source_artifact_json = PathBuf::from(require_kv(args, "--source-artifact-json")?);
    let payload_file = PathBuf::from(require_kv(args, "--payload-file")?);
    let agent = AgentId(
        parse_kv(args, "--agent")
            .unwrap_or("storage.zerog.operator.chimiaclaw.eth")
            .to_string(),
    );
    let created_at_unix = parse_optional_u64(args, "--created-at")?.unwrap_or_else(unix_now);
    let source: chimiaclaw_artifact::Artifact = serde_json::from_str(
        &fs::read_to_string(&source_artifact_json)
            .map_err(|error| format!("read source artifact json: {error}"))?,
    )
    .map_err(|error| format!("parse source artifact json: {error}"))?;
    source
        .verify()
        .map_err(|error| format!("source artifact verification failed: {error:?}"))?;
    let uploader = ZeroGCommandUploader::from_env().map_err(|error| error.to_string())?;
    let receipt = uploader
        .upload_file(&payload_file, created_at_unix)
        .map_err(|error| error.to_string())?;
    let signer = NodeProfile::dev_signer_from_seed_label("live:zerog");
    let anchor = upload_anchor_artifact(&source, receipt, agent, &signer, created_at_unix + 1)
        .map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&anchor).map_err(|error| format!("serialize: {error}"))?
    );
    Ok(())
}

#[cfg(feature = "live-sponsors")]
fn run_live_keeperhub_schedule(args: &[&str]) -> Result<(), String> {
    use chimiaclaw_exec_keeperhub::{scheduled_artifact, KeeperHubClient};

    let workflow_id = require_kv(args, "--workflow-id")?;
    let input = parse_kv(args, "--input-json").unwrap_or("{}");
    let input: serde_json::Value =
        serde_json::from_str(input).map_err(|error| format!("invalid --input-json: {error}"))?;
    let parent_artifact_id =
        parse_kv(args, "--parent-artifact-id").map(|id| ArtifactId(id.to_string()));
    let agent = AgentId(
        parse_kv(args, "--agent")
            .unwrap_or("keeperhub.operator.chimiaclaw.eth")
            .to_string(),
    );
    let created_at_unix = parse_optional_u64(args, "--created-at")?.unwrap_or_else(unix_now);
    let client = KeeperHubClient::from_env().map_err(|error| error.to_string())?;
    let scheduled = client
        .execute_workflow(workflow_id, input, created_at_unix)
        .map_err(|error| error.to_string())?;
    let signer = NodeProfile::dev_signer_from_seed_label("live:keeperhub");
    let artifact = scheduled_artifact(
        &scheduled,
        parent_artifact_id,
        agent,
        &signer,
        created_at_unix + 1,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "scheduled": scheduled,
            "artifact": artifact,
        }))
        .map_err(|error| format!("serialize: {error}"))?
    );
    Ok(())
}

#[cfg(feature = "live-sponsors")]
fn run_live_keeperhub_status(args: &[&str]) -> Result<(), String> {
    use chimiaclaw_exec_keeperhub::{status_artifact, KeeperHubClient};

    let execution_id = require_kv(args, "--execution-id")?;
    let scheduled_artifact_id =
        parse_kv(args, "--scheduled-artifact-id").map(|id| ArtifactId(id.to_string()));
    let agent = AgentId(
        parse_kv(args, "--agent")
            .unwrap_or("keeperhub.operator.chimiaclaw.eth")
            .to_string(),
    );
    let created_at_unix = parse_optional_u64(args, "--created-at")?.unwrap_or_else(unix_now);
    let client = KeeperHubClient::from_env().map_err(|error| error.to_string())?;
    let status = client
        .execution_status(execution_id, created_at_unix)
        .map_err(|error| error.to_string())?;
    let signer = NodeProfile::dev_signer_from_seed_label("live:keeperhub");
    let artifact = status_artifact(
        &status,
        scheduled_artifact_id,
        agent,
        &signer,
        created_at_unix + 1,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": status,
            "artifact": artifact,
        }))
        .map_err(|error| format!("serialize: {error}"))?
    );
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
