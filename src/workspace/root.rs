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
    manager: Manager,
    path: PathBuf,
}

impl Root {
    pub fn new(cwd: impl AsRef<Path>) -> Result<Self, RootError> {
        if let Some(manager) = Manager::from_env()? {
            return Ok(Self::with_manager(cwd, manager)?);
        }

        let (manager, mut path) = Manager::search(cwd)?;
        path.pop(); // Truncate to the manager file's parent path.

        Ok(Self { manager, path })
    }

    pub fn with_manager(cwd: impl AsRef<Path>, manager: Manager) -> io::Result<Self> {
        let mut path = manager.find(cwd)?;
        path.pop(); // Truncate to the manager file's parent path.

        Ok(Self { manager, path })
    }
}
