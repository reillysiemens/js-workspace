use std::{
    io,
    path::{Path, PathBuf},
};

use super::manager::{self, Manager, SEARCH_ORDER};

#[derive(Debug, thiserror::Error)]
pub enum RootError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{0}")]
    Manager(String),
}

impl From<manager::ParseManagerError> for RootError {
    fn from(error: manager::ParseManagerError) -> Self {
        Self::Manager(error.to_string())
    }
}

impl From<manager::InvalidFileError> for RootError {
    fn from(error: manager::InvalidFileError) -> Self {
        Self::Manager(error.to_string())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Root {
    manager: Manager,
    path: PathBuf,
}

impl Root {
    pub fn new(cwd: impl AsRef<Path>) -> Result<Self, RootError> {
        if let Some(manager) = Manager::preferred()? {
            return Ok(Self::with_manager(cwd, manager)?);
        }

        let mut path = search_up(cwd, SEARCH_ORDER)?;
        let manager = Manager::try_from(path.as_ref())?;
        path.pop(); // Truncate to the manager file's parent path.
        Ok(Self { manager, path })
    }

    pub fn with_manager(cwd: impl AsRef<Path>, manager: Manager) -> io::Result<Self> {
        let mut path = search_up(cwd, [&manager])?;
        path.pop();
        Ok(Self { manager, path })
    }
}

fn search_up(
    cwd: impl AsRef<Path>,
    files: impl IntoIterator<Item = impl AsRef<Path>>,
) -> io::Result<PathBuf> {
    // TODO: Are these conversions necessary and/or good? Should cwd be canonicalized?
    let mut cwd = cwd.as_ref().to_path_buf();
    let files: Vec<_> = files.into_iter().map(|p| p.as_ref().to_owned()).collect();

    loop {
        for file in &files {
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
