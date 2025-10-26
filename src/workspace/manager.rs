use std::{
    env, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::env::PREFERRED_WORKSPACE_MANAGER;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("Invalid manager: {0}")]
pub struct ParseManagerError(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    // DO NOT REORDER! This order determines the precedence of the files, which is
    // important for cases like lerna where lerna.json and e.g. yarn.lock may both exist.
    const SEARCH_ORDER: &[Self] = &[
        Manager::Lerna,
        Manager::Rush,
        Manager::Yarn,
        Manager::Pnpm,
        Manager::Npm,
    ];

    pub fn from_env() -> Result<Option<Manager>, ParseManagerError> {
        match env::var(PREFERRED_WORKSPACE_MANAGER) {
            Ok(var) => Ok(Some(var.parse()?)),
            Err(_err) => Ok(None), // TODO: Maybe add some logging here?
        }
    }

    /// Returns the root file used to identify this manager as a relative [Path].
    pub fn root_file(&self) -> &Path {
        Path::new(self.root_filename())
    }

    /// Returns the root filename used to identify this manager.
    fn root_filename(&self) -> &'static str {
        match self {
            Manager::Yarn => "yarn.lock",
            Manager::Pnpm => "pnpm-workspace.yaml",
            Manager::Rush => "rush.json",
            Manager::Npm => "package-lock.json",
            Manager::Lerna => "lerna.json",
        }
    }

    /// Searches upward from `cwd` for any manager root file in precedence order.
    /// Returns the first match as `(Manager, PathBuf)` where the path points to the
    /// discovered root file.
    pub fn search(cwd: impl AsRef<Path>) -> io::Result<(Manager, PathBuf)> {
        let mut dir = cwd.as_ref().canonicalize()?;
        loop {
            for m in Self::SEARCH_ORDER {
                let candidate = dir.join(m.root_file());
                if candidate.exists() {
                    return Ok((*m, candidate));
                }
            }
            if !dir.pop() {
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }
        }
    }

    /// Searches upward from `cwd` for this specific manager's root file only.
    /// Returns the path to the discovered root file, or NotFound if none was found.
    pub fn find(self, cwd: impl AsRef<Path>) -> io::Result<PathBuf> {
        let mut dir = cwd.as_ref().canonicalize()?;
        loop {
            let candidate = dir.join(self.root_file());
            if candidate.exists() {
                return Ok(candidate);
            }
            if !dir.pop() {
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }
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

    #[test_case(Manager::Yarn, &Path::new("yarn.lock") ; "yarn")]
    #[test_case(Manager::Pnpm, &Path::new("pnpm-workspace.yaml") ; "pnpm")]
    #[test_case(Manager::Rush, &Path::new("rush.json") ; "rush")]
    #[test_case(Manager::Npm, Path::new("package-lock.json") ; "npm")]
    #[test_case(Manager::Lerna, &Path::new("lerna.json") ; "lerna")]
    fn root_file(given: Manager, expected: &Path) {
        let actual = given.root_file();
        assert_eq!(actual, expected);
    }
}
