//! System metrics collection for benchmarks.

#![allow(dead_code)]

use std::time::{Duration, Instant};

/// Collected metrics for a benchmark run.
#[derive(Debug, Default, Clone)]
pub struct BenchMetrics {
    pub duration: Duration,
    pub ops_count: u64,
    pub memory_before_mb: f64,
    pub memory_after_mb: f64,
    pub peak_memory_mb: f64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
}

impl BenchMetrics {
    /// Calculate throughput in operations per second.
    pub fn throughput_ops_sec(&self) -> f64 {
        if self.duration.as_secs_f64() == 0.0 {
            return 0.0;
        }
        self.ops_count as f64 / self.duration.as_secs_f64()
    }

    /// Calculate memory delta in MB.
    pub fn memory_delta_mb(&self) -> f64 {
        self.memory_after_mb - self.memory_before_mb
    }
}

/// Get current memory usage in MB.
pub fn get_memory_mb() -> f64 {
    memory_stats::memory_stats()
        .map(|stats| stats.physical_mem as f64 / 1024.0 / 1024.0)
        .unwrap_or(0.0)
}

/// I/O statistics collector.
#[cfg(target_os = "linux")]
pub struct IoStats {
    read_bytes: u64,
    write_bytes: u64,
}

#[cfg(target_os = "linux")]
impl IoStats {
    /// Capture current I/O stats from /proc.
    pub fn capture() -> Self {
        let pid = std::process::id();
        let io_content = std::fs::read_to_string(format!("/proc/{}/io", pid)).unwrap_or_default();

        let mut read_bytes = 0u64;
        let mut write_bytes = 0u64;

        for line in io_content.lines() {
            if line.starts_with("read_bytes:") {
                read_bytes = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("write_bytes:") {
                write_bytes = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }

        Self {
            read_bytes,
            write_bytes,
        }
    }

    /// Calculate the delta between two snapshots.
    pub fn delta(&self, other: &Self) -> (u64, u64) {
        (
            other.read_bytes.saturating_sub(self.read_bytes),
            other.write_bytes.saturating_sub(self.write_bytes),
        )
    }
}

#[cfg(not(target_os = "linux"))]
pub struct IoStats;

#[cfg(not(target_os = "linux"))]
impl IoStats {
    pub fn capture() -> Self {
        Self
    }

    pub fn delta(&self, _other: &Self) -> (u64, u64) {
        (0, 0)
    }
}

/// Metrics collector for tracking benchmark runs.
pub struct MetricsCollector {
    start_time: Instant,
    start_memory: f64,
    start_io: IoStats,
    peak_memory: f64,
    ops_count: u64,
}

impl MetricsCollector {
    /// Start collecting metrics.
    pub fn start() -> Self {
        let start_memory = get_memory_mb();
        Self {
            start_time: Instant::now(),
            start_memory,
            start_io: IoStats::capture(),
            peak_memory: start_memory,
            ops_count: 0,
        }
    }

    /// Record an operation and update peak memory.
    pub fn record_op(&mut self) {
        self.ops_count += 1;
        let current_mem = get_memory_mb();
        if current_mem > self.peak_memory {
            self.peak_memory = current_mem;
        }
    }

    /// Record multiple operations.
    pub fn record_ops(&mut self, count: u64) {
        self.ops_count += count;
        let current_mem = get_memory_mb();
        if current_mem > self.peak_memory {
            self.peak_memory = current_mem;
        }
    }

    /// Finish collecting and return metrics.
    pub fn finish(self) -> BenchMetrics {
        let end_memory = get_memory_mb();
        let end_io = IoStats::capture();
        let (io_read, io_write) = self.start_io.delta(&end_io);

        BenchMetrics {
            duration: self.start_time.elapsed(),
            ops_count: self.ops_count,
            memory_before_mb: self.start_memory,
            memory_after_mb: end_memory,
            peak_memory_mb: self.peak_memory.max(end_memory),
            io_read_bytes: io_read,
            io_write_bytes: io_write,
        }
    }
}

/// Simple timer for measuring operation latency.
pub struct LatencyTimer {
    start: Instant,
}

impl LatencyTimer {
    /// Start the timer.
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed time since start.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Stop the timer and return elapsed duration.
    pub fn stop(self) -> Duration {
        self.start.elapsed()
    }
}
