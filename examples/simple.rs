use js_workspace::workspace;

const RUST_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn main() {
    let repo_root = std::path::Path::new(RUST_ROOT)
        .join("examples")
        .join("fake-repo");

    println!("{}", repo_root.display());

    let root = workspace::Root::new(repo_root).unwrap();
    let packages = root.packages();
}
