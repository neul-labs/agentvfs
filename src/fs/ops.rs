//! Filesystem operations.

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{Result, VfsError};
use crate::fs::entry::{DirEntry, FileEntry, FileType};
use crate::fs::path;
use crate::storage::VaultBackend;

/// Virtual filesystem operations.
pub struct FileSystem {
    backend: Arc<VaultBackend>,
}

impl FileSystem {
    /// Create a new filesystem with the given storage backend.
    pub fn new(backend: Arc<VaultBackend>) -> Self {
        Self { backend }
    }

    /// Get the storage backend.
    pub fn backend(&self) -> &Arc<VaultBackend> {
        &self.backend
    }

    /// Resolve a path to a file entry.
    pub fn get_entry(&self, vpath: &str) -> Result<FileEntry> {
        let normalized = path::normalize(vpath)?;
        self.backend.get_entry_by_path(&normalized)
    }

    /// List contents of a directory.
    pub fn list_dir(&self, vpath: &str) -> Result<Vec<DirEntry>> {
        let normalized = path::normalize(vpath)?;
        let entry = self.backend.get_entry_by_path(&normalized)?;

        if !entry.is_dir() {
            return Err(VfsError::NotADirectory(PathBuf::from(normalized)));
        }

        let children = self.backend.list_children(&normalized)?;
        Ok(children.iter().map(DirEntry::from).collect())
    }

    /// Read file contents.
    pub fn read_file(&self, vpath: &str) -> Result<Vec<u8>> {
        let normalized = path::normalize(vpath)?;
        let entry = self.backend.get_entry_by_path(&normalized)?;

        if !entry.is_file() {
            return Err(VfsError::NotAFile(PathBuf::from(normalized)));
        }

        let hash = entry
            .content_hash
            .ok_or_else(|| VfsError::Internal("file has no content hash".to_string()))?;

        self.backend.read_content(&hash)
    }

    /// Write file contents.
    ///
    /// Creates the file if it doesn't exist, or updates it if it does.
    /// Automatically creates a version snapshot on every write.
    pub fn write_file(&self, vpath: &str, content: &[u8]) -> Result<()> {
        let normalized = path::normalize(vpath)?;
        let (parent_path, name) = path::split(&normalized)?;
        let parent_path =
            parent_path.ok_or_else(|| VfsError::InvalidPath("cannot write to root".to_string()))?;

        if name.is_empty() {
            return Err(VfsError::InvalidPath("cannot write to root".to_string()));
        }

        // Ensure parent exists and is a directory
        let parent = self.backend.get_entry_by_path(&parent_path)?;
        if !parent.is_dir() {
            return Err(VfsError::NotADirectory(PathBuf::from(&parent_path)));
        }

        // Use backend-specific atomic path when available
        match self.backend.as_ref() {
            VaultBackend::Sqlite(backend) => {
                backend.write_file_atomic(parent.id, &name, content, &normalized)?;
                Ok(())
            }
            #[cfg(feature = "sled-backend")]
            VaultBackend::Sled(backend) => {
                let hash = backend.write_content(content)?;
                let size = content.len() as u64;
                if let Some(file_id) = backend.get_file_id(parent.id, &name)? {
                    let current = backend.get_entry_by_id(file_id)?.ok_or_else(|| {
                        VfsError::Internal(format!("file entry not found: id={file_id}"))
                    })?;
                    if let Some(current_hash) = current.content_hash {
                        backend.create_version(file_id, current_hash, current.size)?;
                    }
                    backend.update_file(file_id, hash, size)?;
                } else {
                    let file_id = backend.create_file(parent.id, &name, hash, size)?;
                    backend.create_version(file_id, hash, size)?;
                }
                // Sled manages its own search index
                Ok(())
            }
            #[cfg(feature = "lmdb-backend")]
            VaultBackend::Lmdb(backend) => {
                let hash = backend.write_content(content)?;
                let size = content.len() as u64;
                if let Some(file_id) = backend.get_file_id(parent.id, &name)? {
                    let current = backend.get_entry_by_id(file_id)?.ok_or_else(|| {
                        VfsError::Internal(format!("file entry not found: id={file_id}"))
                    })?;
                    if let Some(current_hash) = current.content_hash {
                        backend.create_version(file_id, current_hash, current.size)?;
                    }
                    backend.update_file(file_id, hash, size)?;
                } else {
                    let file_id = backend.create_file(parent.id, &name, hash, size)?;
                    backend.create_version(file_id, hash, size)?;
                }
                // LMDB manages its own search index
                Ok(())
            }
        }
    }

