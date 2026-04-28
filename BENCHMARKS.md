# Benchmarks

All benchmarks run with Criterion.rs on Apple Silicon (M3 Pro, macOS 15.4). Results are median values across 100 samples (or 10 for longer workloads).

Run them yourself:

```bash
# SQLite only (default)
cargo bench

# All backends
cargo bench --features "sled-backend,lmdb-backend"

# Specific group
cargo bench -- single_vault
cargo bench -- multi_vault
```

---

## Single Vault Throughput

### Write (per file)

| Backend | 1 KB | 64 KB | 1 MB |
|---------|------|-------|------|
| **SQLite** | 106 µs (9.2 MiB/s) | 502 µs (124.6 MiB/s) | 6.1 ms (163.6 MiB/s) |
| **Sled** | 81 µs (12.0 MiB/s) | 4.1 ms (15.1 MiB/s) | 56.6 ms (17.7 MiB/s) |
| **LMDB** | 20.1 ms (49.7 KiB/s) | 25.0 ms (2.5 MiB/s) | 84.5 ms (11.8 MiB/s) |

**Takeaway:** SQLite scales well with file size due to batched WAL writes. Sled is fastest for small files but drops off at 64 KB+. LMDB pays a heavy sync cost per write in this configuration.

### Read (per file)

| Backend | 1 KB | 64 KB | 1 MB |
|---------|------|-------|------|
| **SQLite** | 8.9 µs (109 MiB/s) | 22.5 µs (2.7 GiB/s) | 215 µs (4.5 GiB/s) |
| **Sled** | 15.3 µs (63.8 MiB/s) | 835 µs (74.8 MiB/s) | 13.0 ms (76.6 MiB/s) |
| **LMDB** | — | — | — |

**Takeaway:** SQLite reads are extremely fast once the page cache is warm. Sled is competitive for small reads but slower for large files due to tree traversal overhead.

### Directory Listing

| Backend | 10 files | 100 files | 1000 files |
|---------|----------|-----------|------------|
| **SQLite** | 22.0 µs | 61.7 µs | 474 µs |

**Takeaway:** Listing scales sub-linearly; SQLite's B-tree index keeps it under half a millisecond even at 1000 entries.

### Full-Text Search (FTS)

| Backend | 100 docs queried |
|---------|------------------|
| **SQLite** | 907 µs |

**Takeaway:** FTS5 search across 100 documents with mixed keywords returns in under a millisecond.

---

## Concurrent Agent Workloads

Mixed workload (30% write, 35% read, 15% list, 10% search, 5% delete, 3% copy, 2% move), 100 ops per agent.

| Agents | SQLite latency | Total ops | Effective ops/sec |
|--------|---------------|-----------|-----------------|
| 1 | 12.3 ms | 100 | ~8,130 |
| 2 | 21.6 ms | 200 | ~9,260 |
| 4 | 40.8 ms | 400 | ~9,800 |
| 8 | 80.3 ms | 800 | ~9,960 |
| 16 | 168.6 ms | 1,600 | ~9,490 |

**Takeaway:** Throughput scales nearly linearly up to 8 agents. At 16 agents, contention on the SQLite connection mutex causes diminishing returns. Atomic transactions prevent corruption but serialize writers.

### Workload Profiles (4 agents, 200 ops each)

| Profile | SQLite latency | Effective ops/sec |
|---------|---------------|-------------------|
| Read-heavy (80% read) | 39.2 ms | ~20,400 |
| Write-heavy (60% write) | 182.7 ms | ~4,380 |
| Mixed (default) | — | — |

**Takeaway:** Read-heavy workloads are ~4.7x faster than write-heavy ones on SQLite. This is expected for a WAL-mode database with a single writer lock.

---

## Multi-Vault Scale

### Vault Creation

| Vaults | SQLite time | Per-vault |
|--------|------------|-----------|
| 10 | 33.9 ms | 3.4 ms |
| 100 | 341.6 ms | 3.4 ms |
| 500 | 1.71 s | 3.4 ms |

**Takeaway:** Creation is perfectly linear—no overhead from shared state since each vault is an independent file.

### Parallel Operations Across Vault Pool

| Vaults | Time (create + write + read + list) |
|--------|-------------------------------------|
| 10 | 733 µs |
| 50 | 3.52 ms |
| 100 | 7.72 ms |

**Takeaway:** Operations across independent vaults scale linearly and benefit from parallelism (via Rayon).

### Agent Simulation Across Vaults

| Configuration | Time | Total ops |
|---------------|------|-----------|
| 50 vaults × 2 agents | 347.6 ms | 5,000 |
| 100 vaults × 1 agent | 365.5 ms | 5,000 |
| 200 vaults × 1 agent | 669.5 ms | 10,000 |

**Takeaway:** 100 isolated vaults with 1 agent each completes in roughly the same time as 50 vaults with 2 agents each, demonstrating good vault isolation.

### Search Across Vaults

| Vaults | Time (FTS search per vault) |
|--------|----------------------------|
| 10 | 2.43 ms |
| 50 | 12.81 ms |

**Takeaway:** Parallel search scales linearly with vault count. Each vault's index is independent.

### Scaling Behavior

Fixed work per vault (10 writes + 10 reads), increasing vault count:

| Vaults | Total Time | Work Done |
|--------|-----------|-----------|
| 1 | 1.45 ms | 20 ops |
| 10 | 12.0 ms | 200 ops |
| 25 | 30.4 ms | 500 ops |
| 50 | 62.3 ms | 1,000 ops |
| 100 | 121.0 ms | 2,000 ops |

**Takeaway:** Total throughput holds steady at ~16,500 ops/sec as vault count increases, confirming independent vaults don't contend.

---

## Backend Comparison Summary

| Metric | SQLite | Sled | LMDB |
|--------|--------|------|------|
| Small-file write | Good | **Best** | Slow |
| Large-file write | **Best** | Good | Good |
| Read (all sizes) | **Best** | Moderate | — |
| Listing | **Best** | — | — |
| Search | **Best** | — | — |
| Concurrency | Serialized writers | Serialized writers | Serialized writers |
| Atomic transactions | Yes | No | No |
| Production recommendation | **Recommended** | Experimental | Experimental |

**Notes:**
- SQLite is the default and recommended backend for production use. It offers the best read performance, sub-millisecond search, and ACID atomic transactions via WAL mode.
- Sled is faster for small writes but lacks atomic multi-step transactions and has higher read latency for large files.
- LMDB's current configuration shows poor small-write performance due to per-transaction sync overhead; it may improve with batched writes or `MDB_NOSYNC` tuning.
- All backends use the same `Arc<Mutex<>>` concurrency model in the current implementation; the backend comparison reflects storage engine efficiency, not architectural differences.

---

## Methodology

- **Hardware:** Apple M3 Pro, 36 GB RAM, macOS 15.4
- **Tooling:** Criterion.rs 0.5 with plotters backend
- **Warming:** 1–2 seconds before measurement
- **Samples:** 100 for micro-benchmarks, 10 for workload simulations
- **Data:** Deterministic seeded RNG (SmallRng, seed=42)
- **Metrics:** Median latency and throughput reported; outliers noted in raw Criterion output
