use std::{
    io,
    path::{Path, PathBuf},
};

use super::manager::{self, Manager};

#[derive(Debug, thiserror::Error)]
pub enum RootError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Manager(#[from] manager::ParseManagerError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    // The manager that was found for this workspace root.
    manager: Manager,
    // The file which identifies the workspace root, e.g. `yarn.lock`.
    file: PathBuf,
}

impl Root {
    pub fn new(cwd: impl AsRef<Path>) -> Result<Self, RootError> {
        if let Some(manager) = Manager::from_env()? {
            return Ok(Self::with_manager(cwd, manager)?);
        }

        let (manager, file) = Manager::discover(cwd)?;
        Ok(Self { manager, file })
    }

    pub fn with_manager(cwd: impl AsRef<Path>, manager: Manager) -> io::Result<Self> {
        let file = manager.locate(cwd)?;
        Ok(Self { manager, file })
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn path(&self) -> &Path {
        self.file.parent().expect("file path has parent")
    }
}
