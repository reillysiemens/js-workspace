use std::{fs, io, path::Path};

use pretty_assertions::assert_eq;
use test_case::test_case;

use js_workspace::workspace::{Manager, Workspace, WorkspaceError};

#[test]
fn returns_not_found_when_no_manager_file_exists() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();

    // Act
    let actual = Workspace::builder()
        .cwd(workspace)
        .ceiling(workspace)
        .build();

    // Assert
    assert!(matches!(
        actual.expect_err("workspace should not be found"),
        WorkspaceError::Io(error) if error.kind() == io::ErrorKind::NotFound
    ));

    Ok(())
}

#[test_case(Manager::Yarn ; "yarn")]
#[test_case(Manager::Pnpm ; "pnpm")]
#[test_case(Manager::Rush ; "rush")]
#[test_case(Manager::Npm ; "npm")]
#[test_case(Manager::Lerna ; "lerna")]
fn finds_workspace_from_workspace_root(manager: Manager) -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    let marker = write_workspace(workspace, manager)?;

    // Act
    let workspace = Workspace::builder().cwd(workspace).build()?;

    // Assert
    assert_eq!(workspace.path(), tmp.path());
    assert_eq!(workspace.file(), marker);

    Ok(())
}

#[test_case(Manager::Yarn ; "yarn")]
#[test_case(Manager::Pnpm ; "pnpm")]
#[test_case(Manager::Rush ; "rush")]
#[test_case(Manager::Npm ; "npm")]
#[test_case(Manager::Lerna ; "lerna")]
fn finds_workspace_from_nested_directory(manager: Manager) -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    let nested = workspace.join("packages/test-package");
    let marker = write_workspace(workspace, manager)?;
    fs::create_dir_all(&nested)?;

    // Act
    let workspace = Workspace::builder().cwd(&nested).build()?;

    // Assert
    assert_eq!(workspace.path(), tmp.path());
    assert_eq!(workspace.file(), marker);

    Ok(())
}

#[test_case(&[Manager::Lerna, Manager::Rush, Manager::Yarn, Manager::Pnpm, Manager::Npm], Manager::Lerna ; "all present")]
#[test_case(&[Manager::Rush, Manager::Yarn, Manager::Pnpm, Manager::Npm], Manager::Rush ; "lerna removed")]
#[test_case(&[Manager::Yarn, Manager::Pnpm, Manager::Npm], Manager::Yarn ; "rush removed")]
#[test_case(&[Manager::Pnpm, Manager::Npm], Manager::Pnpm ; "yarn removed")]
#[test_case(&[Manager::Npm], Manager::Npm ; "only npm")]
fn respects_manager_precedence(managers: &[Manager], expected: Manager) -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    for manager in managers {
        write_workspace(workspace, *manager)?;
    }

    // Act
    let workspace = Workspace::builder().cwd(workspace).build()?;

    // Assert
    assert_eq!(
        workspace.file(),
        workspace.path().join(expected.root_file())
    );

    Ok(())
}

#[test]
fn explicit_manager_override_ignores_precedence() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    write_workspace(workspace, Manager::Lerna)?;
    let expected = write_workspace(workspace, Manager::Pnpm)?;

    // Act
    let workspace = Workspace::builder()
        .cwd(workspace)
        .manager(Manager::Pnpm)
        .build()?;

    // Assert
    assert_eq!(workspace.file(), expected);

    Ok(())
}

#[test]
fn explicit_manager_override_returns_not_found_when_manager_file_is_absent() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();

    // Act
    let actual = Workspace::builder()
        .cwd(workspace)
        .manager(Manager::Pnpm)
        .ceiling(workspace)
        .build();

    // Assert
    assert!(matches!(
        actual.expect_err("pnpm workspace should not be found"),
        WorkspaceError::Io(error) if error.kind() == io::ErrorKind::NotFound
    ));

    Ok(())
}

#[test]
fn rejects_marker_without_workspace_configuration() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    fs::write(workspace.join("yarn.lock"), "")?;

    // Act
    let actual = Workspace::builder().cwd(workspace).build();

    // Assert
    assert!(matches!(
        actual.expect_err("workspace configuration should be required"),
        WorkspaceError::MissingWorkspaceConfig { manager: Manager::Yarn, path }
            if path == workspace.join("package.json")
    ));

    Ok(())
}

#[test]
fn finds_workspace_when_cwd_is_ceiling_directory() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let repo = tmp.path();
    let marker = write_workspace(repo, Manager::Yarn)?;

    // Act
    let workspace = Workspace::builder().cwd(repo).ceiling(repo).build()?;

    // Assert
    assert_eq!(workspace.path(), repo);
    assert_eq!(workspace.file(), marker);

    Ok(())
}

#[test]
fn ceiling_prevents_searching_above_ceiling_directory() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let home = tmp.path().join("home");
    let project = home.join("projects/js-workspace");
    fs::create_dir_all(&project)?;
    write_workspace(tmp.path(), Manager::Yarn)?;

    // Act
    let actual = Workspace::builder().cwd(&project).ceiling(&home).build();

    // Assert
    assert!(matches!(
        actual.expect_err("ceiling should exclude parent directory"),
        WorkspaceError::Io(error) if error.kind() == io::ErrorKind::NotFound
    ));

    Ok(())
}

