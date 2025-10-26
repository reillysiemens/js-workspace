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
        let files = [manager.root_file()];
        let mut path = search_up(cwd.as_ref(), &files)?;
        path.pop();

        Ok(Self { manager, path })
    }
}

fn search_up(cwd: &Path, files: &[&Path]) -> io::Result<PathBuf> {
    let mut cwd = cwd.canonicalize()?;

    loop {
        for file in files {
            let candidate = cwd.join(file);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        if !cwd.pop() {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
    }
}
