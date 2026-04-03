//! Changed-file summaries for proxy execution.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::error::Result;
use crate::storage::VaultBackend;

#[derive(Debug, Clone, Copy)]
pub struct ChangeSummaryBaseline {
    audit_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeSummary {
    pub changed_files: Vec<String>,
    pub audit_entries: u64,
}

pub struct ChangeSummaryService;

impl ChangeSummaryService {
    pub fn baseline(&self, backend: &VaultBackend) -> Result<ChangeSummaryBaseline> {
        Ok(ChangeSummaryBaseline {
            audit_count: backend.get_audit_count()?,
        })
    }

    pub fn summarize(
        &self,
        backend: &VaultBackend,
        baseline: ChangeSummaryBaseline,
    ) -> Result<ChangeSummary> {
        let current_count = backend.get_audit_count()?;
        let delta = current_count.saturating_sub(baseline.audit_count);

        if delta == 0 {
            return Ok(ChangeSummary {
                changed_files: Vec::new(),
                audit_entries: 0,
            });
        }

        let entries = backend.get_audit_log(delta as usize, None)?;
        let mut changed_files = BTreeSet::new();

        for entry in entries {
            if entry.operation.starts_with("proxy_exec_") {
                continue;
            }

            if let Some(path) = entry.path {
                changed_files.insert(path);
            }
        }

        Ok(ChangeSummary {
            changed_files: changed_files.into_iter().collect(),
            audit_entries: delta,
        })
    }
}
