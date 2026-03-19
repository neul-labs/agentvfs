//! Virtual filesystem operations.

mod entry;
mod ops;
pub mod path;

pub use entry::{ContentBlob, DirEntry, FileEntry, FileType, FileVersion, Metadata, SearchResult, Tag};
pub use ops::{FileSystem, TreeNode};
