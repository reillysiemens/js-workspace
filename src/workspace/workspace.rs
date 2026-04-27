use std::{
    collections::BTreeSet,
    env, fs, io,
    path::{Path, PathBuf},
};

use bon::bon;
use serde::de::DeserializeOwned;

use super::manager::{self, Manager, ManagerMarker};

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Manager(#[from] manager::ParseManagerError),
    #[error("{manager} workspace configuration was not found at {path}")]
    MissingWorkspaceConfig { manager: Manager, path: PathBuf },
    #[error("{manager} workspace configuration at {path} does not define packages")]
    MissingPackageLayout { manager: Manager, path: PathBuf },
    #[error("invalid {manager} workspace configuration at {path}: {source}")]
    InvalidWorkspaceConfig {
        manager: Manager,
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("invalid {manager} package pattern in {path}: {pattern}")]
    InvalidPackagePattern {
        manager: Manager,
        path: PathBuf,
        pattern: String,
    },
    #[error("invalid {manager} package path in {path}: {package_path}")]
    InvalidPackagePath {
        manager: Manager,
        path: PathBuf,
        package_path: PathBuf,
    },
    #[error("invalid package glob {pattern}: {source}")]
    InvalidPackageGlob {
        pattern: String,
        #[source]
        source: glob::PatternError,
    },
    #[error("failed to read package glob match for {pattern}: {source}")]
    PackageGlob {
        pattern: String,
        #[source]
        source: glob::GlobError,
    },
    #[error("{manager} package path does not contain package.json: {path}")]
    MissingPackageManifest { manager: Manager, path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    path: PathBuf,
}

impl Package {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    root: PathBuf,
    manager: WorkspaceManager,
}

#[bon]
impl Workspace {
    /// Discovers the workspace from the current directory with default options.
    pub fn discover() -> Result<Self, WorkspaceError> {
        Self::builder().cwd(env::current_dir()?).build()
    }

    /// Returns a builder for configuring workspace discovery.
    #[builder]
    pub fn new(
        /// The directory to start searching from.
        #[builder(into)]
        cwd: PathBuf,
        /// Override manager discovery with a specific manager.
        manager: Option<Manager>,
        /// Stop searching after checking this directory.
        #[builder(into)]
        ceiling: Option<PathBuf>,
        /// Whether to check the environment for a preferred manager.
        #[builder(default = true)]
        check_env: bool,
    ) -> Result<Self, WorkspaceError> {
        let marker = if let Some(manager) = manager {
            manager.locate(&cwd, ceiling.as_deref())?
        } else if check_env && let Some(manager) = Manager::from_env()? {
            manager.locate(&cwd, ceiling.as_deref())?
        } else {
            Manager::discover(&cwd, ceiling.as_deref())?
        };

        let manager = WorkspaceManager::from_marker(marker)?;
        let root = manager.marker_file.root().to_path_buf();

        Ok(Self { root, manager })
    }

    pub fn file(&self) -> &Path {
        &self.manager.marker_file.file
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn packages(&self) -> Result<Vec<Package>, WorkspaceError> {
        let paths = match &self.manager.package_layout {
            PackageLayout::Globs(globs) => discover_glob_packages(self.path(), globs)?,
            PackageLayout::Paths(paths) => {
                discover_path_packages(self.path(), self.manager.kind, paths)?
            }
        };

        Ok(paths.into_iter().map(Package::new).collect())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkspaceManager {
    kind: Manager,
    marker_file: ManagerMarker,
    package_layout: PackageLayout,
}

impl WorkspaceManager {
    fn from_marker(marker: ManagerMarker) -> Result<Self, WorkspaceError> {
        let package_layout = parse_package_layout(&marker)?;

        Ok(Self {
            kind: marker.kind,
            marker_file: marker,
            package_layout,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PackageLayout {
    Globs(PackageGlobs),
    Paths(Vec<PathBuf>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PackageGlobs {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl PackageGlobs {
    fn new(
        manager: Manager,
        config_path: &Path,
        patterns: impl IntoIterator<Item = String>,
    ) -> Result<Self, WorkspaceError> {
        let mut include = Vec::new();
        let mut exclude = Vec::new();

        for pattern in patterns {
            let (patterns, pattern) = if let Some(pattern) = pattern.strip_prefix('!') {
                (&mut exclude, pattern)
            } else {
                (&mut include, pattern.as_str())
            };

            if pattern.is_empty()
                || Path::new(pattern).is_absolute()
                || Path::new(pattern)
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
            {
                return Err(WorkspaceError::InvalidPackagePattern {
                    manager,
                    path: config_path.to_path_buf(),
                    pattern: pattern.to_string(),
                });
            }

            patterns.push(pattern.to_string());
        }

        if include.is_empty() {
            return Err(WorkspaceError::MissingPackageLayout {
                manager,
                path: config_path.to_path_buf(),
            });
        }

        Ok(Self { include, exclude })
    }
}

fn parse_package_layout(marker: &ManagerMarker) -> Result<PackageLayout, WorkspaceError> {
    match marker.kind {
        Manager::Yarn | Manager::Npm => parse_package_json_layout(marker.root(), marker.kind),
        Manager::Pnpm => parse_pnpm_layout(marker),
        Manager::Rush => parse_rush_layout(marker),
        Manager::Lerna => parse_lerna_layout(marker),
    }
}

fn parse_package_json_layout(
    root: &Path,
    manager: Manager,
) -> Result<PackageLayout, WorkspaceError> {
    let path = root.join("package.json");
    let package_json: PackageJson = read_json(manager, &path)?;
    let workspaces =
        package_json
            .workspaces
            .ok_or_else(|| WorkspaceError::MissingPackageLayout {
                manager,
                path: path.clone(),
            })?;

    Ok(PackageLayout::Globs(PackageGlobs::new(
        manager,
        &path,
        workspaces.into_patterns(),
    )?))
}

fn parse_pnpm_layout(marker: &ManagerMarker) -> Result<PackageLayout, WorkspaceError> {
    parse_pnpm_config(marker.kind, &marker.file)
}

fn parse_pnpm_config(manager: Manager, path: &Path) -> Result<PackageLayout, WorkspaceError> {
    let workspace: PnpmWorkspace = read_yaml(manager, path)?;
    Ok(PackageLayout::Globs(PackageGlobs::new(
        manager,
        path,
        workspace.packages,
    )?))
}

fn parse_rush_layout(marker: &ManagerMarker) -> Result<PackageLayout, WorkspaceError> {
    let rush: RushJson = read_json(marker.kind, &marker.file)?;
    if rush.projects.is_empty() {
        return Err(WorkspaceError::MissingPackageLayout {
            manager: marker.kind,
            path: marker.file.clone(),
        });
    }

    let paths = rush
        .projects
        .into_iter()
        .map(|project| workspace_path(marker.kind, &marker.file, project.project_folder))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PackageLayout::Paths(paths))
}

fn parse_lerna_layout(marker: &ManagerMarker) -> Result<PackageLayout, WorkspaceError> {
    let lerna: LernaJson = read_json(marker.kind, &marker.file)?;
    if let Some(packages) = lerna.packages {
        return Ok(PackageLayout::Globs(PackageGlobs::new(
            marker.kind,
            &marker.file,
            packages,
        )?));
    }

    let pnpm_config = marker.root().join(Manager::Pnpm.root_file());
    if pnpm_config.try_exists()? {
        return parse_pnpm_config(marker.kind, &pnpm_config);
    }

    parse_package_json_layout(marker.root(), marker.kind)
}

fn read_json<T>(manager: Manager, path: &Path) -> Result<T, WorkspaceError>
where
    T: DeserializeOwned,
{
    let file = open_config(manager, path)?;
    serde_json::from_reader(file).map_err(|source| WorkspaceError::InvalidWorkspaceConfig {
        manager,
        path: path.to_path_buf(),
        source: Box::new(source),
    })
}

fn read_yaml<T>(manager: Manager, path: &Path) -> Result<T, WorkspaceError>
where
    T: DeserializeOwned,
{
    let file = open_config(manager, path)?;
    serde_yml::from_reader(file).map_err(|source| WorkspaceError::InvalidWorkspaceConfig {
        manager,
        path: path.to_path_buf(),
        source: Box::new(source),
    })
}

fn open_config(manager: Manager, path: &Path) -> Result<fs::File, WorkspaceError> {
    fs::File::open(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            WorkspaceError::MissingWorkspaceConfig {
                manager,
                path: path.to_path_buf(),
            }
        } else {
            WorkspaceError::Io(source)
        }
    })
}

fn workspace_path(
    manager: Manager,
    config_path: &Path,
    path: PathBuf,
) -> Result<PathBuf, WorkspaceError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(WorkspaceError::InvalidPackagePath {
            manager,
            path: config_path.to_path_buf(),
            package_path: path,
        });
    }

    Ok(path)
}

fn discover_glob_packages(
    root: &Path,
    globs: &PackageGlobs,
) -> Result<Vec<PathBuf>, WorkspaceError> {
    let mut packages = BTreeSet::new();
    for pattern in &globs.include {
        packages.extend(matching_package_dirs(root, pattern)?);
    }

    for pattern in &globs.exclude {
        for excluded in matching_package_dirs(root, pattern)? {
            packages.remove(&excluded);
        }
    }

    Ok(packages.into_iter().collect())
}

fn matching_package_dirs(root: &Path, pattern: &str) -> Result<BTreeSet<PathBuf>, WorkspaceError> {
    let manifest_pattern = root
        .join(pattern)
        .join("package.json")
        .to_string_lossy()
        .into_owned();
    let matches =
        glob::glob(&manifest_pattern).map_err(|source| WorkspaceError::InvalidPackageGlob {
            pattern: manifest_pattern.clone(),
            source,
        })?;

    let mut package_dirs = BTreeSet::new();
    for matched in matches {
        let manifest = matched.map_err(|source| WorkspaceError::PackageGlob {
            pattern: manifest_pattern.clone(),
            source,
        })?;
        let package_dir = manifest.parent().expect("package manifest path has parent");
        let relative_package_dir = package_dir.strip_prefix(root).unwrap_or(package_dir);
        if relative_package_dir
            .components()
            .any(|component| component.as_os_str() == std::ffi::OsStr::new("node_modules"))
        {
            continue;
        }
        package_dirs.insert(package_dir.canonicalize()?);
    }

    Ok(package_dirs)
}

fn discover_path_packages(
    root: &Path,
    manager: Manager,
    paths: &[PathBuf],
) -> Result<Vec<PathBuf>, WorkspaceError> {
    paths
        .iter()
        .map(|path| {
            let package_dir = root.join(path);
            let manifest = package_dir.join("package.json");
            if !manifest.try_exists()? {
                return Err(WorkspaceError::MissingPackageManifest {
                    manager,
                    path: package_dir,
                });
            }
            package_dir.canonicalize().map_err(WorkspaceError::Io)
        })
        .collect()
}

#[derive(serde::Deserialize)]
struct PackageJson {
    workspaces: Option<PackageJsonWorkspaces>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum PackageJsonWorkspaces {
    Patterns(Vec<String>),
    Object { packages: Vec<String> },
}

impl PackageJsonWorkspaces {
    fn into_patterns(self) -> Vec<String> {
        match self {
            PackageJsonWorkspaces::Patterns(patterns) => patterns,
            PackageJsonWorkspaces::Object { packages } => packages,
        }
    }
}

#[derive(serde::Deserialize)]
struct PnpmWorkspace {
    #[serde(default)]
    packages: Vec<String>,
}

#[derive(serde::Deserialize)]
struct RushJson {
    #[serde(default)]
    projects: Vec<RushProject>,
}

#[derive(serde::Deserialize)]
struct RushProject {
    #[serde(rename = "projectFolder")]
    project_folder: PathBuf,
}

#[derive(serde::Deserialize)]
struct LernaJson {
    packages: Option<Vec<String>>,
}
