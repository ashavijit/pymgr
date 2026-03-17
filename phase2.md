# pymgr — PRD Additions & Extended Design

> **Version:** 1.1.0
> **Status:** Draft
> **Last Updated:** 2026-03-17
> **Authors:** Engineering Team

---

## Table of Contents

1. [Workspace & Monorepo Support](#1-workspace--monorepo-support)
2. [Environment Export & Import](#2-environment-export--import)
3. [Build System Integration](#3-build-system-integration)
4. [Editable Installs](#4-editable-installs)
5. [Dependency Groups](#5-dependency-groups)
6. [Conflict Explainer UX](#6-conflict-explainer-ux)
7. [Rollback & Snapshot System](#7-rollback--snapshot-system)
8. [Offline Mode](#8-offline-mode)
9. [Telemetry Architecture](#9-telemetry-architecture)
10. [IDE Integration](#10-ide-integration)
11. [pymgr publish — Design Sketch](#11-pymgr-publish--design-sketch)
12. [Pre/Post Hooks](#12-prepost-hooks)
13. [Dependency Auditing](#13-dependency-auditing)
14. [Hash Pinning Policy](#14-hash-pinning-policy)
15. [Private Index Authentication Flows](#15-private-index-authentication-flows)
16. [Plugin & Extension API](#16-plugin--extension-api)
17. [Garbage Collection](#17-garbage-collection)
18. [Migration Guides](#18-migration-guides)
19. [Benchmarking Methodology](#19-benchmarking-methodology)

---

## 1. Workspace & Monorepo Support

### Overview

Modern teams increasingly work in monorepos. A workspace is a root directory containing multiple Python packages that share a single resolved dependency graph. Without first-class workspace support, teams must either duplicate lockfiles across sub-packages or fall back to a single flat environment — both are painful.

pymgr workspaces are intentionally modelled after Cargo workspaces, the gold standard in the space.

### User Stories

```
As a monorepo engineer,
I want all sub-packages to share one resolved dependency graph,
So that I don't have version conflicts between packages in the same repo.

Acceptance Criteria:
- `pymgr workspace init` scaffolds a root pymgr.toml with [workspace] table.
- `pymgr install` at the root resolves all members together.
- Each member can still be developed in isolation.
```

```
As a CI engineer,
I want to install only the dependencies needed for a specific sub-package,
So that my pipeline is fast and doesn't install unrelated packages.

Acceptance Criteria:
- `pymgr install --package api-service` installs only that member's deps.
- Transitive shared deps are still resolved from the workspace lockfile.
```

### `pymgr.toml` Workspace Configuration

```toml
# Root pymgr.toml
[workspace]
members = [
  "packages/api",
  "packages/worker",
  "packages/shared",
  "tools/*",          # Glob patterns supported
]

# Optional: exclude specific paths
exclude = [
  "tools/legacy-tool",
]

# Workspace-level Python version (members can override)
[workspace.python]
version = "3.12"

# Shared dev dependencies available to all members
[workspace.dev-dependencies]
pytest = ">=8.0"
ruff = "*"
mypy = "*"
```

Each member has its own `pyproject.toml`:

```toml
# packages/api/pyproject.toml
[tool.pymgr]
name = "api"
version = "0.1.0"

[tool.pymgr.dependencies]
fastapi = ">=0.110"
shared = { workspace = true }   # Refers to packages/shared in the workspace
```

### Workspace Resolution Algorithm

```
1. Parse root pymgr.toml → collect workspace members
2. For each member, parse pyproject.toml → collect requirements
3. Merge all requirements into a single universe
4. Run PubGrub solver over the merged universe
5. Detect intra-workspace references → link them as editable installs
6. Write single root pymgr.lock covering all members
7. Create per-member environments OR a single shared env (configurable)
```

### CLI Commands

```
pymgr workspace init              Scaffold workspace in current directory
pymgr workspace list              List all workspace members
pymgr workspace run <pkg> <cmd>   Run command in a specific member's env
pymgr workspace exec <cmd>        Run command across all members
pymgr install --package <name>    Install deps for a single member only
pymgr add <dep> --package <name>  Add dep to a specific member
```

### Environment Strategies

| Strategy | Description | When to Use |
|---|---|---|
| `shared` (default) | Single env for all members | Small monorepos, no version conflicts |
| `isolated` | Separate env per member | Large monorepos, conflicting needs |
| `layered` | Base env + per-member overlays | Advanced: balance speed vs isolation |

Configure in root `pymgr.toml`:

```toml
[workspace]
env-strategy = "isolated"   # "shared" | "isolated" | "layered"
```

### Intra-Workspace Dependencies

When `packages/api` depends on `packages/shared`, pymgr automatically installs the local package as an editable install, pointing directly at the source directory. No manual `pip install -e` required.

```
workspace root/
├── pymgr.toml       ← [workspace] definition
├── pymgr.lock       ← single lockfile for all members
├── packages/
│   ├── api/
│   │   ├── pyproject.toml
│   │   └── src/api/
│   ├── worker/
│   │   ├── pyproject.toml
│   │   └── src/worker/
│   └── shared/
│       ├── pyproject.toml
│       └── src/shared/
```

---

## 2. Environment Export & Import

### Overview

pymgr's native lockfile format (`pymgr.lock`) is the canonical source of truth, but the ecosystem is full of `requirements.txt` files. Migration path quality determines adoption. Export/import bridges pymgr to legacy pip workflows, CI templates, and tools that only understand `requirements.txt`.

### Export

```
pymgr export [FORMAT] [OPTIONS]

FORMATS:
  requirements        requirements.txt (default)
  constraints         constraints.txt (only version pins, no deps)
  conda               environment.yml for conda
  poetry              pyproject.toml [tool.poetry] format

OPTIONS:
  --dev               Include dev dependencies
  --group <name>      Export a specific dependency group only
  --extras <list>     Include extras (e.g. requests[security])
  --python <version>  Target Python version for platform markers
  --output <file>     Write to file instead of stdout
  --hashes            Include --hash= directives (pip --require-hashes compatible)
```

**Example output — `requirements.txt` with hashes:**

```
# Generated by pymgr 0.3.0 — do not edit manually
# pymgr export requirements --hashes

numpy==1.26.4 \
    --hash=sha256:abc123def456... \
    --hash=sha256:789ghi012jkl...
pandas==2.2.1 \
    --hash=sha256:mno345pqr678...
python-dateutil==2.9.0 \
    --hash=sha256:stu901vwx234...
```

**Example output — `environment.yml`:**

```yaml
# Generated by pymgr 0.3.0
name: my-project
channels:
  - defaults
  - conda-forge
dependencies:
  - python=3.11.9
  - pip
  - pip:
    - numpy==1.26.4
    - pandas==2.2.1
```

### Import

```
pymgr import [FILE] [OPTIONS]

OPTIONS:
  --from <format>     Hint the source format (auto-detected by default)
  --dev               Treat all packages as dev dependencies
  --group <name>      Import into a named dependency group
  --overwrite         Replace existing dependencies (default: merge)
  --dry-run           Show what would change without modifying pyproject.toml
```

Import reads `requirements.txt`, resolves the packages through pymgr's resolver (to get full transitive closure), and writes `pymgr.lock` and `pyproject.toml`.

**Migration workflow:**

```bash
# Step 1: Import existing requirements
pymgr import requirements.txt

# Step 2: Review the generated pyproject.toml
cat pyproject.toml

# Step 3: Sync the environment
pymgr sync

# Done — full pymgr workflow from here
```

### Round-Trip Guarantee

pymgr guarantees that:

```
pymgr export requirements | pip install -r /dev/stdin
```

…produces an environment functionally identical to `pymgr install`. This is verified in the integration test suite with a round-trip test.

---

## 3. Build System Integration

### Overview

The PRD's v1 scope focuses on pure-Python wheel installs — the common case. But many popular packages (numpy, scipy, Pillow, cryptography) have C extensions and require a build step when no pre-built wheel exists for the target platform. Without handling this, pymgr silently fails on these packages in edge cases (unusual platforms, custom CPython builds, or sdist-only packages).

### When Build is Required

pymgr prefers wheels. Build is only triggered when:

1. No wheel is available on PyPI for the current `python_version` + `platform` + `abi` tag tuple.
2. The user explicitly requests source install: `pymgr add <pkg> --no-binary`.
3. A local path dependency is specified (always built from source).

### Build Backends

pymgr delegates to the package's declared build backend via PEP 517/518:

```
pyproject.toml [build-system] table
         │
         ▼
pymgr invokes: python -m build --wheel --no-isolation
         │
   ┌─────┴──────────────────────┐
   │  setuptools (legacy)       │
   │  hatchling                 │
   │  flit-core                 │
   │  meson-python (C/Fortran)  │
   │  scikit-build-core (CMake) │
   └────────────────────────────┘
```

pymgr does **not** implement a build backend — it invokes them as a subprocess. This is intentional: build backends are complex, version-sensitive, and not pymgr's responsibility.

### Build Environment Isolation

Each build runs in an ephemeral isolated environment:

```rust
// Build isolation pseudocode
async fn build_sdist(pkg: &Package) -> Result<WheelPath> {
    let build_env = create_temp_env(pkg.requires_python).await?;
    install_build_deps(&build_env, &pkg.build_requires).await?;
    let wheel = invoke_pep517_build(&build_env, &pkg.source_path).await?;
    cleanup_build_env(build_env).await?;
    Ok(wheel)
}
```

### Build Cache

Successfully built wheels are cached in `~/.pymgr/cache/wheels/` with a cache key of `(package_name, version, python_version, platform_tag)`. Subsequent installs of the same package on the same platform skip the build.

### Error Handling for Build Failures

Build failures are common (missing system libraries, incompatible compilers). pymgr provides actionable output:

```
Error [E203]: Failed to build wheel for 'cryptography==42.0.0'.

  Build output:
    error: could not find library 'openssl'

  To fix:
    • On Ubuntu/Debian: sudo apt install libssl-dev
    • On macOS: brew install openssl
    • On Windows: install OpenSSL from https://slproweb.com/products/Win32OpenSSL.html

  Alternatively, try: pymgr add cryptography --prefer-binary
    (uses pre-built wheels even if newer versions exist)
```

### Build-Related CLI Flags

```
--no-binary <pkg>       Force source build for a specific package
--only-binary <pkg>     Fail if no wheel available (never build)
--prefer-binary         Prefer older wheel over newer sdist (default for most)
--build-isolation       Enable/disable PEP 517 isolation (default: on)
```

---

## 4. Editable Installs

### Overview

During local development, a package under active development should be importable without reinstalling after every code change. Editable installs solve this by making the package importable directly from its source directory.

### How It Works

pymgr supports two mechanisms depending on the build backend:

**Mechanism 1: `.pth` file (pure Python, legacy)**

Creates a `.pth` file in `site-packages` pointing to the source root:

```
# .pymgr/env/lib/python3.11/site-packages/my-package.pth
/home/user/projects/my-package/src
```

Python automatically adds `.pth` paths to `sys.path` at interpreter startup. Zero overhead at runtime.

**Mechanism 2: `direct_url.json` + `editable_pth` (PEP 660, modern)**

For build backends that implement PEP 660 (`hatchling`, `flit-core`, etc.), pymgr invokes the backend's editable install hook:

```bash
python -m pip install --editable . --no-build-isolation
```

This produces a proper dist-info directory, making the package discoverable by tools like `pip list` and `importlib.metadata`.

### CLI Usage

```bash
# Install a local package as editable
pymgr add -e ./packages/my-lib
pymgr add --editable ./packages/my-lib

# Install from a local path without editable (copies files)
pymgr add ./packages/my-lib

# Editable install from a Git URL
pymgr add -e git+https://github.com/org/repo.git@main
```

### `pymgr.lock` Representation

Editable installs are recorded in the lockfile with a `source = "editable"` marker and the resolved absolute path:

```toml
[[package]]
name = "my-lib"
version = "0.1.0"
source = "editable"
path = "./packages/my-lib"       # Relative to workspace root
editable = true
dependencies = [
  { name = "requests", version = ">=2.28" }
]
```

### Caveats & Warnings

- Editable installs are **not reproducible** across machines if the path doesn't exist. pymgr warns during `pymgr install --frozen` if editable packages are in the lockfile.
- Editable installs of packages with C extensions require a build step; changes to C code require rebuilding. pymgr surfaces this clearly.

---

## 5. Dependency Groups

### Overview

The PRD supports `[tool.pymgr.dependencies]` and `[tool.pymgr.dev-dependencies]` — a binary split. Real projects have richer dependency structures: test runners, documentation generators, linting tools, type checkers, and release tools are all distinct sets. Conflating them into "dev" creates unnecessary bloat in CI environments.

### Named Groups

```toml
# pyproject.toml
[tool.pymgr.dependencies]
fastapi = ">=0.110"
sqlalchemy = ">=2.0"

[tool.pymgr.groups.test]
pytest = ">=8.0"
pytest-asyncio = "*"
httpx = "*"              # For FastAPI test client

[tool.pymgr.groups.lint]
ruff = "*"
mypy = ">=1.8"
pre-commit = "*"

[tool.pymgr.groups.docs]
mkdocs = "*"
mkdocs-material = "*"

[tool.pymgr.groups.release]
twine = "*"
build = "*"
```

### CLI Usage

```bash
# Install base deps + test group
pymgr install --group test

# Install multiple groups
pymgr install --group test --group lint

# Install all groups
pymgr install --all-groups

# Add a package to a specific group
pymgr add ruff --group lint

# Remove a package from a group
pymgr remove ruff --group lint

# Show what groups are defined
pymgr group list

# Show packages in a group
pymgr group show lint
```

### Group Resolution

Groups are resolved independently but share the same base dependency graph. This means:

- A package in `[groups.test]` will respect version constraints set in `[dependencies]`.
- Groups do not override base dependencies — they extend the graph.
- Conflicts between groups are reported clearly:

```
Error [E201]: Conflict between groups 'test' and 'docs'.
  httpx is required by test (>=0.25) and docs (>=0.24).
  These constraints are compatible — upgrading httpx to 0.27.0 resolves both.
  Run `pymgr install --group test --group docs` to install both groups.
```

### `pymgr.lock` Group Sections

```toml
[metadata.groups]
default = ["fastapi", "sqlalchemy"]
test    = ["pytest", "pytest-asyncio", "httpx"]
lint    = ["ruff", "mypy", "pre-commit"]
docs    = ["mkdocs", "mkdocs-material"]

[[package]]
name = "pytest"
version = "8.1.1"
groups = ["test"]
...
```

### CI Usage Pattern

```yaml
# .github/workflows/test.yml
- name: Install test deps
  run: pymgr install --group test

# .github/workflows/docs.yml
- name: Install docs deps
  run: pymgr install --group docs
```

---

## 6. Conflict Explainer UX

### Overview

Dependency conflicts are among the most frustrating experiences in Python development. The PubGrub algorithm pymgr uses internally already generates a complete derivation tree explaining why a conflict exists. This section defines how to surface that information as human-readable, actionable output — not just a cryptic error.

### Conflict Types

| Type | Description |
|---|---|
| **Version incompatibility** | Package A requires dep `>=2.0`, Package B requires dep `<2.0` |
| **Python version incompatibility** | Package requires Python >=3.10, env uses 3.9 |
| **Platform incompatibility** | Package only available on Linux, running on Windows |
| **Circular dependency** | A depends on B which depends on A (rare but possible) |
| **Yanked version** | Only matching version was yanked from PyPI |

### Conflict Explanation Format

**Human mode (default):**

```
Error [E201]: Cannot resolve dependencies.

  Conflict: httpx 0.27 is incompatible with your requirements.

  Here's why:
    your project requires httpx >=0.25
      because fastapi 0.115.0 requires httpx >=0.25
    your project requires httpx <0.27
      because stripe 8.0.0 requires httpx >=0.24, <0.27

  Dependency chain:
    fastapi 0.115.0
      └─ httpx >=0.25        ← requires newer httpx
    stripe 8.0.0
      └─ httpx >=0.24, <0.27 ← blocks httpx 0.27

  Possible fixes:
    1. Downgrade fastapi: pymgr add 'fastapi<0.115'
       (fastapi 0.110.0 requires httpx >=0.24 — compatible with stripe)

    2. Upgrade stripe: pymgr add 'stripe>=9.0'
       (stripe 9.0.0 relaxes httpx constraint to >=0.24, <0.28)

    3. Override httpx version (not recommended — may cause runtime issues):
       pymgr add 'httpx>=0.25,<0.27'
```

**JSON mode (`--json`):**

```json
{
  "error": {
    "code": "E201",
    "category": "resolution_conflict",
    "conflict": {
      "package": "httpx",
      "incompatible_constraints": [
        { "required_by": "fastapi==0.115.0", "constraint": ">=0.25" },
        { "required_by": "stripe==8.0.0",    "constraint": ">=0.24,<0.27" }
      ],
      "derivation_tree": {
        "root": "your project",
        "children": [
          {
            "package": "fastapi",
            "version": "0.115.0",
            "via": "direct dependency",
            "children": [
              { "package": "httpx", "constraint": ">=0.25", "via": "fastapi requires" }
            ]
          },
          {
            "package": "stripe",
            "version": "8.0.0",
            "via": "direct dependency",
            "children": [
              { "package": "httpx", "constraint": ">=0.24,<0.27", "via": "stripe requires" }
            ]
          }
        ]
      }
    },
    "suggestions": [
      {
        "description": "Downgrade fastapi to 0.110.0",
        "command": "pymgr add 'fastapi<0.115'"
      },
      {
        "description": "Upgrade stripe to 9.0.0",
        "command": "pymgr add 'stripe>=9.0'"
      }
    ]
  }
}
```

### `pymgr why` Command

A dedicated command for investigating dependency chains without triggering a full conflict:

```bash
# Why is httpx installed?
pymgr why httpx

# Output:
httpx 0.26.0 is installed because:
  ├─ fastapi 0.115.0 requires httpx >=0.24
  └─ your project requires httpx >=0.25 (direct dependency)

# Why is this specific version installed?
pymgr why httpx==0.26.0

# What requires a specific version constraint?
pymgr why --constraint 'httpx<0.27'
```

### Conflict Resolution Assistant

For especially complex conflicts (3+ packages, transitive chains), pymgr calls the conflict summary through a structured output mode that suggests an escape hatch:

```
Note: This conflict is complex (4 packages involved).
Run `pymgr resolve --explain` to see the full derivation tree.
Or use `pymgr resolve --relax` to find the least-constrained solution that satisfies most requirements.
```

---

## 7. Rollback & Snapshot System

### Overview

Package operations are destructive mutations. `pymgr add foo` changes both the environment and the lockfile. If `foo` introduces a runtime incompatibility — even if it resolved without conflict — the developer needs a fast, reliable way back to the last known-good state.

### Design

pymgr uses a **snapshot-before-mutate** strategy. Before any mutating operation, the current lockfile and environment metadata are snapshotted atomically.

```
~/.pymgr/snapshots/<project-hash>/
├── 001/
│   ├── pymgr.lock
│   ├── env.json
│   └── timestamp.txt   # "2026-03-14T10:00:00Z — before: pymgr add numpy"
├── 002/
│   ├── pymgr.lock
│   ├── env.json
│   └── timestamp.txt   # "2026-03-14T10:05:00Z — before: pymgr add pandas"
└── current -> 002/
```

The snapshot stores the **lockfile** and **metadata**, not the full environment. Rollback re-installs from the previous lockfile snapshot, which is fast due to the wheel cache.

### CLI Commands

```bash
# List snapshots for current project
pymgr snapshot list

# Output:
  #001  2026-03-14 10:00  before: pymgr add numpy
  #002  2026-03-14 10:05  before: pymgr add pandas     ← current
  #003  2026-03-14 10:10  before: pymgr remove requests

# Roll back to previous snapshot
pymgr rollback

# Roll back to a specific snapshot
pymgr rollback 001

# Show diff between current and a snapshot
pymgr snapshot diff 001

# Delete all snapshots (free disk space)
pymgr snapshot gc
```

### Snapshot Diff Output

```
pymgr snapshot diff 001

  Changes from snapshot #001 → current:
    + numpy  1.26.4  (added)
    + pandas 2.2.1   (added)
    ~ httpx  0.25.0 → 0.26.0  (upgraded as transitive side-effect)
    - requests 2.31.0  (removed)
```

### Snapshot Policy

- Maximum 20 snapshots retained per project (configurable in `~/.pymgr/config.toml`).
- Snapshots are garbage-collected automatically when the limit is exceeded (oldest first).
- Snapshots are **not** committed to version control — they live in `~/.pymgr/` alongside the wheel cache.

### Transaction Semantics

Mutating operations (add, remove, update, sync) follow a transactional model:

```
1. Snapshot current state
2. Modify lockfile in memory
3. Install/uninstall packages
4. If step 3 fails → automatically restore snapshot
5. If step 3 succeeds → commit new snapshot
```

Step 4 ensures the environment is never left in a partial state.

---

## 8. Offline Mode

### Overview

CI environments behind a firewall, air-gapped servers, and airplane development all share the same need: pymgr must work without network access, failing fast and loudly rather than hanging on a connection timeout.

### `--offline` Flag

Available globally on all commands:

```bash
pymgr install --offline
pymgr add numpy --offline     # Fails if numpy not in cache
pymgr sync --offline
```

### Behavior in Offline Mode

| Situation | Default (online) | `--offline` |
|---|---|---|
| Package in wheel cache | Use cache (0 network) | Use cache (0 network) |
| Package metadata in cache | Use cache if TTL valid | Always use cache |
| Package not in cache | Fetch from PyPI | Fail with E302 |
| Python not installed | Download from python.org | Fail with E003 |
| `pymgr.lock` stale | Re-resolve from PyPI | Fail with E400 |

### Error Messages in Offline Mode

```
Error [E302]: Cannot install 'scipy==1.13.0' — package not in cache.

  You are running in offline mode (--offline).

  To cache this package for offline use, run while online:
    pymgr add scipy --cache-only   (download without installing)
  Or:
    pymgr cache warm scipy         (pre-warm cache for a package)
```

### Cache Warming

Pre-populate the cache for offline use:

```bash
# Cache all packages in the lockfile without installing
pymgr cache warm

# Cache a specific package and its dependencies
pymgr cache warm numpy pandas scipy

# Cache packages for multiple Python versions / platforms
pymgr cache warm numpy --python 3.11 --python 3.12
```

### `~/.pymgr/config.toml` — Offline Policy

```toml
[network]
offline = false           # Global offline mode (default: false)
timeout-secs = 30         # Connection timeout
retry-attempts = 3        # Retry on transient errors
metadata-ttl-secs = 300   # 5 minutes (0 = always revalidate)
```

---

## 9. Telemetry Architecture

### Overview

The PRD's success metrics include a "crash rate < 0.1%" measured via opt-in telemetry, but provides no design. Telemetry done poorly erodes trust and drives users away. This section defines exactly what is collected, how consent is obtained, how data is stored, and how to opt out — making the policy explicit and auditable.

### Opt-In Consent

Telemetry is **disabled by default**. On first run, pymgr prints a one-time prompt:

```
pymgr collects anonymous crash reports and usage statistics to improve the tool.
No personal data, file paths, or package names are ever collected.

  Enable telemetry? [y/N]: _
```

The choice is stored in `~/.pymgr/config.toml`:

```toml
[telemetry]
enabled = false                      # Set to true if user consents
install-id = "a1b2c3d4e5f6..."       # Random UUID, never linked to identity
```

Telemetry can also be controlled via environment variable: `PYMGR_TELEMETRY=0` disables it regardless of config.

### What Is Collected

**Collected:**

| Field | Example | Rationale |
|---|---|---|
| Command name | `install` | Which features are used |
| Exit code | `0` / `E201` | Failure rate per command |
| Duration (ms) | `342` | Performance tracking |
| OS family | `linux` / `macos` / `windows` | Platform coverage |
| pymgr version | `0.3.1` | Version-specific bug tracking |
| Python version | `3.11` | Not patch version |
| Anonymized install ID | `a1b2c3...` | Cohort analysis, no identity |

**Never collected:**

- Package names or versions
- File paths, project names, directory names
- IP addresses (stripped at ingestion)
- Environment variables
- Any user-identifiable data

### Data Pipeline

```
pymgr binary
    │ (HTTPS POST, fire-and-forget, 5s timeout)
    ▼
telemetry.pymgr.dev/ingest
    │ (strip IP, validate schema)
    ▼
ClickHouse (aggregate only)
    │ (daily rollup job)
    ▼
Public dashboard (pymgr.dev/stats)
```

The public dashboard shows aggregate statistics only — no individual event records are exposed.

### Telemetry Payload Schema

```json
{
  "v": 1,
  "id": "a1b2c3d4e5f6",
  "pymgr": "0.3.1",
  "os": "linux",
  "arch": "x86_64",
  "python": "3.11",
  "cmd": "install",
  "exit_code": 0,
  "duration_ms": 342,
  "ts": "2026-03-14T10:00:00Z"
}
```

### Audit

The telemetry source code lives in `src/telemetry.rs` and is clearly separated from all other modules. The ingestion server source is open-source. This makes the "what we collect" claim independently verifiable.

---

## 10. IDE Integration

### Overview

Python IDEs need to discover the interpreter path and site-packages for IntelliSense, type checking, and test discovery. Without a clear integration story, developers manually copy paths and re-configure their IDE after every environment change — a friction point that hurts adoption.

### Interpreter Discovery Protocol

pymgr implements the **PEP 514 / CEP 512 interpreter discovery protocol**, making it discoverable by VS Code, PyCharm, and any compliant tool.

```bash
# Machine-readable interpreter info (used by IDEs)
pymgr env info --json

# Output:
{
  "interpreter": "/home/user/project/.pymgr/env/bin/python",
  "version": "3.11.9",
  "site_packages": "/home/user/project/.pymgr/env/lib/python3.11/site-packages",
  "activated": true,
  "project_root": "/home/user/project"
}
```

### VS Code Integration

```bash
# Write .vscode/settings.json with correct interpreter path
pymgr ide vscode

# Output (to .vscode/settings.json):
{
  "python.defaultInterpreterPath": ".pymgr/env/bin/python",
  "python.terminal.activateEnvironment": false,
  "python.testing.pytestEnabled": true,
  "python.testing.pytestPath": ".pymgr/env/bin/pytest"
}
```

Appends to existing `.vscode/settings.json` rather than overwriting, preserving other settings.

### PyCharm Integration

PyCharm discovers interpreters from `pyvenv.cfg`. pymgr already writes this file on env creation (section 8.1), so PyCharm picks it up automatically when the project is opened. No additional action needed.

### Additional IDE Commands

```bash
pymgr ide vscode        Write .vscode/settings.json
pymgr ide pycharm       Print interpreter path for manual entry (auto-discovery works)
pymgr ide zed           Write .zed/settings.json
pymgr ide helix         Print interpreter path
pymgr ide cursor        Same as vscode (Cursor is VS Code-compatible)
```

### `pyrightconfig.json` / `mypy.ini` Auto-Configuration

```bash
# Configure pyright to use the project's environment
pymgr ide pyright

# Writes pyrightconfig.json:
{
  "venvPath": ".",
  "venv": ".pymgr/env",
  "pythonVersion": "3.11"
}
```

---

## 11. pymgr publish — Design Sketch

### Overview

`pymgr publish` is listed as a v1.x future feature. This section provides enough design detail to scope the work and avoid API decisions in v1 that would break it.

### Scope

`pymgr publish` covers:

1. **Building** — produce a wheel and sdist from the current project.
2. **Checking** — validate the distribution (metadata, README rendering).
3. **Uploading** — push to PyPI or a private index.

pymgr does **not** implement a build backend. It delegates to the project's declared backend (setuptools, hatchling, etc.) via PEP 517.

### CLI Design

```
pymgr build [OPTIONS]
  --wheel             Build wheel only (default: wheel + sdist)
  --sdist             Build sdist only
  --out <dir>         Output directory (default: dist/)
  --clean             Remove dist/ before building

pymgr publish [OPTIONS]
  --index <url>       Target index (default: https://upload.pypi.org/legacy/)
  --token <token>     API token (or set PYMGR_PUBLISH_TOKEN env var)
  --dry-run           Check without uploading
  --skip-existing     Don't error if version already published
  dist/*.whl          Specific files to publish (default: all in dist/)
```

### Build Pipeline

```
pymgr build
      │
      ▼
Read pyproject.toml [build-system]
      │
      ▼
Install build requirements into isolated env
      │
      ▼
PEP 517: call build_wheel() hook
      │
      ▼
PEP 517: call build_sdist() hook
      │
      ▼
Validate wheel (check METADATA, RECORD, etc.)
      │
      ▼
Output to dist/
```

### Version Bumping

A convenience command for common version workflows:

```bash
pymgr version patch   # 1.2.3 → 1.2.4
pymgr version minor   # 1.2.3 → 1.3.0
pymgr version major   # 1.2.3 → 2.0.0
pymgr version 2.0.0   # Set explicitly
```

Updates `version` in `pyproject.toml` and creates a git tag (with `--tag` flag).

---

## 12. Pre/Post Hooks

### Overview

Workflows often require actions triggered by pymgr operations: regenerating type stubs after a package install, running a formatter after a dependency change, or notifying a developer tool. Hooks provide a lightweight way to attach these actions without wrapping pymgr in a shell script.

### Hook Configuration

Hooks are defined in `pymgr.toml`:

```toml
[hooks]
post-add     = "ruff format . && mypy src/"
post-install = "python scripts/generate_stubs.py"
post-sync    = "echo 'Environment synced at $(date)' >> .pymgr/sync.log"
pre-remove   = "echo 'Removing package...'"
post-update  = "python -c 'import sys; print(sys.version)'"
```

All hooks are run in the project's activated environment, so `ruff`, `mypy`, etc. resolve to the env's installed tools.

### Available Hook Points

| Hook | Trigger |
|---|---|
| `pre-init` | Before environment creation |
| `post-init` | After environment creation |
| `pre-add` | Before adding a package |
| `post-add` | After adding a package |
| `pre-remove` | Before removing a package |
| `post-remove` | After removing a package |
| `pre-install` | Before bulk install |
| `post-install` | After bulk install |
| `pre-sync` | Before sync |
| `post-sync` | After sync |
| `pre-update` | Before update |
| `post-update` | After update |

### Hook Environment Variables

Hooks receive context via environment variables:

```bash
# Available in all hooks:
PYMGR_VERSION=0.3.1
PYMGR_PROJECT_ROOT=/home/user/project
PYMGR_ENV_PATH=/home/user/project/.pymgr/env
PYMGR_PYTHON_VERSION=3.11.9

# Available in package hooks (add/remove/update):
PYMGR_PACKAGE_NAME=numpy
PYMGR_PACKAGE_VERSION=1.26.4
PYMGR_PACKAGE_PREVIOUS_VERSION=1.25.0   # For update hooks
```

### Hook Failure Behavior

Hook failure (non-zero exit code) does **not** roll back the pymgr operation by default. This can be changed:

```toml
[hooks]
post-add = "mypy src/"
post-add-on-failure = "warn"   # "warn" | "error" | "ignore" (default: warn)
```

With `"error"`, a failed `post-add` hook causes pymgr to roll back the package addition.

### Skipping Hooks

```bash
pymgr add numpy --no-hooks      # Skip all hooks for this run
pymgr install --no-post-hooks   # Skip only post-install hook
```

---

## 13. Dependency Auditing

### Overview

`pymgr audit` checks installed packages against public vulnerability databases and surfaces known CVEs. This is a first-class safety feature, not an afterthought. Integrating it natively (rather than relying on `pip-audit` as an external tool) means it works in `--offline` mode (with a local advisory cache), integrates with the lockfile, and produces structured JSON output for CI pipelines.

### Data Sources

pymgr queries two advisory databases:

1. **OSV (Open Source Vulnerabilities)** — `https://api.osv.dev/v1/querybatch` — Google-maintained, covers PyPI, GitHub, and more.
2. **PyPI Advisory Database** — `https://github.com/pypa/advisory-database` — PyPI-native, frequently updated.

Advisories are cached locally for offline use:

```
~/.pymgr/cache/advisories/
├── osv/
│   └── 2026-03-14.json   # Daily snapshot
└── pypi/
    └── 2026-03-14.json
```

### CLI Usage

```bash
# Audit current environment
pymgr audit

# Audit and fail in CI if vulnerabilities found
pymgr audit --error-on-high     # Exit 1 if HIGH or CRITICAL found
pymgr audit --error-on-medium   # Exit 1 if MEDIUM or above found

# Audit a specific package
pymgr audit numpy

# JSON output for pipeline integration
pymgr audit --json
```

### Output Format

**Human mode:**

```
pymgr audit — scanning 47 packages

  CRITICAL  requests 2.28.0 — CVE-2024-35195
    Severity:    CRITICAL (CVSS 9.1)
    Description: Certificate verification bypass via proxy
    Fixed in:    requests 2.32.0
    Action:      pymgr add 'requests>=2.32.0'

  HIGH      cryptography 42.0.0 — GHSA-jm77-qphf-c4w8
    Severity:    HIGH (CVSS 7.4)
    Description: NULL dereference in PKCS12 parsing
    Fixed in:    cryptography 42.0.4
    Action:      pymgr add 'cryptography>=42.0.4'

  2 vulnerabilities found (1 CRITICAL, 1 HIGH).
  Run `pymgr audit --fix` to upgrade affected packages.
```

**JSON mode:**

```json
{
  "scanned": 47,
  "vulnerabilities": [
    {
      "package": "requests",
      "installed_version": "2.28.0",
      "advisory_id": "CVE-2024-35195",
      "severity": "CRITICAL",
      "cvss_score": 9.1,
      "description": "Certificate verification bypass via proxy",
      "fixed_in": "2.32.0",
      "fix_command": "pymgr add 'requests>=2.32.0'",
      "references": ["https://nvd.nist.gov/vuln/detail/CVE-2024-35195"]
    }
  ],
  "summary": { "critical": 1, "high": 1, "medium": 0, "low": 0 }
}
```

### `pymgr audit --fix`

Automatically upgrades vulnerable packages to the minimum safe version:

```bash
pymgr audit --fix

  Fixing 2 vulnerabilities:
    requests  2.28.0 → 2.32.0  ✓
    cryptography 42.0.0 → 42.0.4  ✓

  Re-running audit... No vulnerabilities found.
```

### `.pymgr-ignore` — Ignoring False Positives

```toml
# .pymgr-ignore
[[ignore]]
advisory = "CVE-2024-12345"
reason   = "Not exploitable in our usage — we do not use the affected code path"
until    = "2026-06-01"   # Re-surface after this date

[[ignore]]
package  = "some-internal-package"
reason   = "Internal package, advisory does not apply"
```

### CI Integration

```yaml
# GitHub Actions
- name: Security audit
  run: pymgr audit --error-on-high --json > audit-report.json

- name: Upload audit report
  uses: actions/upload-artifact@v4
  with:
    name: security-audit
    path: audit-report.json
```

---

## 14. Hash Pinning Policy

### Overview

The PRD specifies SHA-256 per package in `pymgr.lock`, but doesn't define what happens when a hash changes, a release is yanked, or a mismatch is detected. These scenarios are security-critical and need explicit policy.

### Scenarios & Policy

**Scenario 1: Hash mismatch during install**

The downloaded wheel's SHA-256 doesn't match the lockfile.

```
Error [E202]: Hash mismatch for 'numpy==1.26.4'.

  Expected (from pymgr.lock): abc123def456...
  Got (downloaded):           aaa000bbb111...

  This could indicate:
    1. A tampered or corrupted download.
    2. The package was re-uploaded (rare but happens on PyPI).
    3. A network interception (MITM attack).

  Do NOT proceed without investigating. If you believe this is legitimate
  (e.g. a known re-upload), run: pymgr install --refresh-hashes
```

pymgr **never** automatically accepts a mismatched hash. It always stops and requires explicit user action.

**Scenario 2: Yanked release**

PyPI allows maintainers to "yank" a release — marking it as deprecated but not fully removing it.

```
Warning [W101]: 'numpy==1.26.0' has been yanked from PyPI.
  Reason: "Critical bug in random number generation on Windows"
  Pinned in: pymgr.lock

  Your lockfile pins this version. To upgrade:
    pymgr update numpy
```

pymgr installs yanked packages only if the lockfile explicitly pins that version (for reproducibility), but always warns. `pymgr update` resolves to a non-yanked version.

**Scenario 3: Package removed from PyPI (rare)**

```
Error [E300]: 'deleted-package==1.0.0' is no longer available on PyPI.

  It was present when your lockfile was generated but has since been removed.

  Options:
    1. Remove it: pymgr remove deleted-package
    2. Use a mirror or local cache: pymgr install --offline (if cached)
    3. Vendor it into your repo and add as a local path dependency
```

### `--refresh-hashes` Flag

When a package is legitimately re-uploaded (maintainer corrects a malformed wheel without a version bump — allowed by PyPI policy), hashes must be refreshed:

```bash
pymgr install --refresh-hashes          # Re-fetch all hashes
pymgr install --refresh-hashes numpy   # Re-fetch hash for one package
```

This re-downloads package metadata from PyPI and updates `pymgr.lock`. It **requires explicit user invocation** — it is never automatic.

### Multiple Hash Algorithms

pymgr stores SHA-256 by default. For environments requiring stricter compliance, SHA-512 is also supported:

```toml
[metadata]
hash-algorithm = "sha256"   # "sha256" | "sha512"
```

---

## 15. Private Index Authentication Flows

### Overview

Enterprise teams use private package indices: Nexus, Artifactory, Azure Artifacts, AWS CodeArtifact, Google Artifact Registry, and self-hosted DevPI. The PRD mentions these as v1.0 targets but provides no design. This section defines the authentication model.

### Index Configuration

```toml
# pyproject.toml or pymgr.toml
[[tool.pymgr.sources]]
name    = "pypi"
url     = "https://pypi.org/simple/"
default = true

[[tool.pymgr.sources]]
name    = "corporate"
url     = "https://artifacts.corp.example.com/api/pypi/pypi/simple/"
auth    = "keyring"   # "keyring" | "env" | "netrc" | "token"
```

### Authentication Methods

**Method 1: Token (API token in credentials file)**

```toml
# ~/.pymgr/credentials.toml  (chmod 600)
[[index]]
name  = "corporate"
token = "pypi-AgEIcHlwaS5vcmcA..."
```

**Method 2: Environment variables**

```bash
export PYMGR_INDEX_CORPORATE_TOKEN="pypi-AgEI..."
export PYMGR_INDEX_CORPORATE_USERNAME="user"
export PYMGR_INDEX_CORPORATE_PASSWORD="pass"
```

Variable naming convention: `PYMGR_INDEX_<NAME>_<FIELD>` where `<NAME>` is the index name uppercased.

**Method 3: `.netrc`**

pymgr reads `~/.netrc` (or `%USERPROFILE%\_netrc` on Windows) for username/password authentication:

```
machine artifacts.corp.example.com
login   myuser
password mypassword
```

**Method 4: OS Keychain (v2 feature)**

On macOS: Keychain Access. On Linux: Secret Service (GNOME Keyring, KWallet). On Windows: Windows Credential Manager.

```bash
# Store credentials in OS keychain
pymgr auth add corporate --token pypi-AgEI...

# Remove from keychain
pymgr auth remove corporate

# List configured indices
pymgr auth list
```

### Token Refresh — AWS CodeArtifact

AWS CodeArtifact tokens expire every 12 hours. pymgr supports auto-refresh via the AWS CLI:

```toml
[[tool.pymgr.sources]]
name    = "aws"
url     = "https://corp-123456789.d.codeartifact.us-east-1.amazonaws.com/pypi/my-repo/simple/"
auth    = "aws-codeartifact"

[tool.pymgr.sources.aws.codeartifact]
domain  = "corp"
account = "123456789"
region  = "us-east-1"
repo    = "my-repo"
```

When the token is expired, pymgr calls `aws codeartifact get-authorization-token` automatically, caches the new token, and retries the request — transparent to the developer.

### Priority & Fallback

```
Package search order:
  1. pymgr.lock (exact version + SHA-256)
  2. Wheel cache (offline)
  3. Sources in order defined in pymgr.toml
  4. PyPI (if default = true on PyPI source)
```

Packages are resolved from the first source that provides them. Corporate packages shadow PyPI packages of the same name (intentional, for internal forks).

---

## 16. Plugin & Extension API

### Overview

Plugins are listed as a v1.x future feature. Without defining the interface now, v1 design decisions may accidentally close off extension points. This section sketches a plugin model that keeps options open.

### Plugin Categories

| Category | Purpose | Example |
|---|---|---|
| **Index provider** | Custom package source (not PyPI-compatible) | S3 bucket, Git repo |
| **Resolver hook** | Modify dependency graph before/after resolution | License compliance filter |
| **Installer hook** | Custom install steps | Post-install script runner |
| **Command extension** | Add new top-level commands | `pymgr deploy` |
| **Output formatter** | Custom output format | Slack notification |

### Plugin Discovery

pymgr discovers plugins via two mechanisms:

**1. Path-based (development)**

```toml
# pymgr.toml
[plugins]
paths = [
  "./plugins/my-custom-plugin",
]
```

**2. PyPI package (production)**

Plugins are Python packages with the entry point `pymgr.plugins`:

```toml
# Plugin's pyproject.toml
[project.entry-points."pymgr.plugins"]
my-plugin = "my_plugin:Plugin"
```

Install a plugin: `pymgr plugin add pymgr-my-plugin`

### Plugin Interface (Rust + FFI)

v1 uses a subprocess-based plugin model: plugins are executables that communicate via stdin/stdout JSON. This is language-agnostic and avoids ABI compatibility headaches.

```json
// pymgr → plugin (stdin)
{ "hook": "post_resolve", "packages": [...], "context": {...} }

// plugin → pymgr (stdout)
{ "packages": [...], "warnings": ["Package X has GPL license"] }
```

A future version may expose a native Rust trait-based API for performance-critical plugins.

### Plugin Security

- Plugins run with the same privileges as pymgr (user-level).
- Plugins are sandboxed from the internet in a future capability-based model.
- Plugins are reviewed before listing on the official plugin registry.

---

## 17. Garbage Collection

### Overview

pymgr's caches grow unbounded over time. A developer using pymgr for a year accumulates wheel caches across dozens of Python versions, metadata for thousands of packages, and stale environment snapshots. `pymgr cache gc` reclaims disk space safely.

### CLI Commands

```bash
# Show cache size breakdown
pymgr cache info

# Output:
  Wheel cache:       2.4 GB (1,203 wheels)
  Metadata cache:    45 MB  (8,921 entries)
  Python installs:   1.2 GB (3 versions: 3.10.14, 3.11.9, 3.12.3)
  Snapshots:         12 MB  (47 snapshots across 8 projects)
  ─────────────────────────────────
  Total:             3.7 GB

# Dry-run GC (show what would be deleted)
pymgr cache gc --dry-run

# Run GC
pymgr cache gc

# Aggressive GC (also removes wheels for unreferenced envs)
pymgr cache gc --aggressive

# Remove a specific Python version
pymgr python remove 3.10.14

# Clear all metadata (will be re-fetched on demand)
pymgr cache clear metadata
```

### GC Strategy

The GC uses a **mark-and-sweep** approach:

```
Mark phase:
  1. Scan all active environments (all projects on disk)
  2. Record every wheel referenced by those environments
  3. Record all Python versions referenced by any environment

Sweep phase:
  1. Delete wheels NOT in the referenced set AND older than TTL (default: 90 days)
  2. Delete metadata cache entries older than 24 hours (re-fetched on demand)
  3. Delete snapshots beyond the retention limit (default: 20 per project)
  4. Delete Python versions not referenced by any environment (requires --aggressive)
```

### GC Configuration

```toml
# ~/.pymgr/config.toml
[gc]
wheel-ttl-days     = 90    # Keep wheels used in last 90 days
metadata-ttl-secs  = 300   # Keep metadata for 5 minutes
snapshot-limit     = 20    # Per project
auto-gc            = true  # Run GC automatically when cache > threshold
auto-gc-threshold-gb = 5   # Trigger auto-GC when cache exceeds 5 GB
```

---

## 18. Migration Guides

### Migrating from pip + venv

**Step 1: Install pymgr**

```bash
curl -sSf https://pymgr.dev/install.sh | sh   # Unix
# or: winget install pymgr (Windows)
```

**Step 2: Import existing requirements**

```bash
cd my-project
pymgr import requirements.txt
```

**Step 3: If you have `requirements-dev.txt`**

```bash
pymgr import requirements-dev.txt --group dev
```

**Step 4: Sync the environment**

```bash
pymgr sync
```

**Step 5: Commit the new files**

```bash
git add pyproject.toml pymgr.lock
git rm requirements.txt requirements-dev.txt  # Optional
git commit -m "Migrate to pymgr"
```

**Before/after comparison:**

| Task | Before (pip+venv) | After (pymgr) |
|---|---|---|
| Create env | `python -m venv .venv && source .venv/bin/activate` | `pymgr init` |
| Install deps | `pip install -r requirements.txt` | `pymgr install` |
| Add package | `pip install numpy && pip freeze > requirements.txt` | `pymgr add numpy` |
| Activate env | `source .venv/bin/activate` | Auto (after `pymgr shell-init`) |

---

### Migrating from Poetry

**Step 1: Export from Poetry**

```bash
poetry export -f requirements.txt --output requirements.txt --with dev
```

**Step 2: Import into pymgr**

```bash
pymgr import requirements.txt
```

**Step 3: Copy dependency constraints from `pyproject.toml`**

Poetry and pymgr both use `pyproject.toml`. The `[tool.poetry.dependencies]` table maps directly to `[tool.pymgr.dependencies]`. pymgr provides a migration helper:

```bash
pymgr migrate --from poetry
```

This reads `[tool.poetry.dependencies]` and rewrites them to `[tool.pymgr.dependencies]`, converting Poetry-specific syntax (`^1.0` → `>=1.0,<2.0`) where necessary.

**Before/after comparison:**

| Task | Before (Poetry) | After (pymgr) |
|---|---|---|
| Add package | `poetry add numpy` | `pymgr add numpy` |
| Install from lockfile | `poetry install` | `pymgr install` |
| Run command | `poetry run python` | `pymgr run python` |
| Spawn shell | `poetry shell` | `pymgr shell` |

---

### Migrating from conda

Conda migration is more complex because conda environments can contain non-Python packages (R, CUDA, system libraries). pymgr handles only Python packages. Non-Python packages must remain in conda or be installed via system package manager.

**Step 1: Export Python-only packages from conda**

```bash
conda env export --from-history > environment.yml
```

**Step 2: Import into pymgr**

```bash
pymgr import environment.yml --from conda
```

pymgr reads the `pip:` section of `environment.yml` and the pure-Python conda packages, ignoring packages that are non-Python (e.g., `cudatoolkit`, `libopenblas`).

**Step 3: Review and resolve**

pymgr will flag any packages it cannot find on PyPI:

```
Warning: 'libopenblas=0.3.21' is not available on PyPI.
  If this is a system library, install it via your system package manager.
  If it is a Python binding, try: pymgr add openblas-python
```

---

### Migrating from pyenv + pip-tools

**Step 1: Pin the Python version**

```bash
# .python-version is already pyenv-compatible
# pymgr reads it automatically
cat .python-version
# 3.11.9
```

**Step 2: Import `requirements.in`**

```bash
pymgr import requirements.in
# Note: pymgr re-resolves from scratch, ignoring requirements.txt
```

**Step 3: Verify the resolved versions match expectations**

```bash
pymgr list
# Compare with: pip-compile --dry-run
```

---

## 19. Benchmarking Methodology

### Overview

The PRD lists aggressive performance targets (`< 100ms` warm env creation, etc.) but doesn't define how they're measured. Without a reproducible benchmark harness, targets are aspirational rather than contractual. This section defines the methodology so benchmarks are meaningful and CI-enforced.

### Benchmark Hardware Baseline

All published benchmarks are measured on the following standardized configuration:

| Component | Spec |
|---|---|
| CPU | AMD Ryzen 9 7950X (16 cores) |
| RAM | 64 GB DDR5-5600 |
| Storage | NVMe SSD (Samsung 990 Pro) |
| OS | Ubuntu 24.04 LTS |
| Kernel | Linux 6.8 |
| Python | CPython 3.11.9 |

macOS and Windows benchmarks use Apple M3 Pro and Intel Core i7-13700H respectively, published separately.

### Benchmark Suite

The benchmark suite lives in `benches/` and is run with `cargo bench` (using the `criterion` crate for statistical rigor).

```rust
// benches/env_creation.rs
fn bench_env_init_cold(c: &mut Criterion) {
    c.bench_function("env_init_cold", |b| {
        b.iter_with_setup(
            || clear_cache(),         // Setup: wipe cache
            |_| pymgr_init_env()      // Measured: create env
        )
    });
}

fn bench_env_init_warm(c: &mut Criterion) {
    c.bench_function("env_init_warm", |b| {
        b.iter_with_setup(
            || prime_cache(),         // Setup: warm cache
            |_| pymgr_init_env()      // Measured: create env
        )
    });
}
```

### Measured Operations

| Benchmark Name | Description | Target | Measurement |
|---|---|---|---|
| `env_init_cold` | `pymgr init`, no cache | < 500ms | wall clock, median of 50 runs |
| `env_init_warm` | `pymgr init`, full cache | < 50ms | wall clock, median of 100 runs |
| `add_single_cached` | `pymgr add numpy`, wheel cached | < 20ms | wall clock, median of 100 runs |
| `install_10_cached` | `pymgr install`, 10 packages, all cached | < 100ms | wall clock, median of 50 runs |
| `install_10_cold` | `pymgr install`, 10 packages, no cache | < 3s | wall clock, median of 20 runs |
| `run_python` | `pymgr run python --version` | < 10ms | wall clock, median of 100 runs |
| `resolve_complex` | Resolve 50-dep graph, metadata cached | < 200ms | wall clock, median of 50 runs |

### Statistical Rigor

- `criterion` reports mean, standard deviation, and confidence intervals.
- Benchmarks fail CI if any measurement exceeds **2x the target** (allowing for CI machine variance).
- Benchmarks are run on dedicated CI nodes (not shared runners) to reduce noise.
- Results are published to `https://pymgr.dev/benchmarks` after every release.

### Comparison Benchmarks

Published benchmarks include side-by-side comparisons with existing tools, measured identically:

| Tool | `init` (warm) | Install 10 pkgs (warm) |
|---|---|---|
| pymgr | < 50ms | < 100ms |
| uv | ~50ms | ~100ms |
| poetry | ~300ms | ~500ms |
| pip+venv | ~2s | ~3s |
| conda | ~5s | ~10s |

Comparison benchmarks are automated and run monthly to track regressions relative to competing tools.

### CI Enforcement

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench --bench env_creation -- --output-format bencher | tee bench-output.txt

- name: Check regression
  run: python scripts/check_bench_regression.py bench-output.txt
  # Fails if any benchmark exceeds 2x target
```

---

*This addendum extends pymgr PRD v1.0.0. All additions follow the same versioning, breaking-change, and deprecation policies defined in the base document.*