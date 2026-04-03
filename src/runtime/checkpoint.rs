//! Checkpoint service for rollback-oriented workflows.

use chrono::Utc;

use crate::error::Result;
use crate::runtime::workspace::WorkspaceService;
use crate::storage::{SnapshotInfo, VaultBackend};

pub struct CheckpointService {
    workspaces: WorkspaceService,
}

impl CheckpointService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            workspaces: WorkspaceService::new()?,
        })
    }

    pub fn with_workspaces(workspaces: WorkspaceService) -> Self {
        Self { workspaces }
    }

    pub fn create_auto(&self, vault: &str, description: Option<&str>) -> Result<SnapshotInfo> {
        let backend = self.workspaces.open(vault)?;
        self.create_on_backend(&backend, None, description)
    }

    pub fn create_on_backend(
        &self,
        backend: &VaultBackend,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<SnapshotInfo> {
        let checkpoint_name = name
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("checkpoint-{}", Utc::now().format("%Y%m%d-%H%M%S")));
        backend.save_snapshot(&checkpoint_name, description)
    }
}
