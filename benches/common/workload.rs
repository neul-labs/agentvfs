//! Agent workload simulation.

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use agentvfs::fs::FileSystem;

use super::fixtures::DataGenerator;

/// Operation types for workload mix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Write,
    Read,
    List,
    Search,
    Delete,
    Copy,
    Move,
}

/// Workload configuration.
#[derive(Clone)]
pub struct WorkloadConfig {
    /// Operations per agent.
    pub ops_per_agent: usize,
    /// Operation mix (operation, weight).
    pub op_weights: Vec<(Operation, f64)>,
    /// File size range (min, max).
    pub file_size_range: (usize, usize),
    /// Search query patterns.
    pub search_keywords: Vec<String>,
}

impl Default for WorkloadConfig {
    fn default() -> Self {
        Self {
            ops_per_agent: 1000,
            op_weights: vec![
                (Operation::Write, 0.30),
                (Operation::Read, 0.35),
                (Operation::List, 0.15),
                (Operation::Search, 0.10),
                (Operation::Delete, 0.05),
                (Operation::Copy, 0.03),
                (Operation::Move, 0.02),
            ],
            file_size_range: (1024, 65536),
            search_keywords: vec![
                "important".into(),
                "todo".into(),
                "note".into(),
                "benchmark".into(),
            ],
        }
    }
}

impl WorkloadConfig {
    /// Create a read-heavy workload (80% reads).
    pub fn read_heavy() -> Self {
        Self {
            op_weights: vec![
                (Operation::Write, 0.10),
                (Operation::Read, 0.60),
                (Operation::List, 0.20),
                (Operation::Search, 0.05),
                (Operation::Delete, 0.02),
                (Operation::Copy, 0.02),
                (Operation::Move, 0.01),
            ],
            ..Default::default()
        }
    }

    /// Create a write-heavy workload (60% writes).
    pub fn write_heavy() -> Self {
        Self {
            op_weights: vec![
                (Operation::Write, 0.50),
                (Operation::Read, 0.20),
                (Operation::List, 0.10),
                (Operation::Search, 0.05),
                (Operation::Delete, 0.10),
                (Operation::Copy, 0.03),
                (Operation::Move, 0.02),
            ],
            ..Default::default()
        }
    }

    /// Create a search-focused workload.
    pub fn search_focused() -> Self {
        Self {
            op_weights: vec![
                (Operation::Write, 0.20),
                (Operation::Read, 0.20),
                (Operation::List, 0.10),
                (Operation::Search, 0.40),
                (Operation::Delete, 0.05),
                (Operation::Copy, 0.03),
                (Operation::Move, 0.02),
            ],
            ..Default::default()
        }
    }
}

/// Agent workload statistics.
#[derive(Debug)]
pub struct WorkloadStats {
    pub success_count: AtomicU64,
    pub error_count: AtomicU64,
    pub write_count: AtomicU64,
    pub read_count: AtomicU64,
    pub list_count: AtomicU64,
    pub search_count: AtomicU64,
    pub delete_count: AtomicU64,
    pub copy_count: AtomicU64,
    pub move_count: AtomicU64,
}

impl Default for WorkloadStats {
    fn default() -> Self {
        Self {
            success_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            write_count: AtomicU64::new(0),
            read_count: AtomicU64::new(0),
            list_count: AtomicU64::new(0),
            search_count: AtomicU64::new(0),
            delete_count: AtomicU64::new(0),
            copy_count: AtomicU64::new(0),
            move_count: AtomicU64::new(0),
        }
    }
}

impl WorkloadStats {
    /// Get total operation count.
    pub fn total_ops(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed) + self.error_count.load(Ordering::Relaxed)
    }

    /// Get success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        let total = self.total_ops();
        if total == 0 {
            return 100.0;
        }
        (self.success_count.load(Ordering::Relaxed) as f64 / total as f64) * 100.0
    }
}

/// Agent workload simulation with weighted random operations.
pub struct AgentWorkload {
    pub config: WorkloadConfig,
    pub stats: WorkloadStats,
}

