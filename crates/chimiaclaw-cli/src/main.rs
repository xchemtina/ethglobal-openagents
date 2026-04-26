use chimiaclaw_artifact::{
    ArtifactDraft, ArtifactSigner, ArtifactStore, InMemoryArtifactStore, PayloadRef,
};
use chimiaclaw_ord_adt::{
    adt_experiment_hash, demo_suzuki_ord_like, OrdToAdtTranslator, ADT_REACTION_TAG, ORD_ADT_AGENT,
    ORD_REACTION_TAG,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use retroquoter::{
    ProcurementExecutionRequest, ProcurementExecutor, ReagentCatalog, ReagentRequirement,
    ReagentRole, RetroQuoter, RouteProposal, RouteStep, SupplierOffer, PLANNER_AGENT,
    PROCUREMENT_AGENT, ROUTE_PROPOSAL_TAG,
};
use std::collections::BTreeSet;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("demo-dag") => run_demo_dag(),
        Some("demo-ord-adt") => run_demo_ord_adt(),
        _ => print_help(),
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
    println!("  chimiaclaw-cli demo-dag    create and verify a deterministic local artifact DAG");
    println!("  chimiaclaw-cli demo-ord-adt    translate demo ORD-like reaction into signed ADT artifact");
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
