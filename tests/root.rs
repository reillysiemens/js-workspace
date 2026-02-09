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
