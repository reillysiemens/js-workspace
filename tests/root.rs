use std::{fs, io};

use pretty_assertions::assert_eq;
use test_case::test_case;

use js_workspace::workspace::{Manager, Root, root::RootError};

#[test]
fn returns_not_found_when_no_manager_file_exists() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();

    // Act
    let actual = Root::builder().cwd(workspace).ceiling(workspace).build();

    // Assert
    assert!(matches!(
        actual.expect_err("workspace root should not be found"),
        RootError::Io(error) if error.kind() == io::ErrorKind::NotFound
    ));

    Ok(())
}

#[test_case("yarn.lock" ; "yarn")]
#[test_case("pnpm-workspace.yaml" ; "pnpm")]
#[test_case("rush.json" ; "rush")]
#[test_case("package-lock.json" ; "npm")]
#[test_case("lerna.json" ; "lerna")]
fn finds_root_from_workspace_root(root_file: &str) -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    let root_file = workspace.join(root_file);
    fs::write(&root_file, "")?;

    // Act
    let root = Root::builder().cwd(workspace).build()?;

    // Assert
    assert_eq!(root.path(), workspace);
    assert_eq!(root.file(), root_file);

    Ok(())
}

#[test_case("yarn.lock" ; "yarn")]
#[test_case("pnpm-workspace.yaml" ; "pnpm")]
#[test_case("rush.json" ; "rush")]
#[test_case("package-lock.json" ; "npm")]
#[test_case("lerna.json" ; "lerna")]
fn finds_root_from_nested_directory(root_file: &str) -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    let nested = workspace.join("packages/test-package");
    let root_file = workspace.join(root_file);
    fs::write(&root_file, "")?;
    fs::create_dir_all(&nested)?;

    // Act
    let root = Root::builder().cwd(&nested).build()?;

    // Assert
    assert_eq!(root.path(), workspace);
    assert_eq!(root.file(), root_file);

    Ok(())
}

#[test_case(&["lerna.json", "rush.json", "yarn.lock", "pnpm-workspace.yaml", "package-lock.json"], "lerna.json" ; "all present")]
#[test_case(&["rush.json", "yarn.lock", "pnpm-workspace.yaml", "package-lock.json"], "rush.json" ; "lerna removed")]
#[test_case(&["yarn.lock", "pnpm-workspace.yaml", "package-lock.json"], "yarn.lock" ; "rush removed")]
#[test_case(&["pnpm-workspace.yaml", "package-lock.json"], "pnpm-workspace.yaml" ; "yarn removed")]
#[test_case(&["package-lock.json"], "package-lock.json" ; "only npm")]
fn respects_manager_precedence(files: &[&str], expected: &str) -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    for file in files {
        fs::write(workspace.join(file), "")?;
    }

    // Act
    let root = Root::builder().cwd(workspace).build()?;

    // Assert
    assert_eq!(root.file(), workspace.join(expected));

    Ok(())
}

#[test]
fn explicit_manager_override_ignores_precedence() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();
    let unexpected = workspace.join("lerna.json");
    let expected = workspace.join("pnpm-workspace.yaml");
    fs::write(&unexpected, "")?;
    fs::write(&expected, "")?;

    // Act
    let root = Root::builder()
        .cwd(workspace)
        .manager(Manager::Pnpm)
        .build()?;

    // Assert
    assert_eq!(root.file(), expected);

    Ok(())
}

#[test]
fn explicit_manager_override_returns_not_found_when_manager_file_is_absent() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path();

    // Act
    let actual = Root::builder()
        .cwd(workspace)
        .manager(Manager::Pnpm)
        .ceiling(workspace)
        .build();

    // Assert
    assert!(matches!(
        actual.expect_err("pnpm root should not be found"),
        RootError::Io(error) if error.kind() == io::ErrorKind::NotFound
    ));

    Ok(())
}

#[test]
fn finds_root_when_cwd_is_ceiling_directory() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let repo = tmp.path();
    let root_file = repo.join("yarn.lock");
    fs::write(&root_file, "")?;

    // Act
    let root = Root::builder().cwd(repo).ceiling(repo).build()?;

    // Assert
    assert_eq!(root.path(), repo);
    assert_eq!(root.file(), root_file);

    Ok(())
}

#[test]
fn ceiling_prevents_searching_above_ceiling_directory() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let home = tmp.path().join("home");
    let project = home.join("projects/js-workspace");
    fs::create_dir_all(&project)?;
    fs::write(tmp.path().join("yarn.lock"), "")?;

    // Act
    let actual = Root::builder().cwd(&project).ceiling(&home).build();

    // Assert
    assert!(matches!(
        actual.expect_err("ceiling should exclude parent directory"),
        RootError::Io(error) if error.kind() == io::ErrorKind::NotFound
    ));

    Ok(())
}

#[test]
fn finds_nested_root_below_ceiling() -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let ceiling = tmp.path();
    let workspace = ceiling.join("monorepo");
    let nested = workspace.join("packages/test-package");
    let root_file = workspace.join("yarn.lock");
    fs::create_dir_all(&nested)?;
    fs::write(&root_file, "")?;

    // Act
    let root = Root::builder().cwd(&nested).ceiling(ceiling).build()?;

    // Assert
    assert_eq!(root.path(), workspace);
    assert_eq!(root.file(), root_file);

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
    let actual = Root::builder().cwd(&workspace).ceiling(&package).build();

    // Assert
    assert!(matches!(
        actual.expect_err("ceiling below cwd should be rejected"),
        RootError::Io(error) if error.kind() == io::ErrorKind::InvalidInput
    ));

    Ok(())
}