#[test]
fn finds_nested_workspace_below_ceiling() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let ceiling = tmp.path();
    let workspace = ceiling.join("monorepo");
    let nested = workspace.join("packages/test-package");
    let marker = write_workspace(&workspace, Manager::Yarn)?;
    fs::create_dir_all(&nested)?;

    // Act
    let workspace = Workspace::builder().cwd(&nested).ceiling(ceiling).build()?;

    // Assert
    assert_eq!(workspace.path(), ceiling.join("monorepo"));
    assert_eq!(workspace.file(), marker);

    Ok(())
}

#[test]
fn rejects_ceiling_below_cwd() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path().join("workspace");
    let package = workspace.join("packages/test-package");
    fs::create_dir_all(&package)?;

    // Act
    let actual = Workspace::builder()
        .cwd(&workspace)
        .ceiling(&package)
        .build();

    // Assert
    assert!(matches!(
        actual.expect_err("ceiling below cwd should be rejected"),
        WorkspaceError::Io(error) if error.kind() == io::ErrorKind::InvalidInput
    ));

    Ok(())
}

#[test]
fn discovers_packages_from_package_json_workspaces() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    write_workspace(workspace, Manager::Yarn)?;
    let expected = create_packages(workspace, &["packages/a", "packages/b"])?;

    // Act
    let packages = Workspace::builder().cwd(workspace).build()?.packages()?;
    let actual = package_paths(&packages);

    // Assert
    assert_eq!(actual, expected);

    Ok(())
}

#[test]
fn discovers_packages_from_rush_project_paths() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    write_workspace(workspace, Manager::Rush)?;
    let expected = create_packages(workspace, &["packages/a", "tools/b"])?;

    // Act
    let packages = Workspace::builder().cwd(workspace).build()?.packages()?;
    let actual = package_paths(&packages);

    // Assert
    assert_eq!(actual, expected);

    Ok(())
}

#[test]
fn package_globs_respect_exclusions() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    fs::write(
        workspace.join("pnpm-workspace.yaml"),
        "packages:\n  - packages/*\n  - '!packages/ignored'\n",
    )?;
    let expected = create_packages(workspace, &["packages/a"])?;
    create_packages(workspace, &["packages/ignored"])?;

    // Act
    let packages = Workspace::builder().cwd(workspace).build()?.packages()?;
    let actual = package_paths(&packages);

    // Assert
    assert_eq!(actual, expected);

    Ok(())
}

#[test]
fn lerna_can_fall_back_to_pnpm_workspace_configuration() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    fs::write(workspace.join("lerna.json"), "{}")?;
    fs::write(
        workspace.join("pnpm-workspace.yaml"),
        "packages:\n  - apps/*\n",
    )?;
    let expected = create_packages(workspace, &["apps/a"])?;

    // Act
    let packages = Workspace::builder().cwd(workspace).build()?.packages()?;
    let actual = package_paths(&packages);

    // Assert
    assert_eq!(actual, expected);

    Ok(())
}

fn write_workspace(root: &Path, manager: Manager) -> anyhow::Result<std::path::PathBuf> {
    fs::create_dir_all(root)?;
    match manager {
        Manager::Yarn => {
            let marker = root.join("yarn.lock");
            fs::write(&marker, "")?;
            write_package_json_workspaces(root)?;
            Ok(marker)
        }
        Manager::Pnpm => {
            let marker = root.join("pnpm-workspace.yaml");
            fs::write(&marker, "packages:\n  - packages/*\n")?;
            Ok(marker)
        }
        Manager::Rush => {
            let marker = root.join("rush.json");
            fs::write(
                &marker,
                r#"{"projects":[{"projectFolder":"packages/a"},{"projectFolder":"tools/b"}]}"#,
            )?;
            Ok(marker)
        }
        Manager::Npm => {
            let marker = root.join("package-lock.json");
            fs::write(&marker, "{}")?;
            write_package_json_workspaces(root)?;
            Ok(marker)
        }
        Manager::Lerna => {
            let marker = root.join("lerna.json");
            fs::write(&marker, r#"{"packages":["packages/*"]}"#)?;
            Ok(marker)
        }
    }
}

fn write_package_json_workspaces(root: &Path) -> anyhow::Result<()> {
    fs::write(
        root.join("package.json"),
        r#"{"workspaces":["packages/*"]}"#,
    )?;
    Ok(())
}

fn create_packages(root: &Path, packages: &[&str]) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();
    for package in packages {
        let path = root.join(package);
        fs::create_dir_all(&path)?;
        fs::write(path.join("package.json"), "{}")?;
        paths.push(path.canonicalize()?);
    }
    paths.sort();
    Ok(paths)
}

fn package_paths(packages: &[js_workspace::workspace::Package]) -> Vec<std::path::PathBuf> {
    packages
        .iter()
        .map(|package| package.path().to_path_buf())
        .collect()
}
