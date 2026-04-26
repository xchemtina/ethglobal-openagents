//! KeeperHub execution adapter placeholder.

use chimiaclaw_artifact::ArtifactId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledJob {
    pub job_artifact: ArtifactId,
    pub keeper_job_id: String,
}

pub trait JobScheduler {
    fn schedule(&self, job_artifact: &ArtifactId) -> Result<ScheduledJob, String>;
    fn status(&self, keeper_job_id: &str) -> Result<String, String>;
}