    /// Create a directory.
    pub fn create_dir(&self, vpath: &str) -> Result<()> {
        let normalized = path::normalize(vpath)?;
        let (parent_path, name) = path::split(&normalized)?;
        let parent_path =
            parent_path.ok_or_else(|| VfsError::InvalidPath("cannot create root".to_string()))?;

        if name.is_empty() {
            return Err(VfsError::InvalidPath("cannot create root".to_string()));
        }

        // Ensure parent exists and is a directory
        let parent = self.backend.get_entry_by_path(&parent_path)?;
        if !parent.is_dir() {
            return Err(VfsError::NotADirectory(PathBuf::from(&parent_path)));
        }

        // Use backend-specific atomic path when available
        match self.backend.as_ref() {
            VaultBackend::Sqlite(backend) => {
                backend.create_directory_atomic(parent.id, &name, &normalized)?;
                Ok(())
            }
            #[cfg(feature = "sled-backend")]
            VaultBackend::Sled(backend) => {
                if backend.name_exists(parent.id, &name)? {
                    return Err(VfsError::AlreadyExists(PathBuf::from(&normalized)));
                }
                backend.create_directory(parent.id, &name)?;
                Ok(())
            }
            #[cfg(feature = "lmdb-backend")]
            VaultBackend::Lmdb(backend) => {
                if backend.name_exists(parent.id, &name)? {
                    return Err(VfsError::AlreadyExists(PathBuf::from(&normalized)));
                }
                backend.create_directory(parent.id, &name)?;
                Ok(())
            }
        }
    }

