//! tree command - display directory tree.

use clap::Args;
use serde::Serialize;

use crate::commands::Output;
use crate::error::Result;
use crate::fs::{FileSystem, FileType, TreeNode};
use crate::vault::VaultManager;

#[derive(Args)]
pub struct TreeArgs {
    /// Path to display tree from (defaults to /)
    #[arg(default_value = "/")]
    pub path: String,

    /// Maximum depth to display
    #[arg(short = 'L', long)]
    pub level: Option<usize>,
}

#[derive(Serialize)]
struct TreeJsonNode {
    name: String,
    #[serde(rename = "type")]
    file_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<TreeJsonNode>,
}

impl From<&TreeNode> for TreeJsonNode {
    fn from(node: &TreeNode) -> Self {
        TreeJsonNode {
            name: if node.name.is_empty() {
                "/".to_string()
            } else {
                node.name.clone()
            },
            file_type: match node.file_type {
                FileType::Directory => "directory".to_string(),
                FileType::File => "file".to_string(),
            },
            size: if node.file_type.is_file() {
                Some(node.size)
            } else {
                None
            },
            children: node.children.iter().map(TreeJsonNode::from).collect(),
        }
    }
}

pub fn run(args: TreeArgs, output: &Output, vault: Option<String>) -> Result<()> {
    let manager = VaultManager::new()?;
    let backend = match vault {
        Some(name) => manager.open(&name)?,
        None => manager.open_current()?,
    };

    let fs = FileSystem::new(backend);
    let tree = fs.tree(&args.path, args.level)?;

    if output.is_json() {
        let json_tree = TreeJsonNode::from(&tree);
        output.print_json(&json_tree);
    } else {
        print!("{}", tree.format("", true, true));
    }

    Ok(())
}
