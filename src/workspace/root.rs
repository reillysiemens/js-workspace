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

        let files: Vec<_> = Manager::root_files_in_search_order().collect();
        let mut path = search_up(cwd.as_ref(), &files)?;
        let manager = Manager::try_from(path.as_ref())
            .expect("root file discovered via search order should parse into a Manager");
        path.pop(); // Truncate to the manager file's parent path.

        Ok(Self { manager, path })
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