    /// Create a directory and all parent directories.
    pub fn create_dir_all(&self, vpath: &str) -> Result<()> {
        let components = path::components(vpath)?;
        let mut current_path = String::new();

        for component in components {
            current_path = if current_path.is_empty() {
                format!("/{}", component)
            } else {
                format!("{}/{}", current_path, component)
            };

            // Try to create, ignore if exists
            match self.create_dir(&current_path) {
                Ok(()) => {}
                Err(VfsError::AlreadyExists(_)) => {}
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// Remove a file or directory.
    ///
    /// If recursive is true, removes directory contents recursively.
    pub fn remove(&self, vpath: &str, recursive: bool) -> Result<()> {
        let normalized = path::normalize(vpath)?;

        if path::is_root(&normalized) {
            return Err(VfsError::InvalidPath("cannot remove root".to_string()));
        }

        let entry = self.backend.get_entry_by_path(&normalized)?;

        if entry.is_dir() && !recursive {
            // Check if directory has contents
            if self.backend.has_children(entry.id)? {
                return Err(VfsError::NotEmpty(PathBuf::from(&normalized)));
            }
        }

        self.remove_index_subtree(&normalized)?;

        // Use backend-specific atomic path when available
        match self.backend.as_ref() {
            VaultBackend::Sqlite(backend) => {
                backend.delete_entry_atomic(entry.id, &normalized)?;
            }
            #[cfg(feature = "sled-backend")]
            VaultBackend::Sled(backend) => {
                backend.delete_entry(entry.id, recursive)?;
            }
            #[cfg(feature = "lmdb-backend")]
            VaultBackend::Lmdb(backend) => {
                backend.delete_entry(entry.id, recursive)?;
            }
        }

        Ok(())
    }

    /// Copy a file or directory.
    pub fn copy(&self, src: &str, dst: &str) -> Result<()> {
        let src_normalized = path::normalize(src)?;
        let dst_normalized = path::normalize(dst)?;

        let src_entry = self.backend.get_entry_by_path(&src_normalized)?;

        if src_entry.is_file() {
            self.copy_file(&src_entry, &dst_normalized)
        } else {
            self.copy_dir(&src_normalized, &dst_normalized)
        }
    }

    /// Copy a single file.
    fn copy_file(&self, src_entry: &FileEntry, dst_path: &str) -> Result<()> {
        let (parent_path, mut name) = path::split(dst_path)?;
        let mut parent_path =
            parent_path.ok_or_else(|| VfsError::InvalidPath("cannot copy to root".to_string()))?;

        // Check if destination is a directory
        if let Ok(dst_entry) = self.backend.get_entry_by_path(dst_path) {
            if dst_entry.is_dir() {
                // Copy into directory with same name
                parent_path = dst_path.to_string();
                name = src_entry.name.clone();
            } else {
                return Err(VfsError::AlreadyExists(PathBuf::from(dst_path)));
            }
        }

        let parent = self.backend.get_entry_by_path(&parent_path)?;
        if !parent.is_dir() {
            return Err(VfsError::NotADirectory(PathBuf::from(&parent_path)));
        }

        let new_path = path::join(&parent_path, &name)?;

        match self.backend.as_ref() {
            VaultBackend::Sqlite(backend) => {
                backend.copy_file_atomic(src_entry, parent.id, &name, &new_path)?;
            }
            #[cfg(any(feature = "sled-backend", feature = "lmdb-backend"))]
            _ => {
                self.backend
                    .copy_file(src_entry, parent.id, &name, &new_path)?;
            }
        }

        self.backend.sync_file_index(&new_path)?;

        Ok(())
    }

    /// Copy a directory recursively.
    fn copy_dir(&self, src_path: &str, dst_path: &str) -> Result<()> {
        let src_entry = self.backend.get_entry_by_path(src_path)?;
        let (parent_path, mut name) = path::split(dst_path)?;
        let mut parent_path =
            parent_path.ok_or_else(|| VfsError::InvalidPath("cannot copy to root".to_string()))?;

        // Check if destination is a directory
        if let Ok(dst_entry) = self.backend.get_entry_by_path(dst_path) {
            if dst_entry.is_dir() {
                // Copy into directory
                parent_path = dst_path.to_string();
                name = src_entry.name.clone();
            } else {
                return Err(VfsError::AlreadyExists(PathBuf::from(dst_path)));
            }
        }

        let parent = self.backend.get_entry_by_path(&parent_path)?;
        if !parent.is_dir() {
            return Err(VfsError::NotADirectory(PathBuf::from(&parent_path)));
        }

        let new_dir_path = path::join(&parent_path, &name)?;
        if path_is_same_or_descendant(src_path, &new_dir_path) {
            return Err(VfsError::InvalidPath(format!(
                "cannot copy '{}' into itself or its descendant '{}'",
                src_path, new_dir_path
            )));
        }

        // Create new directory
        self.backend
            .create_directory(parent.id, &name, &new_dir_path)?;
        let new_dir = self.backend.get_entry_by_path(&new_dir_path)?;

        // Copy children
        let children = self.backend.list_children(src_path)?;
        for child in children {
            let child_src_path = path::join(src_path, &child.name)?;
            let child_dst_path = path::join(&new_dir_path, &child.name)?;

            if child.is_dir() {
                self.copy_dir(&child_src_path, &child_dst_path)?;
            } else {
                match self.backend.as_ref() {
                    VaultBackend::Sqlite(backend) => {
                        backend.copy_file_atomic(
                            &child,
                            new_dir.id,
                            &child.name,
                            &child_dst_path,
                        )?;
                    }
                    #[cfg(any(feature = "sled-backend", feature = "lmdb-backend"))]
                    _ => {
                        self.backend
                            .copy_file(&child, new_dir.id, &child.name, &child_dst_path)?;
                    }
                }
                self.backend.sync_file_index(&child_dst_path)?;
            }
        }

        Ok(())
    }

    /// Move/rename a file or directory.
    pub fn move_entry(&self, src: &str, dst: &str) -> Result<()> {
        let src_normalized = path::normalize(src)?;
        let dst_normalized = path::normalize(dst)?;

        if path::is_root(&src_normalized) {
            return Err(VfsError::InvalidPath("cannot move root".to_string()));
        }

        let src_entry = self.backend.get_entry_by_path(&src_normalized)?;

        let (parent_path, mut name) = path::split(&dst_normalized)?;
        let mut parent_path =
            parent_path.ok_or_else(|| VfsError::InvalidPath("cannot move to root".to_string()))?;

        // Check if destination is a directory (move into it)
        if let Ok(dst_entry) = self.backend.get_entry_by_path(&dst_normalized) {
            if dst_entry.is_dir() {
                parent_path = dst_normalized.clone();
                name = src_entry.name.clone();
            } else {
                return Err(VfsError::AlreadyExists(PathBuf::from(&dst_normalized)));
            }
        }

        let parent = self.backend.get_entry_by_path(&parent_path)?;
        if !parent.is_dir() {
            return Err(VfsError::NotADirectory(PathBuf::from(&parent_path)));
        }

        // Check if destination name already exists
        if self.backend.name_exists(parent.id, &name)? {
            return Err(VfsError::AlreadyExists(PathBuf::from(path::join(
                &parent_path,
                &name,
            )?)));
        }

        let new_path = path::join(&parent_path, &name)?;
        if src_entry.is_dir() && path_is_same_or_descendant(&src_normalized, &new_path) {
            return Err(VfsError::InvalidPath(format!(
                "cannot move '{}' into itself or its descendant '{}'",
                src_normalized, new_path
            )));
        }

        // Use backend-specific atomic path when available
        match self.backend.as_ref() {
            VaultBackend::Sqlite(backend) => {
                backend.move_entry_atomic(
                    src_entry.id,
                    parent.id,
                    &name,
                    &src_normalized,
                    &new_path,
                )?;
            }
            #[cfg(any(feature = "sled-backend", feature = "lmdb-backend"))]
            _ => {
                self.backend.move_entry(
                    src_entry.id,
                    parent.id,
                    &name,
                    &src_normalized,
                    &new_path,
                )?;

                if src_entry.is_dir() {
                    self.backend.rebuild_child_paths(src_entry.id, &new_path)?;
                }
            }
        }

        self.sync_index_subtree(&new_path)?;

        Ok(())
    }

    fn remove_index_subtree(&self, vpath: &str) -> Result<()> {
        let entry = self.backend.get_entry_by_path(vpath)?;
        if entry.is_file() {
            self.backend.remove_from_index(vpath)?;
            return Ok(());
        }

        for child in self.backend.list_children(vpath)? {
            let child_path = path::join(vpath, &child.name)?;
            self.remove_index_subtree(&child_path)?;
        }

        Ok(())
    }

    fn sync_index_subtree(&self, vpath: &str) -> Result<()> {
        let entry = self.backend.get_entry_by_path(vpath)?;
        if entry.is_file() {
            self.backend.sync_file_index(vpath)?;
            return Ok(());
        }

        for child in self.backend.list_children(vpath)? {
            let child_path = path::join(vpath, &child.name)?;
            self.sync_index_subtree(&child_path)?;
        }

        Ok(())
    }

    /// Check if a path exists.
    pub fn exists(&self, vpath: &str) -> Result<bool> {
        match self.get_entry(vpath) {
            Ok(_) => Ok(true),
            Err(VfsError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get a tree structure of a directory.
    pub fn tree(&self, vpath: &str, max_depth: Option<usize>) -> Result<TreeNode> {
        let normalized = path::normalize(vpath)?;
        let entry = self.backend.get_entry_by_path(&normalized)?;

        self.build_tree(&entry, &normalized, 0, max_depth)
    }

    fn build_tree(
        &self,
        entry: &FileEntry,
        vpath: &str,
        depth: usize,
        max_depth: Option<usize>,
    ) -> Result<TreeNode> {
        let children = if entry.is_dir() && max_depth.is_none_or(|m| depth < m) {
            let entries = self.backend.list_children(vpath)?;
            let mut children = Vec::new();
            for child in entries {
                let child_path = path::join(vpath, &child.name)?;
                children.push(self.build_tree(&child, &child_path, depth + 1, max_depth)?);
            }
            children
        } else {
            Vec::new()
        };

        Ok(TreeNode {
            name: entry.name.clone(),
            file_type: entry.file_type,
            size: entry.size,
            children,
        })
    }
}

/// Tree node for directory structure.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub file_type: FileType,
    pub size: u64,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// Format tree as text.
    pub fn format(&self, prefix: &str, is_last: bool, is_root: bool) -> String {
        let mut result = String::new();

        if is_root {
            result.push_str(&self.name);
            if self.name.is_empty() {
                result.push('/');
            }
            result.push('\n');
        } else {
            let connector = if is_last { "└── " } else { "├── " };
            result.push_str(prefix);
            result.push_str(connector);
            result.push_str(&self.name);
            if self.file_type.is_dir() {
                result.push('/');
            }
            result.push('\n');
        }

        let child_prefix = if is_root {
            String::new()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        for (i, child) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;
            result.push_str(&child.format(&child_prefix, is_last_child, false));
        }

        result
    }
}

fn path_is_same_or_descendant(root: &str, candidate: &str) -> bool {
    candidate == root
        || candidate
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::FileSystem;
    use crate::storage::{BackendType, VaultBackend};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn binary_rewrite_removes_stale_search_results() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.avfs");
        let backend = Arc::new(VaultBackend::open(&db_path, BackendType::Sqlite).unwrap());
        let fs = FileSystem::new(backend.clone());

        fs.create_dir("/docs").unwrap();
        fs.write_file("/docs/file.txt", b"hello world").unwrap();
        assert_eq!(backend.search_content("hello", 10).unwrap().len(), 1);

        fs.write_file("/docs/file.txt", &[0xff, 0xfe, 0xfd])
            .unwrap();
        assert!(backend.search_content("hello", 10).unwrap().is_empty());
    }
}
