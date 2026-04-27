# Workspace Discovery Transformation

Package discovery creates pressure to distinguish "found a manager marker file"
from "proved this is a valid JavaScript workspace." The current `Root`/`Manager`
design is a good marker-discovery layer, but it may overpromise once package
enumeration is added.

## Proposed direction

Keep the current public `Manager` enum as the lightweight, caller-facing manager
choice, and introduce a private richer manager value for parsed workspace state:

```rust
pub enum Manager {
    Yarn,
    Pnpm,
    Rush,
    Npm,
    Lerna,
}

struct WorkspaceManager {
    kind: Manager,
    marker_file: PathBuf,
    package_layout: PackageLayout,
}

pub enum PackageLayout {
    Globs(Vec<String>),
    Paths(Vec<PathBuf>),
}

pub struct Workspace {
    root: PathBuf,
    manager: WorkspaceManager,
}
```

The high-level user-facing API should become:

```rust
use js_workspace::workspace;

let workspace = workspace::Workspace::builder().cwd("/repo").build()?;
let packages = workspace.packages()?;
```

This keeps the ergonomic "ask the workspace for packages" shape while making
the discovered value stronger than today's marker-only `Root`.

## Intended invariants

- `Manager` is the small public enum for parsing, display, discovery
  precedence, marker filenames, and typed builder overrides.
- `WorkspaceManager` is a private/internal type representing a discovered
  manager marker plus parsed workspace definition configuration.
- `Workspace::builder().build()?` should prove at least:
  - a manager marker was discovered;
  - the corresponding workspace configuration was found and parsed;
  - the configuration defines a valid package layout, such as globs or explicit
    paths.
- `Workspace::build()` should not necessarily read every child `package.json` or
  eagerly glob the filesystem. That can remain part of `packages()`.

## Marker files vs workspace configuration

Do not assume the marker file is always the workspace configuration file:

| Manager kind | Marker file           | Workspace configuration                        |
| ------------ | --------------------- | ---------------------------------------------- |
| Yarn         | `yarn.lock`           | root `package.json` `workspaces`               |
| npm          | `package-lock.json`   | root `package.json` `workspaces`               |
| pnpm         | `pnpm-workspace.yaml` | same file                                      |
| Rush         | `rush.json`           | same file                                      |
| Lerna        | `lerna.json`          | same file, or fallback to npm/yarn/pnpm config |

For this reason, `WorkspaceManager` should mean "parsed workspace definition for
this manager," not necessarily "parsed marker file."

## Migration sketch

1. Keep today's `Manager` enum as the public typed manager choice.
2. Keep marker-file discovery helpers on `Manager`.
3. Introduce an internal marker result, e.g. `{ kind, file }`, if useful.
4. Introduce a private `WorkspaceManager` that owns the marker path and package
   layout.
5. Replace the public high-level `Root` type with `Workspace`.
6. Add package-discovery parsing for package-json workspaces, pnpm, Rush, and
   Lerna package patterns or fallback workspace configuration.

## Implemented first pass

- The public high-level type is now `Workspace`; `Root` has been removed rather
  than kept as a public marker-only abstraction.
- `Manager` remains the public typed manager choice, while marker discovery now
  returns an internal marker value.
- A private `WorkspaceManager` owns the discovered marker and parsed package
  layout.
- `Workspace::build()` parses enough workspace configuration to prove a package
  layout exists, but `Workspace::packages()` performs package-path resolution.

## Open design questions

- Should `WorkspaceManager` stay entirely private, or should any manager details
  be exposed for diagnostics or tooling?
- How much package metadata should `Workspace::packages()` eventually return
  beyond the package path?
