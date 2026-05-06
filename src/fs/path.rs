//! Path utilities for virtual filesystem paths.

use crate::error::{Result, VfsError};

/// Normalize a virtual filesystem path.
///
/// - Converts to absolute path (prepends / if needed)
/// - Removes duplicate slashes
/// - Resolves . and .. components
/// - Removes trailing slash (except for root)
pub fn normalize(path: &str) -> Result<String> {
    if path.is_empty() {
        return Ok("/".to_string());
    }

    // Ensure path starts with /
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };

    // Split into components and resolve . and ..
    let mut components: Vec<&str> = Vec::new();

    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                if components.is_empty() {
                    // Can't go above root
                    return Err(VfsError::InvalidPath(
                        "path escapes root directory".to_string(),
                    ));
                }
                components.pop();
            }
            _ => {
                // Validate component
                validate_component(part)?;
                components.push(part);
            }
        }
    }

    if components.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", components.join("/")))
    }
}

/// Validate a path component (filename or directory name).
fn validate_component(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(VfsError::InvalidPath("empty component".to_string()));
    }

    if name.len() > 255 {
        return Err(VfsError::InvalidPath(format!(
            "component too long: {} chars (max 255)",
            name.len()
        )));
    }

    // Check for invalid characters
    for c in name.chars() {
        if c == '\0' || c == '/' {
            return Err(VfsError::InvalidPath(format!(
                "invalid character in path: {:?}",
                c
            )));
        }
    }

    Ok(())
}

/// Split a path into parent directory and filename.
///
/// Returns (parent, name). For root, returns (None, "").
pub fn split(path: &str) -> Result<(Option<String>, String)> {
    let normalized = normalize(path)?;

    if normalized == "/" {
        return Ok((None, String::new()));
    }

    match normalized.rfind('/') {
        Some(0) => {
            // Direct child of root
            Ok((Some("/".to_string()), normalized[1..].to_string()))
        }
        Some(pos) => {
            let parent = normalized[..pos].to_string();
            let name = normalized[pos + 1..].to_string();
            Ok((Some(parent), name))
        }
        None => {
            // Should not happen after normalization
            Err(VfsError::InvalidPath(normalized))
        }
    }
}

/// Join a parent path with a child name.
pub fn join(parent: &str, name: &str) -> Result<String> {
    if name.contains('/') {
        return Err(VfsError::InvalidPath(
            "child name cannot contain /".to_string(),
        ));
    }

    let parent = normalize(parent)?;

    if parent == "/" {
        normalize(&format!("/{}", name))
    } else {
        normalize(&format!("{}/{}", parent, name))
    }
}

/// Get all path components.
///
/// For "/a/b/c", returns ["a", "b", "c"].
/// For "/", returns [].
pub fn components(path: &str) -> Result<Vec<String>> {
    let normalized = normalize(path)?;

    if normalized == "/" {
        return Ok(Vec::new());
    }

    Ok(normalized[1..].split('/').map(|s| s.to_string()).collect())
}

/// Get the filename (last component) of a path.
pub fn filename(path: &str) -> Result<String> {
    let (_, name) = split(path)?;
    Ok(name)
}

/// Get the parent directory of a path.
pub fn parent(path: &str) -> Result<Option<String>> {
    let (parent, _) = split(path)?;
    Ok(parent)
}

/// Check if a path is the root directory.
pub fn is_root(path: &str) -> bool {
    matches!(normalize(path), Ok(p) if p == "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("/").unwrap(), "/");
        assert_eq!(normalize("").unwrap(), "/");
        assert_eq!(normalize("/a/b/c").unwrap(), "/a/b/c");
        assert_eq!(normalize("a/b/c").unwrap(), "/a/b/c");
        assert_eq!(normalize("/a//b///c").unwrap(), "/a/b/c");
        assert_eq!(normalize("/a/b/c/").unwrap(), "/a/b/c");
        assert_eq!(normalize("/a/./b/c").unwrap(), "/a/b/c");
        assert_eq!(normalize("/a/b/../c").unwrap(), "/a/c");
        assert_eq!(normalize("/a/b/c/..").unwrap(), "/a/b");
    }

    #[test]
    fn test_normalize_errors() {
        assert!(normalize("/..").is_err());
        assert!(normalize("/a/../..").is_err());
    }

    #[test]
    fn test_split() {
        assert_eq!(split("/").unwrap(), (None, "".to_string()));
        assert_eq!(
            split("/a").unwrap(),
            (Some("/".to_string()), "a".to_string())
        );
        assert_eq!(
            split("/a/b").unwrap(),
            (Some("/a".to_string()), "b".to_string())
        );
        assert_eq!(
            split("/a/b/c").unwrap(),
            (Some("/a/b".to_string()), "c".to_string())
        );
    }

    #[test]
    fn test_join() {
        assert_eq!(join("/", "a").unwrap(), "/a");
        assert_eq!(join("/a", "b").unwrap(), "/a/b");
        assert_eq!(join("/a/b", "c").unwrap(), "/a/b/c");
    }

    #[test]
    fn test_components() {
        assert_eq!(components("/").unwrap(), Vec::<String>::new());
        assert_eq!(components("/a").unwrap(), vec!["a"]);
        assert_eq!(components("/a/b/c").unwrap(), vec!["a", "b", "c"]);
    }
}
