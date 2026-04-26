//! Artifact-to-RDF projection placeholder for World Avatar / OntoChimia interop.

use chimiaclaw_artifact::Artifact;
use chimiaclaw_schema::SchemaTag;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RdfTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

pub fn project_artifact(artifact: &Artifact) -> Vec<RdfTriple> {
    let mut triples = vec![RdfTriple {
        subject: artifact.id.0.clone(),
        predicate: "rdf:type".to_string(),
        object: "ontochimia:Artifact".to_string(),
    }];
    triples.extend(artifact.parent_artifact_ids.iter().map(|parent| RdfTriple {
        subject: artifact.id.0.clone(),
        predicate: "prov:wasDerivedFrom".to_string(),
        object: parent.0.clone(),
    }));
    triples.extend(
        artifact
            .schema_tags
            .iter()
            .map(schema_tag_triple(&artifact.id.0)),
    );
    triples
}

fn schema_tag_triple(subject: &str) -> impl Fn(&SchemaTag) -> RdfTriple + '_ {
    move |tag| RdfTriple {
        subject: subject.to_string(),
        predicate: "ontochimia:hasSchemaTag".to_string(),
        object: tag.0.clone(),
    }
}
