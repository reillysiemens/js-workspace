use std::path::PathBuf;

use serde::Deserialize;

// TODO: Are these the best visibility rules?

#[derive(Debug, PartialEq, Eq)]
pub struct WorkspacePackage {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct RootPackage {
    workspaces: serde_json::Value,
}

impl RootPackage {
    pub fn workspace_packages(&self) -> Result<Vec<WorkspacePackage>, ()> {
        if let Some(globs) = self.workspaces.as_array() {
            todo!("DO THE THING");
        }

        Err(())
    }
}
