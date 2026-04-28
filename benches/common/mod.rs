//! Shared benchmark utilities.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

use agentvfs::fs::FileSystem;
use agentvfs::storage::{BackendType, VaultBackend};

pub mod fixtures;
pub mod metrics;
pub mod workload;

/// Backend configuration for parameterized benchmarks.
#[derive(Clone, Copy, Debug)]
pub struct BackendConfig {
    pub backend_type: BackendType,
    pub name: &'static str,
}

/// Get all available backends for benchmarking.
pub fn available_backends() -> Vec<BackendConfig> {
    #[allow(unused_mut)]
    let mut backends = vec![BackendConfig {
        backend_type: BackendType::Sqlite,
        name: "sqlite",
    }];

    #[cfg(feature = "sled-backend")]
    backends.push(BackendConfig {
        backend_type: BackendType::Sled,
        name: "sled",
    });

    #[cfg(feature = "lmdb-backend")]
    backends.push(BackendConfig {
        backend_type: BackendType::Lmdb,
        name: "lmdb",
    });

    backends
}

/// Test vault wrapper with automatic cleanup.
pub struct TestVault {
    pub fs: FileSystem,
    pub backend: Arc<VaultBackend>,
    _temp_dir: TempDir,
}

impl TestVault {
    /// Create a new test vault with the specified backend.
    pub fn new(backend_type: BackendType) -> Self {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir
            .path()
            .join(format!("bench.{}", backend_type.extension()));
        let backend =
            Arc::new(VaultBackend::open(&db_path, backend_type).expect("failed to open vault"));
        let fs = FileSystem::new(Arc::clone(&backend));
        Self {
            fs,
            backend,
            _temp_dir: temp_dir,
        }
    }

    /// Get the path to the vault database.
    pub fn path(&self) -> PathBuf {
        self._temp_dir.path().to_path_buf()
    }
}

/// Multi-vault pool for scale testing.
pub struct VaultPool {
    pub vaults: Vec<TestVault>,
    _temp_dir: TempDir,
}

impl VaultPool {
    /// Create a pool of test vaults.
    pub fn new(count: usize, backend_type: BackendType) -> Self {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let vaults = (0..count)
            .map(|i| {
                let db_path = temp_dir
                    .path()
                    .join(format!("vault_{}.{}", i, backend_type.extension()));
                let backend = Arc::new(
                    VaultBackend::open(&db_path, backend_type).expect("failed to open vault"),
                );
                let fs = FileSystem::new(Arc::clone(&backend));
                // Note: Each vault doesn't need its own TempDir since they're all in the pool's temp dir
                TestVault {
                    fs,
                    backend,
                    _temp_dir: TempDir::new().expect("failed to create temp dir"),
                }
            })
            .collect();

        Self {
            vaults,
            _temp_dir: temp_dir,
        }
    }

    /// Get the number of vaults in the pool.
    pub fn len(&self) -> usize {
        self.vaults.len()
    }

    /// Check if the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.vaults.is_empty()
    }
}

// ============================================================================
// FUSE Test Vault (requires fuse feature)
// ============================================================================

#[cfg(feature = "fuse")]
use agentvfs::mount::VfsFilesystem;
#[cfg(feature = "fuse")]
use fuser::MountOption;
#[cfg(feature = "fuse")]
use std::path::Path;
#[cfg(feature = "fuse")]
use std::process::Command;
#[cfg(feature = "fuse")]
use std::thread;
#[cfg(feature = "fuse")]
use std::time::Duration;

/// FUSE-mounted test vault with automatic mount/unmount.
#[cfg(feature = "fuse")]
pub struct FuseTestVault {
    /// Path to the mountpoint directory.
    pub mountpoint: PathBuf,
    /// Temp directory holding the vault and mountpoint.
    _vault_dir: TempDir,
    /// Handle to the mount thread.
    _mount_thread: Option<thread::JoinHandle<()>>,
}

#[cfg(feature = "fuse")]
impl FuseTestVault {
    /// Create a new FUSE-mounted test vault.
    pub fn new(backend_type: BackendType) -> Self {
        // Create vault in temp dir
        let vault_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = vault_dir
            .path()
            .join(format!("bench.{}", backend_type.extension()));

        // Initialize vault with a /bench directory
        {
            let backend =
                Arc::new(VaultBackend::open(&db_path, backend_type).expect("failed to open vault"));
            let fs = FileSystem::new(backend);
            fs.create_dir("/bench").expect("failed to create /bench");
        }

        // Create mountpoint directory
        let mountpoint = vault_dir.path().join("mnt");
        std::fs::create_dir(&mountpoint).expect("failed to create mountpoint");

        // Clone paths for the mount thread
        let db_path_clone = db_path.clone();
        let mountpoint_clone = mountpoint.clone();

        // Spawn mount thread (mount2 blocks until unmount)
        let mount_thread = thread::spawn(move || {
            let backend = Arc::new(
                VaultBackend::open(&db_path_clone, backend_type).expect("failed to open vault"),
            );
            let fs = FileSystem::new(backend);
            let vfs_fs = VfsFilesystem::new(fs, false);
            let options = vec![MountOption::FSName("bench".to_string())];
            // This blocks until unmount
            let _ = fuser::mount2(vfs_fs, &mountpoint_clone, &options);
        });

        // Wait for mount to be ready
        Self::wait_for_mount(&mountpoint);

        Self {
            mountpoint,
            _vault_dir: vault_dir,
            _mount_thread: Some(mount_thread),
        }
    }

    /// Wait for the FUSE mount to become ready.
    fn wait_for_mount(mountpoint: &Path) {
        for _ in 0..50 {
            // Check if the /bench directory is visible
            if mountpoint.join("bench").exists() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("FUSE mount did not become ready within 5 seconds");
    }
}

#[cfg(feature = "fuse")]
impl Drop for FuseTestVault {
    fn drop(&mut self) {
        // Unmount the filesystem
        let _ = Command::new("fusermount")
            .args(["-u", self.mountpoint.to_str().unwrap()])
            .output();

        // Give the unmount a moment to complete
        thread::sleep(Duration::from_millis(100));
    }
}
