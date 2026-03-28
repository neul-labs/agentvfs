//! Integration tests for the avfs CLI.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

/// Get a command for testing avfs
fn avfs() -> Command {
    Command::cargo_bin("avfs").unwrap()
}

#[test]
fn test_version() {
    avfs()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("avfs"));
}

#[test]
fn test_help() {
    avfs()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Virtual filesystem"));
}

#[test]
fn test_vault_create_and_list() {
    let home = tempdir().unwrap();
    let avfs_dir = home.path().join(".avfs");

    // Create vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created vault"));

    // List vaults
    avfs()
        .env("HOME", home.path())
        .args(["vault", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-vault"));

    // Check vault file exists
    assert!(avfs_dir.join("vaults").join("test-vault.avfs").exists());
}

#[test]
fn test_vault_use() {
    let home = tempdir().unwrap();

    // Create vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "my-vault"])
        .assert()
        .success();

    // Use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "use", "my-vault"])
        .assert()
        .success()
        .stdout(predicate::str::contains("using vault"));
}

#[test]
fn test_write_and_cat() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write a file
    avfs()
        .env("HOME", home.path())
        .args(["write", "/hello.txt", "Hello, World!"])
        .assert()
        .success();

    // Read the file
    avfs()
        .env("HOME", home.path())
        .args(["cat", "/hello.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, World!"));
}

#[test]
fn test_mkdir_and_ls() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Create directory
    avfs()
        .env("HOME", home.path())
        .args(["mkdir", "/mydir"])
        .assert()
        .success();

    // List root
    avfs()
        .env("HOME", home.path())
        .args(["ls", "/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mydir"));
}

#[test]
fn test_cp_and_mv() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write a file
    avfs()
        .env("HOME", home.path())
        .args(["write", "/original.txt", "Test content"])
        .assert()
        .success();

    // Copy file
    avfs()
        .env("HOME", home.path())
        .args(["cp", "/original.txt", "/copy.txt"])
        .assert()
        .success();

    // Verify copy exists
    avfs()
        .env("HOME", home.path())
        .args(["cat", "/copy.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Test content"));

    // Move file
    avfs()
        .env("HOME", home.path())
        .args(["mv", "/copy.txt", "/moved.txt"])
        .assert()
        .success();

    // Verify moved file exists
    avfs()
        .env("HOME", home.path())
        .args(["cat", "/moved.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Test content"));
}

#[test]
fn test_mv_directory_into_descendant_fails_without_corrupting_vault() {
    let home = tempdir().unwrap();

    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["mkdir", "/a"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["mkdir", "/a/b"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["mv", "/a", "/a/b"])
        .assert()
        .failure();

    avfs()
        .env("HOME", home.path())
        .args(["ls", "/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a"));

    avfs()
        .env("HOME", home.path())
        .args(["ls", "/a"])
        .assert()
        .success()
        .stdout(predicate::str::contains("b"));
}

#[test]
fn test_rm() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write a file
    avfs()
        .env("HOME", home.path())
        .args(["write", "/to_delete.txt", "Delete me"])
        .assert()
        .success();

    // Delete file
    avfs()
        .env("HOME", home.path())
        .args(["rm", "/to_delete.txt"])
        .assert()
        .success();

    // Verify file is gone
    avfs()
        .env("HOME", home.path())
        .args(["cat", "/to_delete.txt"])
        .assert()
        .failure();
}

#[test]
fn test_tree() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Create some structure
    avfs()
        .env("HOME", home.path())
        .args(["mkdir", "/dir1"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["write", "/dir1/file.txt", "content"])
        .assert()
        .success();

    // Show tree
    avfs()
        .env("HOME", home.path())
        .args(["tree", "/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dir1"));
}

#[test]
fn test_versioning() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write initial version
    avfs()
        .env("HOME", home.path())
        .args(["write", "/versioned.txt", "Version 1"])
        .assert()
        .success();

    // Write second version
    avfs()
        .env("HOME", home.path())
        .args(["write", "/versioned.txt", "Version 2"])
        .assert()
        .success();

    // Check version log
    avfs()
        .env("HOME", home.path())
        .args(["log", "/versioned.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Version"));
}

#[test]
fn test_json_output() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write a file
    avfs()
        .env("HOME", home.path())
        .args(["write", "/test.txt", "content"])
        .assert()
        .success();

    // List with JSON output
    avfs()
        .env("HOME", home.path())
        .args(["--json", "ls", "/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""));
}

#[test]
fn test_search() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write files with searchable content
    avfs()
        .env("HOME", home.path())
        .args([
            "write",
            "/searchable.txt",
            "This contains unique searchterm",
        ])
        .assert()
        .success();

    // Search for content
    avfs()
        .env("HOME", home.path())
        .args(["search", "searchterm"])
        .assert()
        .success()
        .stdout(predicate::str::contains("searchable.txt"));
}

#[test]
fn test_tags() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write a file
    avfs()
        .env("HOME", home.path())
        .args(["write", "/tagged.txt", "content"])
        .assert()
        .success();

    // Add tag
    avfs()
        .env("HOME", home.path())
        .args(["tag", "/tagged.txt", "important"])
        .assert()
        .success();

    // List tags on file
    avfs()
        .env("HOME", home.path())
        .args(["tag", "--list", "/tagged.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("important"));
}

#[test]
fn test_metadata() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write a file
    avfs()
        .env("HOME", home.path())
        .args(["write", "/metafile.txt", "content"])
        .assert()
        .success();

    // Set metadata
    avfs()
        .env("HOME", home.path())
        .args(["meta", "/metafile.txt", "author", "test"])
        .assert()
        .success();

    // Get metadata
    avfs()
        .env("HOME", home.path())
        .args(["meta", "/metafile.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("author"));
}

#[test]
fn test_snapshot() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write some files
    avfs()
        .env("HOME", home.path())
        .args(["write", "/snap.txt", "snapshot content"])
        .assert()
        .success();

    // Save snapshot
    avfs()
        .env("HOME", home.path())
        .args(["snapshot", "save", "test-snap"])
        .assert()
        .success();

    // List snapshots
    avfs()
        .env("HOME", home.path())
        .args(["snapshot", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-snap"));
}

#[test]
fn test_snapshot_restore_preserves_tags_and_metadata() {
    let home = tempdir().unwrap();

    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["write", "/snap.txt", "snapshot content"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["tag", "/snap.txt", "important"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["meta", "/snap.txt", "owner", "alice"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["snapshot", "save", "full-state"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["rm", "/snap.txt"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["snapshot", "restore", "full-state"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["tag", "--list", "/snap.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("important"));

    avfs()
        .env("HOME", home.path())
        .args(["meta", "/snap.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn test_stats() {
    let home = tempdir().unwrap();

    // Create and use vault
    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    // Write a file
    avfs()
        .env("HOME", home.path())
        .args(["write", "/stats.txt", "some content"])
        .assert()
        .success();

    // Get stats
    avfs()
        .env("HOME", home.path())
        .args(["stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files"));
}

#[test]
fn test_quota_allows_deduplicated_copy() {
    let home = tempdir().unwrap();
    let data = vec![b'a'; 1024 * 1024];

    avfs()
        .env("HOME", home.path())
        .args(["vault", "create", "test-vault"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["write", "/big.bin"])
        .write_stdin(data)
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["quota", "set", "max_size_mb", "1"])
        .assert()
        .success();

    avfs()
        .env("HOME", home.path())
        .args(["cp", "/big.bin", "/big-copy.bin"])
        .assert()
        .success();
}
