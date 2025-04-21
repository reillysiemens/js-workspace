use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::env::PREFERRED_WORKSPACE_MANAGER;

// DO NOT REORDER! This order determines the precedence of the files, which is
// important for cases like lerna where lerna.json and e.g. yarn.lock may both exist.
pub(crate) const SEARCH_ORDER: &[Manager] = &[
    Manager::Lerna,
    Manager::Rush,
    Manager::Yarn,
    Manager::Pnpm,
    Manager::Npm,
];

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("Invalid manager: {0}")]
pub struct ParseManagerError(String);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("Invalid manager file: {0}")]
pub struct InvalidFileError(PathBuf);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] ParseManagerError),
    #[error(transparent)]
    InvalidFile(#[from] InvalidFileError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Manager {
    Yarn,
    Pnpm,
    Rush,
    Npm,
    Lerna,
}

impl FromStr for Manager {
    type Err = ParseManagerError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "yarn" => Ok(Self::Yarn),
            "pnpm" => Ok(Self::Pnpm),
            "rush" => Ok(Self::Rush),
            "npm" => Ok(Self::Npm),
            "lerna" => Ok(Self::Lerna),
            _ => Err(ParseManagerError(input.to_string())),
        }
    }
}

impl Manager {
    pub fn preferred() -> Result<Option<Manager>, ParseManagerError> {
        match env::var(PREFERRED_WORKSPACE_MANAGER) {
            Ok(var) => Ok(Some(var.parse()?)),
            Err(_err) => Ok(None), // TODO: Maybe add some logging here?
        }
    }
}

impl AsRef<Path> for Manager {
    fn as_ref(&self) -> &Path {
        match self {
            Manager::Yarn => &Path::new("yarn.lock"),
            Manager::Pnpm => &Path::new("pnpm-workspace.yaml"),
            Manager::Rush => &Path::new("rush.json"),
            Manager::Npm => &Path::new("package-lock.json"),
            Manager::Lerna => &Path::new("lerna.json"),
        }
    }
}

impl TryFrom<&Path> for Manager {
    type Error = InvalidFileError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        match path.file_name().and_then(OsStr::to_str) {
            Some("yarn.lock") => Ok(Self::Yarn),
            Some("pnpm-workspace.yaml") => Ok(Manager::Pnpm),
            Some("rush.json") => Ok(Manager::Rush),
            Some("package-lock.json") => Ok(Manager::Npm),
            Some("lerna.json") => Ok(Manager::Lerna),
            _ => Err(InvalidFileError(path.to_path_buf())),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use test_case::test_case;

    use super::*;

    #[test_case("yarn", Ok(Manager::Yarn) ; "lowercase yarn")]
    #[test_case("YARN", Ok(Manager::Yarn) ; "uppercase yarn")]
    #[test_case("pnpm", Ok(Manager::Pnpm) ; "lowercase pnpm")]
    #[test_case("PNPM", Ok(Manager::Pnpm) ; "uppercase pnpm")]
    #[test_case("rush", Ok(Manager::Rush) ; "lowercase rush")]
    #[test_case("RUSH", Ok(Manager::Rush) ; "uppercase rush")]
    #[test_case("npm", Ok(Manager::Npm) ; "lowercase npm")]
    #[test_case("NPM", Ok(Manager::Npm) ; "uppercase npm")]
    #[test_case("lerna", Ok(Manager::Lerna) ; "lowercase lerna")]
    #[test_case("LERNA", Ok(Manager::Lerna) ; "uppercase lerna")]
    #[test_case("lolwut", Err(ParseManagerError(String::from("lolwut"))) ; "lowercase failure")]
    #[test_case("LOLWUT", Err(ParseManagerError(String::from("LOLWUT"))) ; "uppercase failure")]
    fn parse_manager(given: &str, expected: Result<Manager, ParseManagerError>) {
        let actual = given.parse();
        assert_eq!(actual, expected);
    }

    #[test_case(Manager::Yarn, &Path::new("yarn.lock"))]
    #[test_case(Manager::Pnpm, &Path::new("pnpm-workspace.yaml"))]
    #[test_case(Manager::Rush, &Path::new("rush.json"))]
    #[test_case(Manager::Npm, Path::new("package-lock.json"))]
    #[test_case(Manager::Lerna, &Path::new("lerna.json"))]
    fn asref_path(given: Manager, expected: &Path) {
        let actual = given.as_ref();
        assert_eq!(actual, expected);
    }
}
