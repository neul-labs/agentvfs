//! Main benchmark harness.
//!
//! This file ties together all benchmark modules and defines the criterion groups.
//!
//! Run with:
//!   cargo bench --all-features              # All backends
//!   cargo bench                             # SQLite only
//!   cargo bench -- single_vault             # Single vault benchmarks
//!   cargo bench -- multi_vault              # Multi-vault benchmarks
//!   cargo bench -- --save-baseline main     # Save baseline
//!   cargo bench -- --baseline main          # Compare to baseline

use std::sync::Arc;
use std::thread;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rayon::prelude::*;

use agentvfs::fs::FileSystem;

mod common;

use common::fixtures::{DataGenerator, LARGE_FILE, MEDIUM_FILE, SMALL_FILE};
use common::workload::{AgentWorkload, WorkloadConfig};
use common::{available_backends, TestVault, VaultPool};

#[cfg(feature = "fuse")]
use common::FuseTestVault;

// ============================================================================
// Single Vault Benchmarks
// ============================================================================

/// Benchmark write throughput across different file sizes.
fn bench_write_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vault/write");

    for backend in available_backends() {
        for &size in &[SMALL_FILE, MEDIUM_FILE, LARGE_FILE] {
            let size_label = match size {
                SMALL_FILE => "1KB",
                MEDIUM_FILE => "64KB",
                LARGE_FILE => "1MB",
                _ => "unknown",
            };

            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(
                BenchmarkId::new(backend.name, size_label),
                &(backend.backend_type, size),
                |b, &(backend_type, size)| {
                    let vault = TestVault::new(backend_type);
                    let mut gen = DataGenerator::new(42);
                    vault.fs.create_dir("/bench").unwrap();

                    b.iter_batched(
                        || (gen.path("/bench"), gen.content(size)),
                        |(path, content)| vault.fs.write_file(&path, &content),
                        criterion::BatchSize::SmallInput,
                    );
                },
            );
        }
    }

    group.finish();
}

