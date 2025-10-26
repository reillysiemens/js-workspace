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
        let (manager, mut file_path) = Manager::search(cwd.as_ref())?;
        file_path.pop(); // parent directory of root file
        Ok(Self {
            manager,
            path: file_path,
        })
    }

    pub fn with_manager(cwd: impl AsRef<Path>, manager: Manager) -> io::Result<Self> {
        let mut file_path = manager.find(cwd.as_ref())?;
        file_path.pop();
        Ok(Self {
            manager,
            path: file_path,
        })
    }
}
