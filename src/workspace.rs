use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::tools::ToolError;

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, ToolError> {
        let root = fs::canonicalize(root)?;
        if !root.is_dir() {
            return Err(ToolError::InvalidArguments(
                "workspace must be a directory".into(),
            ));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Reject traversal and symlinks, including symlinks in existing ancestors.
    /// Missing components are allowed for creation; the tool checks its parent.
    pub fn resolve(&self, path: &str) -> Result<PathBuf, ToolError> {
        if path.is_empty() {
            return Err(ToolError::InvalidArguments("path must not be empty".into()));
        }
        let supplied = Path::new(path);
        let relative = if supplied.is_absolute() {
            supplied
                .strip_prefix(&self.root)
                .map_err(|_| ToolError::PathDenied(path.into()))?
        } else {
            supplied
        };
        let mut resolved = self.root.clone();
        for component in relative.components() {
            match component {
                Component::CurDir => continue,
                Component::Normal(name) => resolved.push(name),
                _ => return Err(ToolError::PathDenied(path.into())),
            }
            match fs::symlink_metadata(&resolved) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(ToolError::PathDenied(path.into()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(resolved)
    }

    pub fn instructions(&self) -> Result<Option<String>, ToolError> {
        let path = self.resolve("AGENTS.md")?;
        match fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }
}
