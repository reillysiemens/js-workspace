use std::{
    env, io,
    path::{Path, PathBuf},
};

use bon::bon;

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

#[bon]
impl Root {
    /// Discovers the workspace root from the current directory with default options.
    pub fn discover() -> Result<Self, RootError> {
        Self::builder().cwd(env::current_dir()?).build()
    }

    /// Returns a builder for configuring workspace root discovery.
    #[builder]
    pub fn new(
        /// The directory to start searching from.
        #[builder(into)]
        cwd: PathBuf,
        /// Override manager discovery with a specific manager.
        manager: Option<Manager>,
        /// Stop searching at this directory (exclusive).
        #[builder(into)]
        ceiling: Option<PathBuf>,
        /// Whether to check the environment for a preferred manager.
        #[builder(default = true)]
        check_env: bool,
    ) -> Result<Self, RootError> {
        // If manager is explicitly set, use it directly.
        if let Some(manager) = manager {
            let file = manager.locate(&cwd, ceiling.as_deref())?;
            return Ok(Root { manager, file });
        }

        // Check environment if enabled.
        if check_env && let Some(manager) = Manager::from_env()? {
            let file = manager.locate(&cwd, ceiling.as_deref())?;
            return Ok(Root { manager, file });
        }

        // Discover manager from filesystem.
        let (manager, file) = Manager::discover(&cwd, ceiling.as_deref())?;
        Ok(Root { manager, file })
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn path(&self) -> &Path {
        self.file.parent().expect("file path has parent")
    }
}
