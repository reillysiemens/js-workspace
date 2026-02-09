use std::fs;

use anyhow::Ok;
use pretty_assertions::assert_eq;
use test_case::test_case;

use js_workspace::workspace::Root;

#[test_case("yarn.lock" ; "yarn")]
#[test_case("pnpm-workspace.yaml" ; "pnpm")]
#[test_case("rush.json" ; "rush")]
#[test_case("package-lock.json" ; "npm")]
#[test_case("lerna.json" ; "lerna")]
fn finds_root_from_workspace_root(root_file: &str) -> anyhow::Result<()> {
    // Arrange
    let tmp = tempfile::tempdir()?;
    let workspace = tmp.path().canonicalize()?;
    let root_file = workspace.join(root_file);
    fs::write(&root_file, "")?;

    // Act
    let root = Root::new(&workspace)?;

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
    let workspace = tmp.path().canonicalize()?;
    let nested = workspace.join("packages/test-package");
    let root_file = workspace.join(root_file);
    fs::write(&root_file, "")?;
    fs::create_dir_all(&nested)?;

    // Act
    let root = Root::new(&nested)?;

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
    let workspace = tmp.path().canonicalize()?;
    for file in files {
        fs::write(workspace.join(file), "")?;
    }

    // Act
    let root = Root::new(&workspace)?;

    // Assert
    assert_eq!(root.file(), workspace.join(expected));

    Ok(())
}
