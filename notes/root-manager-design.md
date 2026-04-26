# Root And Manager Design

Before merging the current root-discovery work into `main`, review these gaps
against the corresponding `workspace-tools` behavior.

## Testing notes

- The root discovery tests cover these pure builder and filesystem behaviors:
  - explicit manager override chooses that manager's root file instead of normal
    precedence;
  - explicit manager override returns `NotFound` when that manager's root file
    is absent;
  - `ceiling` includes the ceiling directory;
  - `ceiling` prevents searching above the ceiling directory;
  - a ceiling below `cwd` returns `io::ErrorKind::InvalidInput`;
  - nested roots below a ceiling are still discoverable;
  - no manager file returns `io::ErrorKind::NotFound`.
- Do not test `PREFERRED_WORKSPACE_MANAGER` by mutating the process environment
  in-process.
  - In Rust 2024, environment mutation is unsafe because the environment is
    process-global mutable state.
  - Treat `Manager::from_env()` as a small boundary around `std::env::var` plus
    the already-tested `FromStr` implementation.
  - Invalid environment values are covered by parser tests; end-to-end
    environment integration can be tested later with a subprocess if it becomes
    necessary.

## API shape

- Prefer hanging workspace operations off `Root`, not exposing the detected
  manager for callers to dispatch on.
  - `Root` should use its manager internally, e.g. `root.packages()` should
    choose the right manager-specific implementation under the hood.
  - Avoid encouraging user code like `match root.manager() { ... }`, since that
    leaks crate internals and makes higher-level behavior harder to evolve.
  - Keep `Manager` public enough for explicit discovery configuration, parsing,
    and diagnostics.
  - `Manager` implements `Display` with lowercase names matching
    `workspace-tools`: `yarn`, `pnpm`, `rush`, `npm`, `lerna`.
  - Only expose the detected manager from `Root` if a concrete introspection use
    case appears, such as CLI reporting, telemetry, or debugging.
- Keep the ergonomic high-level shape in mind:

  ```rust
  use js_workspace::workspace;

  let root = workspace::Root::builder().cwd("/repo").build()?;
  let packages = root.packages();
  ```

  This still feels like the right user-facing API, even if the internal naming
  or validation phases change later.

## Intentional behavior differences

| Area                                  | `workspace-tools`                                                    | Current Rust behavior                                  | Consideration                                                                                                                            |
| ------------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Invalid `PREFERRED_WORKSPACE_MANAGER` | Ignores invalid values and falls back to filesystem discovery.       | Returns `ParseManagerError`.                           | Stricter Rust behavior is reasonable. Cover the parsing behavior with `FromStr` tests rather than mutating process environment in tests. |
| Missing root                          | Returns `undefined` unless wrapper options request throwing.         | Returns `io::ErrorKind::NotFound`.                     | Good Rust translation: explicit failure instead of silent absence.                                                                       |
| `cwd` existence                       | `path.resolve(cwd)` works lexically even if the path does not exist. | `canonicalize()` requires the path to exist.           | Probably acceptable, but document/test if callers may pass prospective paths.                                                            |
| Symlinks                              | Mostly lexical absolute paths.                                       | Canonical paths.                                       | Better for identity, but can surprise callers expecting symlink-preserving paths.                                                        |
| Caching                               | Uses global mutable caches keyed by `cwd`.                           | No cache.                                              | Good omission for now. If needed later, prefer explicit/caller-owned caching.                                                            |
| Filesystem root search                | Does not check the filesystem root itself.                           | Checks each directory before stopping, including root. | Harmless expansion.                                                                                                                      |

## Later package-discovery concern

- Current root discovery proves only that a manager marker file was found.
  - It does not prove the directory is a valid workspace whose packages can be
    enumerated.
  - `workspace-tools` has the same shallow marker-file behavior: `yarn.lock` or
    `package-lock.json` can identify a root even if the required workspace
    configuration is absent.
  - Package discovery should decide where full workspace configuration parsing
    belongs so callers do not get a misleading "root discovery succeeded, package
    discovery immediately failed" experience.
  - Naming can be revisited later; the important design pressure is to parse
    enough configuration before promising package enumeration.

Lerna needs special handling during package discovery. Upstream detects
`lerna.json` as the workspace manager root, but if `lerna.json` lacks
`packages`, newer Lerna behavior delegates workspace patterns to the actual
package manager (`npm`, `yarn`, or `pnpm`). Keep this out of `Root`/`Manager`;
it belongs in manager-specific package pattern discovery.