impl AgentWorkload {
    /// Create a new workload with the given configuration.
    pub fn new(config: WorkloadConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            stats: WorkloadStats::default(),
        })
    }

    /// Run a single agent workload.
    pub fn run_agent(&self, fs: &FileSystem, agent_id: usize, seed: u64) {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut gen = DataGenerator::new(seed);
        let prefix = format!("/agent_{}", agent_id);

        // Create agent directory
        let _ = fs.create_dir(&prefix);

        let mut created_files: Vec<String> = Vec::new();

        for _ in 0..self.config.ops_per_agent {
            let op = self.select_operation(&mut rng);
            let result = self.execute_operation(fs, op, &prefix, &mut gen, &mut rng, &mut created_files);

            match result {
                Ok(_) => {
                    self.stats.success_count.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    self.stats.error_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    fn execute_operation(
        &self,
        fs: &FileSystem,
        op: Operation,
        prefix: &str,
        gen: &mut DataGenerator,
        rng: &mut SmallRng,
        created_files: &mut Vec<String>,
    ) -> Result<(), ()> {
        match op {
            Operation::Write => {
                self.stats.write_count.fetch_add(1, Ordering::Relaxed);
                let path = gen.path(prefix);
                let size = gen.file_size(self.config.file_size_range.0, self.config.file_size_range.1);
                let content = gen.content(size);
                let res = fs.write_file(&path, &content);
                if res.is_ok() {
                    created_files.push(path);
                }
                res.map_err(|_| ())
            }
            Operation::Read => {
                self.stats.read_count.fetch_add(1, Ordering::Relaxed);
                if created_files.is_empty() {
                    return Err(());
                }
                let path = &created_files[rng.gen_range(0..created_files.len())];
                fs.read_file(path).map(|_| ()).map_err(|_| ())
            }
            Operation::List => {
                self.stats.list_count.fetch_add(1, Ordering::Relaxed);
                fs.list_dir(prefix).map(|_| ()).map_err(|_| ())
            }
            Operation::Search => {
                self.stats.search_count.fetch_add(1, Ordering::Relaxed);
                if self.config.search_keywords.is_empty() {
                    return Err(());
                }
                let keyword =
                    &self.config.search_keywords[rng.gen_range(0..self.config.search_keywords.len())];
                fs.backend()
                    .search_content(keyword, 10)
                    .map(|_| ())
                    .map_err(|_| ())
            }
            Operation::Delete => {
                self.stats.delete_count.fetch_add(1, Ordering::Relaxed);
                if created_files.is_empty() {
                    return Err(());
                }
                let idx = rng.gen_range(0..created_files.len());
                let path = created_files.remove(idx);
                fs.remove(&path, false).map_err(|_| ())
            }
            Operation::Copy => {
                self.stats.copy_count.fetch_add(1, Ordering::Relaxed);
                if created_files.is_empty() {
                    return Err(());
                }
                let src = &created_files[rng.gen_range(0..created_files.len())];
                let dst = gen.path(prefix);
                let res = fs.copy(src, &dst);
                if res.is_ok() {
                    created_files.push(dst);
                }
                res.map_err(|_| ())
            }
            Operation::Move => {
                self.stats.move_count.fetch_add(1, Ordering::Relaxed);
                if created_files.is_empty() {
                    return Err(());
                }
                let idx = rng.gen_range(0..created_files.len());
                let src = created_files.remove(idx);
                let dst = gen.path(prefix);
                let res = fs.move_entry(&src, &dst);
                if res.is_ok() {
                    created_files.push(dst);
                }
                res.map_err(|_| ())
            }
        }
    }

    fn select_operation(&self, rng: &mut SmallRng) -> Operation {
        let total: f64 = self.config.op_weights.iter().map(|(_, w)| w).sum();
        let mut threshold: f64 = rng.gen::<f64>() * total;

        for (op, weight) in &self.config.op_weights {
            threshold -= weight;
            if threshold <= 0.0 {
                return *op;
            }
        }

        // Fallback to Read
        Operation::Read
    }
}