/// Benchmark read throughput across different file sizes.
fn bench_read_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vault/read");

    for backend in available_backends() {
        for &size in &[SMALL_FILE, MEDIUM_FILE, LARGE_FILE] {
            let size_label = match size {
                SMALL_FILE => "1KB",
                MEDIUM_FILE => "64KB",
                LARGE_FILE => "1MB",
                _ => "unknown",
            };

            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(
                BenchmarkId::new(backend.name, size_label),
                &(backend.backend_type, size),
                |b, &(backend_type, size)| {
                    let vault = TestVault::new(backend_type);
                    let mut gen = DataGenerator::new(42);

                    // Setup: create files to read
                    vault.fs.create_dir("/bench").unwrap();
                    let paths: Vec<_> = (0..100)
                        .map(|i| {
                            let path = format!("/bench/file_{}.txt", i);
                            vault.fs.write_file(&path, &gen.content(size)).unwrap();
                            path
                        })
                        .collect();

                    let mut idx = 0;
                    b.iter(|| {
                        let path = &paths[idx % paths.len()];
                        idx += 1;
                        vault.fs.read_file(path)
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark full-text search performance.
fn bench_search_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vault/search");

    for backend in available_backends() {
        group.bench_with_input(
            BenchmarkId::new(backend.name, "fts"),
            &backend.backend_type,
            |b, &backend_type| {
                let vault = TestVault::new(backend_type);
                let mut gen = DataGenerator::new(42);

                // Setup: create searchable files
                vault.fs.create_dir("/docs").unwrap();
                for i in 0..100 {
                    let path = format!("/docs/doc_{}.txt", i);
                    let content = gen.searchable_text(4096, &["benchmark", "test", "important"]);
                    vault.fs.write_file(&path, content.as_bytes()).unwrap();
                }

                b.iter(|| vault.backend.search_content("benchmark", 20));
            },
        );
    }

    group.finish();
}

/// Benchmark directory listing with different file counts.
fn bench_list_dir(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vault/list");

    for backend in available_backends() {
        for &file_count in &[10, 100, 1000] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, file_count),
                &(backend.backend_type, file_count),
                |b, &(backend_type, count)| {
                    let vault = TestVault::new(backend_type);
                    let mut gen = DataGenerator::new(42);

                    vault.fs.create_dir("/listing").unwrap();
                    for i in 0..count {
                        vault
                            .fs
                            .write_file(&format!("/listing/f_{}.txt", i), &gen.content(100))
                            .unwrap();
                    }

                    b.iter(|| vault.fs.list_dir("/listing"));
                },
            );
        }
    }

    group.finish();
}

/// Benchmark concurrent agent simulation on a single vault.
fn bench_concurrent_agents(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vault/concurrent");
    group.sample_size(10); // Fewer samples for longer benchmarks

    for backend in available_backends() {
        for &agent_count in &[1, 2, 4, 8, 16] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_agents", agent_count)),
                &(backend.backend_type, agent_count),
                |b, &(backend_type, agents)| {
                    b.iter_custom(|iters| {
                        let mut total_duration = std::time::Duration::ZERO;

                        for iter in 0..iters {
                            let vault = TestVault::new(backend_type);
                            let backend = vault.backend;

                            let config = WorkloadConfig {
                                ops_per_agent: 100,
                                ..Default::default()
                            };
                            let workload = AgentWorkload::new(config);

                            let start = std::time::Instant::now();

                            // Spawn agent threads
                            let handles: Vec<_> = (0..agents)
                                .map(|i| {
                                    let backend_clone = Arc::clone(&backend);
                                    let wl = Arc::clone(&workload);
                                    thread::spawn(move || {
                                        let fs = FileSystem::new(backend_clone);
                                        wl.run_agent(&fs, i, iter * 1000 + i as u64);
                                    })
                                })
                                .collect();

                            for h in handles {
                                h.join().unwrap();
                            }

                            total_duration += start.elapsed();
                        }

                        total_duration
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark mixed workload latency distribution.
fn bench_mixed_workload_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vault/latency");

    for backend in available_backends() {
        group.bench_with_input(
            BenchmarkId::new(backend.name, "mixed"),
            &backend.backend_type,
            |b, &backend_type| {
                let vault = TestVault::new(backend_type);
                let mut gen = DataGenerator::new(42);

                // Pre-populate with data
                vault.fs.create_dir("/data").unwrap();
                for i in 0..50 {
                    vault
                        .fs
                        .write_file(&format!("/data/file_{}.txt", i), &gen.content(4096))
                        .unwrap();
                }

                b.iter(|| {
                    // Mixed operation sequence
                    vault
                        .fs
                        .write_file(&gen.path("/data"), &gen.content(1024))
                        .unwrap();
                    let _ = vault.fs.read_file("/data/file_0.txt");
                    let _ = vault.fs.list_dir("/data");
                });
            },
        );
    }

    group.finish();
}

/// Run read-heavy workload benchmark.
fn bench_read_heavy_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vault/workload_read_heavy");
    group.sample_size(10);

    for backend in available_backends() {
        group.bench_with_input(
            BenchmarkId::new(backend.name, "4_agents"),
            &backend.backend_type,
            |b, &backend_type| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::ZERO;

                    for iter in 0..iters {
                        let vault = TestVault::new(backend_type);
                        let backend = vault.backend;

                        // Pre-populate with data
                        let fs = FileSystem::new(Arc::clone(&backend));
                        fs.create_dir("/data").unwrap();
                        let mut gen = DataGenerator::new(42);
                        for i in 0..100 {
                            fs.write_file(&format!("/data/file_{}.txt", i), &gen.content(4096))
                                .unwrap();
                        }

                        let config = WorkloadConfig::read_heavy();
                        let workload = AgentWorkload::new(WorkloadConfig {
                            ops_per_agent: 200,
                            ..config
                        });

                        let start = std::time::Instant::now();

                        let handles: Vec<_> = (0..4)
                            .map(|i| {
                                let backend_clone = Arc::clone(&backend);
                                let wl = Arc::clone(&workload);
                                thread::spawn(move || {
                                    let fs = FileSystem::new(backend_clone);
                                    wl.run_agent(&fs, i, iter * 1000 + i as u64);
                                })
                            })
                            .collect();

                        for h in handles {
                            h.join().unwrap();
                        }

                        total_duration += start.elapsed();
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

/// Run write-heavy workload benchmark.
fn bench_write_heavy_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vault/workload_write_heavy");
    group.sample_size(10);

    for backend in available_backends() {
        group.bench_with_input(
            BenchmarkId::new(backend.name, "4_agents"),
            &backend.backend_type,
            |b, &backend_type| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::ZERO;

                    for iter in 0..iters {
                        let vault = TestVault::new(backend_type);
                        let backend = vault.backend;

                        let config = WorkloadConfig::write_heavy();
                        let workload = AgentWorkload::new(WorkloadConfig {
                            ops_per_agent: 200,
                            ..config
                        });

                        let start = std::time::Instant::now();

                        let handles: Vec<_> = (0..4)
                            .map(|i| {
                                let backend_clone = Arc::clone(&backend);
                                let wl = Arc::clone(&workload);
                                thread::spawn(move || {
                                    let fs = FileSystem::new(backend_clone);
                                    wl.run_agent(&fs, i, iter * 1000 + i as u64);
                                })
                            })
                            .collect();

                        for h in handles {
                            h.join().unwrap();
                        }

                        total_duration += start.elapsed();
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Multi-Vault Benchmarks
// ============================================================================

/// Benchmark vault creation at scale.
fn bench_vault_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_vault/creation");
    group.sample_size(10);

    for backend in available_backends() {
        for &count in &[10, 100, 500] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, count),
                &(backend.backend_type, count),
                |b, &(backend_type, count)| {
                    b.iter(|| VaultPool::new(count, backend_type));
                },
            );
        }
    }

    group.finish();
}

/// Benchmark parallel operations across many vaults.
fn bench_parallel_vault_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_vault/parallel_ops");
    group.sample_size(10);

    for backend in available_backends() {
        for &vault_count in &[10, 50, 100] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_vaults", vault_count)),
                &(backend.backend_type, vault_count),
                |b, &(backend_type, count)| {
                    let pool = VaultPool::new(count, backend_type);

                    b.iter(|| {
                        // Parallel operations across all vaults
                        pool.vaults.par_iter().enumerate().for_each(|(i, vault)| {
                            let mut gen = DataGenerator::new(i as u64);
                            let _ = vault.fs.create_dir("/test");
                            vault
                                .fs
                                .write_file("/test/file.txt", &gen.content(1024))
                                .unwrap();
                            let _ = vault.fs.read_file("/test/file.txt");
                            let _ = vault.fs.list_dir("/");
                        });
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark agent simulation across vault pool.
fn bench_multi_vault_agent_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_vault/agents");
    group.sample_size(10);

    for backend in available_backends() {
        // Different configurations: (vault_count, agents_per_vault)
        for &(vaults, agents_per_vault) in &[(50, 2), (100, 1), (200, 1)] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}v_{}a", vaults, agents_per_vault)),
                &(backend.backend_type, vaults, agents_per_vault),
                |b, &(backend_type, vault_count, agent_count)| {
                    b.iter_custom(|iters| {
                        let mut total = std::time::Duration::ZERO;

                        for iter in 0..iters {
                            let pool = VaultPool::new(vault_count, backend_type);
                            let config = WorkloadConfig {
                                ops_per_agent: 50,
                                ..Default::default()
                            };

                            let start = std::time::Instant::now();

                            pool.vaults.par_iter().enumerate().for_each(|(v, vault)| {
                                let workload = AgentWorkload::new(config.clone());
                                for a in 0..agent_count {
                                    workload.run_agent(
                                        &vault.fs,
                                        a,
                                        iter * 10000 + v as u64 * 100 + a as u64,
                                    );
                                }
                            });

                            total += start.elapsed();
                        }

                        total
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark search across multiple vaults.
fn bench_multi_vault_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_vault/search");
    group.sample_size(10);

    for backend in available_backends() {
        for &vault_count in &[10, 50] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_vaults", vault_count)),
                &(backend.backend_type, vault_count),
                |b, &(backend_type, count)| {
                    let pool = VaultPool::new(count, backend_type);

                    // Setup: populate each vault with searchable content
                    for (i, vault) in pool.vaults.iter().enumerate() {
                        let mut gen = DataGenerator::new(i as u64);
                        vault.fs.create_dir("/docs").unwrap();
                        for j in 0..10 {
                            let path = format!("/docs/doc_{}.txt", j);
                            let content =
                                gen.searchable_text(2048, &["benchmark", "important", "test"]);
                            vault.fs.write_file(&path, content.as_bytes()).unwrap();
                        }
                    }

                    b.iter(|| {
                        // Search across all vaults in parallel
                        pool.vaults.par_iter().for_each(|vault| {
                            let _ = vault.backend.search_content("benchmark", 10);
                        });
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark mixed read/write operations across vaults.
fn bench_multi_vault_mixed_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_vault/mixed_ops");
    group.sample_size(10);

    for backend in available_backends() {
        for &vault_count in &[25, 50, 100] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_vaults", vault_count)),
                &(backend.backend_type, vault_count),
                |b, &(backend_type, count)| {
                    let pool = VaultPool::new(count, backend_type);

                    // Setup: create initial files
                    for (i, vault) in pool.vaults.iter().enumerate() {
                        let mut gen = DataGenerator::new(i as u64);
                        vault.fs.create_dir("/data").unwrap();
                        for j in 0..5 {
                            vault
                                .fs
                                .write_file(&format!("/data/file_{}.txt", j), &gen.content(1024))
                                .unwrap();
                        }
                    }

                    b.iter(|| {
                        pool.vaults.par_iter().enumerate().for_each(|(i, vault)| {
                            let mut gen = DataGenerator::new(i as u64 + 1000);

                            // Mixed operations
                            let _ = vault.fs.list_dir("/data");
                            let _ = vault.fs.read_file("/data/file_0.txt");
                            vault
                                .fs
                                .write_file(&gen.path("/data"), &gen.content(512))
                                .unwrap();
                            let _ = vault.fs.read_file("/data/file_1.txt");
                        });
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark vault isolation (operations on one vault don't affect others).
fn bench_vault_isolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_vault/isolation");
    group.sample_size(10);

    for backend in available_backends() {
        group.bench_with_input(
            BenchmarkId::new(backend.name, "50_vaults_concurrent_writes"),
            &backend.backend_type,
            |b, &backend_type| {
                let pool = VaultPool::new(50, backend_type);

                b.iter(|| {
                    // Heavy writes on all vaults simultaneously
                    pool.vaults.par_iter().enumerate().for_each(|(i, vault)| {
                        let mut gen = DataGenerator::new(i as u64);
                        let _ = vault.fs.create_dir("/heavy");
                        for j in 0..20 {
                            vault
                                .fs
                                .write_file(&format!("/heavy/file_{}.txt", j), &gen.content(4096))
                                .unwrap();
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark scaling behavior - how throughput changes with vault count.
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_vault/scaling");
    group.sample_size(10);

    for backend in available_backends() {
        // Test how ops/sec scales with vault count
        for &vault_count in &[1, 10, 25, 50, 100] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_vaults", vault_count)),
                &(backend.backend_type, vault_count),
                |b, &(backend_type, count)| {
                    b.iter_custom(|iters| {
                        let mut total = std::time::Duration::ZERO;

                        for _ in 0..iters {
                            let pool = VaultPool::new(count, backend_type);

                            let start = std::time::Instant::now();

                            // Fixed amount of work per vault
                            pool.vaults.par_iter().enumerate().for_each(|(i, vault)| {
                                let mut gen = DataGenerator::new(i as u64);
                                let _ = vault.fs.create_dir("/scale");
                                for j in 0..10 {
                                    vault
                                        .fs
                                        .write_file(
                                            &format!("/scale/file_{}.txt", j),
                                            &gen.content(1024),
                                        )
                                        .unwrap();
                                    let _ = vault.fs.read_file(&format!("/scale/file_{}.txt", j));
                                }
                            });

                            total += start.elapsed();
                        }

                        total
                    });
                },
            );
        }
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

fn configure_criterion() -> Criterion {
    Criterion::default()
        .significance_level(0.05)
        .noise_threshold(0.02)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(5))
}

fn configure_criterion_long() -> Criterion {
    Criterion::default()
        .significance_level(0.05)
        .noise_threshold(0.05)
        .warm_up_time(std::time::Duration::from_secs(2))
        .measurement_time(std::time::Duration::from_secs(10))
        .sample_size(10)
}

// Single vault throughput benchmarks
criterion_group! {
    name = single_vault_throughput;
    config = configure_criterion();
    targets =
        bench_write_throughput,
        bench_read_throughput,
        bench_search_throughput,
        bench_list_dir
}

// Single vault concurrent agent benchmarks
criterion_group! {
    name = single_vault_concurrent;
    config = configure_criterion_long();
    targets =
        bench_concurrent_agents,
        bench_mixed_workload_latency,
        bench_read_heavy_workload,
        bench_write_heavy_workload
}

// Multi-vault scale benchmarks
criterion_group! {
    name = multi_vault_scale;
    config = configure_criterion_long();
    targets =
        bench_vault_creation,
        bench_parallel_vault_operations,
        bench_multi_vault_agent_simulation,
        bench_multi_vault_search,
        bench_multi_vault_mixed_ops,
        bench_vault_isolation,
        bench_scaling
}

// ============================================================================
// FUSE Benchmarks (requires fuse feature)
// ============================================================================

#[cfg(feature = "fuse")]
use std::process::Command;
#[cfg(feature = "fuse")]
use std::sync::atomic::{AtomicU64, Ordering};

/// Benchmark parallel FUSE writes with multiple threads.
#[cfg(feature = "fuse")]
fn bench_fuse_parallel_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuse/parallel_write");
    group.sample_size(10);

    for backend in available_backends() {
        for &thread_count in &[1, 2, 4, 8] {
            let files_per_thread = 20;
            let file_size = SMALL_FILE; // 1KB files
            let total_bytes = thread_count * files_per_thread * file_size;

            group.throughput(Throughput::Bytes(total_bytes as u64));
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_threads", thread_count)),
                &(backend.backend_type, thread_count),
                |b, &(backend_type, threads)| {
                    b.iter_custom(|iters| {
                        let mut total = std::time::Duration::ZERO;

                        for iter in 0..iters {
                            let vault = FuseTestVault::new(backend_type);
                            let bench_dir = vault.mountpoint.join("bench");
                            let counter = AtomicU64::new(iter * 10000);

                            let start = std::time::Instant::now();

                            // Parallel writes across threads
                            (0..threads).into_par_iter().for_each(|t| {
                                let mut gen = DataGenerator::new(t as u64 + iter * 1000);
                                for _i in 0..files_per_thread {
                                    let id = counter.fetch_add(1, Ordering::Relaxed);
                                    let path = bench_dir.join(format!("file_{}_{}.txt", t, id));
                                    std::fs::write(&path, gen.content(file_size)).unwrap();
                                }
                            });

                            total += start.elapsed();
                        }

                        total
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark parallel FUSE reads with multiple threads.
#[cfg(feature = "fuse")]
fn bench_fuse_parallel_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuse/parallel_read");
    group.sample_size(10);

    for backend in available_backends() {
        for &thread_count in &[1, 2, 4, 8] {
            let files_per_thread = 50;
            let file_size = SMALL_FILE;
            let total_bytes = thread_count * files_per_thread * file_size;

            group.throughput(Throughput::Bytes(total_bytes as u64));
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_threads", thread_count)),
                &(backend.backend_type, thread_count),
                |b, &(backend_type, threads)| {
                    // Setup: create vault with files
                    let vault = FuseTestVault::new(backend_type);
                    let bench_dir = vault.mountpoint.join("bench");
                    let mut gen = DataGenerator::new(42);

                    // Pre-populate with files
                    let total_files = threads * files_per_thread;
                    let paths: Vec<_> = (0..total_files)
                        .map(|i| {
                            let path = bench_dir.join(format!("file_{}.txt", i));
                            std::fs::write(&path, gen.content(file_size)).unwrap();
                            path
                        })
                        .collect();

                    b.iter(|| {
                        // Parallel reads across threads
                        (0..threads).into_par_iter().for_each(|t| {
                            let start_idx = t * files_per_thread;
                            for j in 0..files_per_thread {
                                let path = &paths[start_idx + j];
                                let _ = std::fs::read(path);
                            }
                        });
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark ripgrep search through FUSE-mounted filesystem.
#[cfg(feature = "fuse")]
fn bench_fuse_ripgrep(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuse/ripgrep");
    group.sample_size(10);

    // Check if ripgrep is available
    let rg_available = Command::new("rg").arg("--version").output().is_ok();
    if !rg_available {
        eprintln!("Warning: ripgrep (rg) not found, skipping ripgrep benchmarks");
        group.finish();
        return;
    }

    for backend in available_backends() {
        for &file_count in &[50, 200] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_files", file_count)),
                &(backend.backend_type, file_count),
                |b, &(backend_type, count)| {
                    let vault = FuseTestVault::new(backend_type);
                    let bench_dir = vault.mountpoint.join("bench");
                    let mut gen = DataGenerator::new(42);

                    // Create searchable files with keywords
                    for i in 0..count {
                        let content =
                            gen.searchable_text(2048, &["benchmark", "important", "test"]);
                        std::fs::write(bench_dir.join(format!("doc_{}.txt", i)), content).unwrap();
                    }

                    b.iter(|| {
                        Command::new("rg")
                            .args([
                                "--no-ignore",
                                "-c",
                                "benchmark",
                                bench_dir.to_str().unwrap(),
                            ])
                            .output()
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark parallel ripgrep searches (multiple concurrent searches).
#[cfg(feature = "fuse")]
fn bench_fuse_parallel_ripgrep(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuse/parallel_ripgrep");
    group.sample_size(10);

    let rg_available = Command::new("rg").arg("--version").output().is_ok();
    if !rg_available {
        group.finish();
        return;
    }

    for backend in available_backends() {
        for &search_threads in &[1, 2, 4] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_searches", search_threads)),
                &(backend.backend_type, search_threads),
                |b, &(backend_type, threads)| {
                    let vault = FuseTestVault::new(backend_type);
                    let bench_dir = vault.mountpoint.join("bench");
                    let mut gen = DataGenerator::new(42);

                    // Create searchable files
                    for i in 0..100 {
                        let content =
                            gen.searchable_text(2048, &["alpha", "beta", "gamma", "delta"]);
                        std::fs::write(bench_dir.join(format!("doc_{}.txt", i)), content).unwrap();
                    }

                    let search_terms = ["alpha", "beta", "gamma", "delta"];
                    let bench_path = bench_dir.to_str().unwrap().to_string();

                    b.iter(|| {
                        // Run multiple ripgrep searches in parallel
                        (0..threads).into_par_iter().for_each(|t| {
                            let term = search_terms[t % search_terms.len()];
                            let _ = Command::new("rg")
                                .args(["--no-ignore", "-c", term, &bench_path])
                                .output();
                        });
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark parallel mixed operations (read/write/search).
#[cfg(feature = "fuse")]
fn bench_fuse_parallel_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuse/parallel_mixed");
    group.sample_size(10);

    let rg_available = Command::new("rg").arg("--version").output().is_ok();

    for backend in available_backends() {
        for &thread_count in &[2, 4, 8] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, format!("{}_threads", thread_count)),
                &(backend.backend_type, thread_count),
                |b, &(backend_type, threads)| {
                    b.iter_custom(|iters| {
                        let mut total = std::time::Duration::ZERO;

                        for iter in 0..iters {
                            let vault = FuseTestVault::new(backend_type);
                            let bench_dir = vault.mountpoint.join("bench");
                            let mut gen = DataGenerator::new(42);

                            // Pre-populate with searchable files
                            for i in 0..50 {
                                let content =
                                    gen.searchable_text(1024, &["findme", "searchterm", "keyword"]);
                                std::fs::write(bench_dir.join(format!("file_{}.txt", i)), content)
                                    .unwrap();
                            }

                            let counter = AtomicU64::new(iter * 10000);
                            let bench_path = bench_dir.to_str().unwrap().to_string();

                            let start = std::time::Instant::now();

                            // Mixed parallel operations
                            (0..threads).into_par_iter().for_each(|t| {
                                let mut gen = DataGenerator::new(t as u64 + iter * 1000);
                                let ops_per_thread = 20;

                                for i in 0..ops_per_thread {
                                    match i % 4 {
                                        0 => {
                                            // Write
                                            let id = counter.fetch_add(1, Ordering::Relaxed);
                                            let path =
                                                bench_dir.join(format!("new_{}_{}.txt", t, id));
                                            std::fs::write(&path, gen.content(512)).unwrap();
                                        }
                                        1 => {
                                            // Read
                                            let path =
                                                bench_dir.join(format!("file_{}.txt", i % 50));
                                            let _ = std::fs::read(&path);
                                        }
                                        2 => {
                                            // List
                                            let _ = std::fs::read_dir(&bench_dir).unwrap().count();
                                        }
                                        3 if rg_available => {
                                            // Search (if ripgrep available)
                                            let _ = Command::new("rg")
                                                .args(["--no-ignore", "-l", "findme", &bench_path])
                                                .output();
                                        }
                                        _ => {}
                                    }
                                }
                            });

                            total += start.elapsed();
                        }

                        total
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark FUSE directory listing.
#[cfg(feature = "fuse")]
fn bench_fuse_readdir(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuse/readdir");
    group.sample_size(10);

    for backend in available_backends() {
        for &file_count in &[10, 100] {
            group.bench_with_input(
                BenchmarkId::new(backend.name, file_count),
                &(backend.backend_type, file_count),
                |b, &(backend_type, count)| {
                    let vault = FuseTestVault::new(backend_type);
                    let test_dir = vault.mountpoint.join("bench/listing");
                    std::fs::create_dir_all(&test_dir).unwrap();

                    let mut gen = DataGenerator::new(42);
                    for i in 0..count {
                        std::fs::write(test_dir.join(format!("f_{}.txt", i)), gen.content(100))
                            .unwrap();
                    }

                    b.iter(|| std::fs::read_dir(&test_dir).unwrap().count());
                },
            );
        }
    }

    group.finish();
}

/// Benchmark FUSE metadata operations (stat/getattr).
#[cfg(feature = "fuse")]
fn bench_fuse_stat(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuse/stat");
    group.sample_size(10);

    for backend in available_backends() {
        group.bench_with_input(
            BenchmarkId::new(backend.name, "getattr"),
            &backend.backend_type,
            |b, &backend_type| {
                let vault = FuseTestVault::new(backend_type);
                let file_path = vault.mountpoint.join("bench/statfile.txt");
                std::fs::write(&file_path, b"test content for stat benchmark").unwrap();

                b.iter(|| std::fs::metadata(&file_path));
            },
        );
    }

    group.finish();
}

// FUSE benchmark group
#[cfg(feature = "fuse")]
criterion_group! {
    name = fuse_operations;
    config = configure_criterion_long();
    targets =
        bench_fuse_parallel_write,
        bench_fuse_parallel_read,
        bench_fuse_ripgrep,
        bench_fuse_parallel_ripgrep,
        bench_fuse_parallel_mixed,
        bench_fuse_readdir,
        bench_fuse_stat
}

#[cfg(feature = "fuse")]
criterion_main!(
    single_vault_throughput,
    single_vault_concurrent,
    multi_vault_scale,
    fuse_operations
);

#[cfg(not(feature = "fuse"))]
criterion_main!(
    single_vault_throughput,
    single_vault_concurrent,
    multi_vault_scale
);
