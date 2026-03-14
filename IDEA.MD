# pymgr — Product Requirements Document & System Design

> **Version:** 1.0.0  
> **Status:** Draft  
> **Last Updated:** 2026-03-14  
> **Authors:** Engineering Team

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Goals & Non-Goals](#3-goals--non-goals)
4. [Target Users](#4-target-users)
5. [User Stories](#5-user-stories)
6. [CLI Design & UX](#6-cli-design--ux)
7. [System Architecture](#7-system-architecture)
8. [Core Modules — Detailed Design](#8-core-modules--detailed-design)
9. [Data Models & File Formats](#9-data-models--file-formats)
10. [Performance Architecture](#10-performance-architecture)
11. [Error Handling Strategy](#11-error-handling-strategy)
12. [Platform Support Matrix](#12-platform-support-matrix)
13. [Security Considerations](#13-security-considerations)
14. [Dependency Graph](#14-dependency-graph)
15. [Milestones & Roadmap](#15-milestones--roadmap)
16. [Success Metrics](#16-success-metrics)
17. [Appendix — Rust Crates Reference](#17-appendix--rust-crates-reference)

---

## 1. Executive Summary

**pymgr** is a blazing-fast, developer-first Python environment manager written in Rust. It replaces the fragmented ecosystem of `venv`, `virtualenv`, `pyenv`, and `pip` with a single unified CLI that can create isolated Python environments in **under 100ms**, manage Python version installations, resolve and lock dependencies, and integrate seamlessly into shell workflows.

The core design philosophy is: **zero configuration by default, full control when needed.**

---

## 2. Problem Statement

### Current Developer Pain Points

| Tool | Problem |
|------|---------|
| `venv` | Slow environment creation (~2–5s), no version management, no lockfile |
| `virtualenv` | Faster, but still Python-based overhead, complex internals |
| `pyenv` | Version management only, no env or dep management |
| `pip` | No dependency resolution, no lockfile, no parallelism |
| `poetry` | Heavy, slow, opinionated, non-standard |
| `conda` | Massive footprint, slow solver, not Python-native |

### Core Problems to Solve

1. **Fragmentation** — developers use 3–4 separate tools for a single workflow.
2. **Speed** — Python-based tooling adds interpreter startup overhead to every operation.
3. **Reproducibility** — no single lockfile standard across the ecosystem.
4. **UX** — inconsistent interfaces, confusing activation semantics, poor error messages.

---

## 3. Goals & Non-Goals

### Goals

- Create Python virtual environments in **< 100ms** on warm cache, **< 500ms** cold.
- Provide a **single binary** with zero runtime dependencies.
- Support **Python version installation** and switching per project.
- Generate a **`pymgr.lock`** for fully reproducible environments.
- First-class support for **Linux, macOS, and Windows**.
- Shell integration for **automatic environment activation** on `cd`.
- Parallel package installation using async Rust.
- Human-readable, actionable error messages.

### Non-Goals

- Replacing the Python Package Index (PyPI) — pymgr pulls from it, not replace it.
- Supporting non-Python runtimes (Node, Ruby, etc.) in v1.
- Providing a GUI or web dashboard in v1.
- Building a custom package format — wheels and sdists are used as-is.

---

## 4. Target Users

### Primary — Solo Developer / Indie Hacker
Fast setup, minimal config, just works. Uses `pymgr init` and never thinks about it again.

### Secondary — Professional/Team Engineer
Needs lockfiles, CI integration, reproducible builds, and Python version pinning per repo.

### Tertiary — DevOps / Platform Engineer
Needs scriptable CLI, exit codes, JSON output mode, and Docker integration.

---

## 5. User Stories

### Epic 1: Environment Lifecycle

```
As a developer,
I want to create an isolated Python environment in one command,
So that my project dependencies don't pollute the system Python.

Acceptance Criteria:
- `pymgr init` creates a .pymgr/ directory with a ready-to-use environment.
- The env is usable within 500ms on a cold cache.
- Running `pymgr run python` uses the isolated interpreter.
```

```
As a developer,
I want environments to activate automatically when I cd into a project,
So that I never have to remember to source activate scripts.

Acceptance Criteria:
- After running `pymgr shell-init`, shell hooks auto-activate the env.
- Deactivation is automatic on cd out of the project.
```

### Epic 2: Python Version Management

```
As a developer,
I want to install and pin specific Python versions per project,
So that my team runs the exact same interpreter everywhere.

Acceptance Criteria:
- `pymgr python install 3.12` downloads and installs Python 3.12.
- `pymgr python use 3.12` pins this version in .python-version.
- The env uses this version automatically.
```

### Epic 3: Dependency Management

```
As a developer,
I want to add packages and have them recorded in a lockfile,
So that my teammates get identical environments.

Acceptance Criteria:
- `pymgr add numpy` installs numpy and updates pymgr.lock.
- `pymgr install` on a fresh clone restores the exact same versions.
- Parallel downloads make bulk installs fast.
```

### Epic 4: CI/CD Integration

```
As a DevOps engineer,
I want pymgr to be scriptable and provide machine-readable output,
So that I can integrate it into pipelines.

Acceptance Criteria:
- All commands support --json flag for structured output.
- Non-zero exit codes on failure with meaningful codes.
- `pymgr install --frozen` fails if lockfile is stale.
```

---

## 6. CLI Design & UX

### Command Surface

```
pymgr <COMMAND> [OPTIONS]

COMMANDS:
  init                 Initialize a new environment in the current directory
  create <name>        Create a named environment
  activate             Print activation script (eval $(pymgr activate))
  deactivate           Deactivate current environment
  run <cmd> [args...]  Run a command inside the environment
  shell                Spawn a subshell with the environment active

  python               Manage Python installations
    python list        List installed Python versions
    python install     Install a Python version
    python use         Pin a Python version for this project
    python remove      Remove an installed version

  add <pkg...>         Add packages and update lockfile
  remove <pkg...>      Remove packages
  install              Install from lockfile (or pyproject.toml)
  update [pkg...]      Update packages to latest compatible versions
  sync                 Sync environment exactly to lockfile
  list                 List installed packages

  env                  Manage environments
    env list           List all environments
    env remove         Remove an environment
    env info           Show environment details

  shell-init           Print shell integration script
  self-update          Update pymgr itself
  doctor               Diagnose environment problems

GLOBAL OPTIONS:
  -v, --verbose        Enable verbose output
  --json               Machine-readable JSON output
  --no-color           Disable color output
  --env <path>         Override environment path
  -h, --help           Show help
  -V, --version        Show version
```

### UX Principles

**1. Progressive disclosure** — simple commands for simple tasks, flags for power users.

**2. Opinionated defaults** — `pymgr init` works without any arguments. Config only when needed.

**3. Fast feedback** — progress spinners, clear output, timing shown with `--verbose`.

**4. Actionable errors** — every error suggests a fix:

```
Error: Python 3.11 not found.
  Hint: run `pymgr python install 3.11` to install it.
```

**5. No surprise mutations** — read-only commands never modify state. Mutations are explicit.

---

## 7. System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                          pymgr CLI                              │
│                       (clap-powered)                            │
└──────────┬──────────────────────────────────────┬──────────────┘
           │                                      │
     ┌─────▼──────┐                       ┌───────▼────────┐
     │  Resolver  │                       │  Env Manager   │
     │  Engine    │                       │                │
     └─────┬──────┘                       └───────┬────────┘
           │                                      │
     ┌─────▼──────┐    ┌────────────┐    ┌───────▼────────┐
     │  Package   │    │  Python    │    │  Activation    │
     │  Installer │    │  Locator   │    │  Generator     │
     └─────┬──────┘    └─────┬──────┘    └────────────────┘
           │                 │
     ┌─────▼──────┐    ┌─────▼──────┐
     │   Wheel    │    │  Python    │
     │   Cache    │    │  Installer │
     └─────┬──────┘    └─────┬──────┘
           │                 │
     ┌─────▼─────────────────▼──────┐
     │        Storage Layer         │
     │  ~/.pymgr/{cache,envs,py}    │
     └──────────────────────────────┘
```

### Module Breakdown

| Module | Responsibility | Key Crates |
|--------|---------------|------------|
| `cli` | Argument parsing, command dispatch | `clap` |
| `env` | Create, delete, list environments | `std::fs`, `symlink` |
| `python` | Locate, install, manage Python versions | `which`, `reqwest` |
| `resolver` | SAT-based dependency resolution | `pubgrub` |
| `installer` | Download, verify, extract wheels | `tokio`, `reqwest`, `zip` |
| `cache` | Wheel cache, env template cache | `std::fs`, `sha2` |
| `activation` | Generate shell activation scripts | `std::fs`, `tera` |
| `lockfile` | Read/write `pymgr.lock` | `serde`, `toml` |
| `config` | Read `pyproject.toml`, `.pymgr.toml` | `serde`, `toml` |
| `shell` | Shell hook integration | string templating |
| `doctor` | Diagnostics and health checks | custom |

---

## 8. Core Modules — Detailed Design

### 8.1 Environment Manager (`env.rs`)

**Responsibilities:**
- Create the virtual environment directory tree.
- Symlink or copy the Python interpreter.
- Write `pyvenv.cfg`.
- Bootstrap pip via `ensurepip`.

**Environment directory structure:**

```
.pymgr/
└── env/
    ├── bin/                    (Scripts/ on Windows)
    │   ├── python -> /usr/bin/python3.11   (symlink)
    │   ├── python3 -> python
    │   ├── pip -> pip3
    │   ├── activate            (bash/zsh)
    │   ├── activate.fish
    │   └── activate.ps1        (PowerShell)
    ├── lib/
    │   └── python3.11/
    │       └── site-packages/
    │           └── .gitkeep
    ├── include/
    └── pyvenv.cfg
```

**Creation algorithm:**

```
1. Resolve Python interpreter path
2. Read Python version via subprocess
3. Create directory skeleton (create_dir_all)
4. On Unix: symlink interpreter → bin/python
   On Windows: copy interpreter + DLLs or junction
5. Write pyvenv.cfg
6. Run: env/bin/python -m ensurepip --upgrade
7. Write activation scripts
8. Write .pymgr/env.json metadata
```

**`pyvenv.cfg` format:**

```ini
home = /usr/bin
include-system-site-packages = false
version = 3.11.9
pymgr = true
pymgr-version = 0.1.0
```

---

### 8.2 Python Locator & Installer (`python.rs`)

**Locator search order:**

```
1. .python-version file in project root (pyenv-compatible)
2. pymgr.toml [python] version field
3. PYMGR_PYTHON environment variable
4. PATH scan (python3.X, python3, python)
5. pyenv shims (~/.pyenv/shims)
6. Common install paths:
     Linux:   /usr/bin, /usr/local/bin, /opt/homebrew/bin
     macOS:   /usr/local/bin, /opt/homebrew/bin, /Library/Frameworks
     Windows: %LOCALAPPDATA%\Programs\Python, C:\Python3X
```

**Python Installer:**

Downloads from [python.org/ftp](https://www.python.org/ftp/python/) or a configurable mirror. Verifies SHA-256 checksums against official signatures.

Install locations:

```
~/.pymgr/python/
├── 3.11.9/
│   ├── bin/python3.11
│   ├── lib/
│   └── include/
├── 3.12.3/
└── ...
```

Supports:
- Standalone Python builds from `python-build-standalone` project for faster installs.
- Official CPython releases as fallback.

---

### 8.3 Dependency Resolver (`resolver.rs`)

Uses the **PubGrub** algorithm (same as Cargo, Poetry 2.x) for correct version resolution.

**Resolution flow:**

```
Parse requirements (pyproject.toml / CLI)
       │
       ▼
Fetch package metadata from PyPI JSON API
(parallel, cached to ~/.pymgr/cache/metadata/)
       │
       ▼
Run PubGrub solver
       │
  ┌────┴────┐
  │ Success │ → Write pymgr.lock
  └────┬────┘
  │ Conflict │ → Human-readable conflict explanation
  └──────────┘
```

**Lock file (`pymgr.lock`) format:**

```toml
[metadata]
python = "3.11.9"
resolver = "pubgrub-v1"
generated-at = "2026-03-14T10:00:00Z"

[[package]]
name = "numpy"
version = "1.26.4"
source = "pypi"
sha256 = "abc123..."
requires-python = ">=3.9"
dependencies = []

[[package]]
name = "pandas"
version = "2.2.1"
source = "pypi"
sha256 = "def456..."
dependencies = [
  { name = "numpy", version = ">=1.23" },
  { name = "python-dateutil", version = ">=2.8" },
]
```

---

### 8.4 Package Installer (`installer.rs`)

**Download pipeline (async, parallel):**

```
Lockfile packages
      │
      ▼ (for each package, concurrently)
 Check wheel cache (~/.pymgr/wheels/)
      │
  ┌───┴───┐
  │ HIT   │──→ Hardlink wheel to site-packages
  └───────┘
  │ MISS  │
  └───┬───┘
      ▼
 Fetch wheel from PyPI (tokio async)
      │
      ▼
 Verify SHA-256
      │
      ▼
 Extract to site-packages
      │
      ▼
 Write to wheel cache (hardlink)
```

**Concurrency model:**

```rust
// Controlled parallelism — avoid thundering herd on PyPI
let semaphore = Arc::new(Semaphore::new(8)); // max 8 concurrent downloads

let tasks: Vec<_> = packages
    .iter()
    .map(|pkg| {
        let sem = Arc::clone(&semaphore);
        tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            download_and_install(pkg).await
        })
    })
    .collect();

join_all(tasks).await;
```

---

### 8.5 Activation Generator (`activation.rs`)

Generates shell-specific activation scripts at environment creation time.

**Scripts generated:**

| File | Shell |
|------|-------|
| `bin/activate` | bash, zsh, dash |
| `bin/activate.fish` | fish |
| `bin/activate.ps1` | PowerShell |
| `bin/activate.bat` | Windows CMD |

**`activate` (bash/zsh) template:**

```bash
# pymgr activate — generated by pymgr {{version}}
export VIRTUAL_ENV="{{env_path}}"
export PYMGR_ACTIVE="1"
export PYMGR_PROJECT="{{project_name}}"

_pymgr_old_path="$PATH"
export PATH="$VIRTUAL_ENV/bin:$PATH"

_pymgr_old_ps1="${PS1-}"
PS1="({{project_name}}) ${PS1-}"
export PS1

deactivate() {
    export PATH="$_pymgr_old_path"
    export PS1="$_pymgr_old_ps1"
    unset VIRTUAL_ENV PYMGR_ACTIVE PYMGR_PROJECT
    unset -f deactivate
}
```

**Shell hook for auto-activation (`shell-init`):**

```bash
# Add to ~/.bashrc or ~/.zshrc via: eval "$(pymgr shell-init bash)"
__pymgr_chpwd() {
    if [ -f ".pymgr/env.json" ]; then
        if [ "$PYMGR_PROJECT" != "$(pwd)" ]; then
            source .pymgr/env/bin/activate
        fi
    elif [ -n "$PYMGR_ACTIVE" ]; then
        deactivate
    fi
}
# Hook into cd
cd() { builtin cd "$@" && __pymgr_chpwd; }
```

---

### 8.6 Cache Architecture (`cache.rs`)

Three-layer cache for maximum speed.

```
~/.pymgr/
├── cache/
│   ├── metadata/              PyPI package metadata (JSON), TTL: 5 min
│   │   └── numpy/
│   │       └── 1.26.4.json
│   ├── wheels/                Verified wheels, keyed by sha256
│   │   └── ab/cd/abcd1234.../  (first 4 chars as directory sharding)
│   └── python/                Python build artifacts
├── envs/                      Named environments (not project-local)
│   └── my-project/
└── python/                    Installed Python versions
    ├── 3.11.9/
    └── 3.12.3/
```

**Hardlinking strategy:**

Instead of copying wheels on every install, pymgr uses **hardlinks** from the cache to the environment's `site-packages`. This is `O(1)` and consumes zero additional disk space.

```rust
// Attempt hardlink first (same filesystem required)
match std::fs::hard_link(&cache_path, &dest_path) {
    Ok(_) => {},
    // Fallback to copy if cross-filesystem
    Err(_) => std::fs::copy(&cache_path, &dest_path)?,
}
```

**Metadata caching:**

PyPI metadata is cached with a 5-minute TTL. Resolver reads from cache first, PyPI second.

---

## 9. Data Models & File Formats

### `pyproject.toml` (user-facing config)

```toml
[tool.pymgr]
python = "3.11"           # Minimum Python version
env-path = ".pymgr/env"  # Override default env path

[tool.pymgr.dependencies]
numpy = ">=1.24"
pandas = "^2.0"
requests = "*"

[tool.pymgr.dev-dependencies]
pytest = ">=7.0"
black = "*"
```

### `.pymgr/env.json` (internal metadata)

```json
{
  "version": "0.1.0",
  "python_version": "3.11.9",
  "python_path": "/usr/bin/python3.11",
  "env_path": ".pymgr/env",
  "created_at": "2026-03-14T10:00:00Z",
  "platform": "linux-x86_64"
}
```

### `.python-version` (pyenv-compatible)

```
3.11.9
```

---

## 10. Performance Architecture

### Target Benchmarks

| Operation | Cold (no cache) | Warm (full cache) |
|-----------|----------------|-------------------|
| `pymgr init` | < 500ms | < 50ms |
| `pymgr add numpy` (already cached) | < 200ms | < 20ms |
| `pymgr install` (10 packages) | < 3s | < 100ms |
| `pymgr run python` | < 10ms | < 10ms |
| Python version install | < 30s | < 5s |

### Speed Techniques

**1. Symlinks over copies**
The Python interpreter is symlinked, not copied. Saves ~10–50ms.

**2. Hardlinks for wheels**
Zero-copy package installation from cache. Saves ~50–500ms per package.

**3. Parallel async downloads**
`tokio` with controlled concurrency (8 workers). 10 packages download simultaneously.

**4. Metadata caching**
PyPI JSON API responses cached locally. Resolver runs entirely offline on cache hits.

**5. Environment template cache**
A pre-built env skeleton is cached. New envs are created by cloning the template with hardlinks.

```
~/.pymgr/cache/env-template/
└── 3.11.9/
    ├── bin/python  (hardlink target)
    └── lib/...
```

**6. Lazy pip bootstrap**
`ensurepip` only runs if pip is actually needed. `pymgr run` works without pip installed.

**7. SIMD-accelerated hashing**
Uses `sha2` crate with hardware acceleration for fast checksum verification.

---

## 11. Error Handling Strategy

### Error Philosophy

Every error must be:
- **Specific** — what exactly went wrong.
- **Actionable** — what the user should do.
- **Non-noisy** — no stack traces unless `--verbose`.

### Error Taxonomy

| Code | Category | Example |
|------|----------|---------|
| E001 | Python not found | No Python 3.x found in PATH |
| E002 | Version mismatch | Project requires 3.11, found 3.9 |
| E100 | Env already exists | .pymgr/env already exists |
| E101 | Env not found | No environment at .pymgr/env |
| E200 | Package not found | Package 'numpyy' not found on PyPI |
| E201 | Resolution conflict | numpy requires Python >=3.9, environment has 3.8 |
| E202 | Checksum mismatch | Wheel SHA-256 doesn't match PyPI metadata |
| E300 | Network error | Failed to connect to pypi.org |
| E301 | Auth required | Private index requires credentials |
| E400 | Lock stale | pymgr.lock is outdated (--frozen mode) |

### Error output format

**Human mode (default):**

```
Error [E201]: Dependency conflict detected.
  numpy 2.0 requires Python >=3.9
  Your environment uses Python 3.8.10

  To fix, either:
    • Run `pymgr python use 3.11` to upgrade
    • Or pin to numpy <2.0 in pyproject.toml
```

**JSON mode (`--json`):**

```json
{
  "error": {
    "code": "E201",
    "category": "resolution_conflict",
    "message": "Dependency conflict: numpy 2.0 requires Python >=3.9",
    "context": {
      "package": "numpy",
      "version": "2.0",
      "requires_python": ">=3.9",
      "environment_python": "3.8.10"
    },
    "suggestions": [
      "pymgr python use 3.11",
      "pin numpy <2.0 in pyproject.toml"
    ]
  }
}
```

---

## 12. Platform Support Matrix

| Feature | Linux (x86_64) | Linux (aarch64) | macOS (Intel) | macOS (Apple Silicon) | Windows (x64) |
|---------|:--------------:|:---------------:|:-------------:|:--------------------:|:-------------:|
| Env creation | ✅ | ✅ | ✅ | ✅ | ✅ |
| Python symlinks | ✅ | ✅ | ✅ | ✅ | ⚠️ Junction |
| bash/zsh activation | ✅ | ✅ | ✅ | ✅ | ❌ |
| fish activation | ✅ | ✅ | ✅ | ✅ | ❌ |
| PowerShell activation | ✅ | ✅ | ✅ | ✅ | ✅ |
| CMD activation | ❌ | ❌ | ❌ | ❌ | ✅ |
| Shell auto-activation | ✅ | ✅ | ✅ | ✅ | ⚠️ PS only |
| Python version install | ✅ | ✅ | ✅ | ✅ | ✅ |
| Parallel installs | ✅ | ✅ | ✅ | ✅ | ✅ |
| Wheel cache | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## 13. Security Considerations

### Package Integrity

- All wheels verified against PyPI SHA-256 before installation.
- Metadata verified via PyPI's JSON API (HTTPS only).
- No execution of arbitrary code during install (no `setup.py` runs where avoidable; prefer wheel-only installs).

### Python Binary Verification

When downloading Python builds:
- Verify PGP signature or SHA-256 from official source.
- Never execute unsigned or unverified binaries.

### Credential Management

- PyPI tokens read from `~/.pymgr/credentials.toml` or environment variables.
- Never stored in project files or lockfiles.
- Credentials encrypted at rest using OS keychain (v2 feature).

### Supply Chain

- `pymgr` itself is distributed as a signed binary.
- Reproducible builds via pinned `Cargo.lock`.
- SLSA Level 2 provenance for releases.

---

## 14. Dependency Graph

### `Cargo.toml` (key dependencies)

```toml
[dependencies]
# CLI
clap = { version = "4", features = ["derive", "color"] }

# Error handling
anyhow = "1"
thiserror = "1"

# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"], default-features = false }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Filesystem
dirs = "5"
walkdir = "2"
which = "6"
tempfile = "3"

# Hashing & crypto
sha2 = "0.10"
hex = "0.4"

# Archive extraction
zip = "0.6"
tar = "0.4"
flate2 = "1"

# Version resolution (PubGrub)
pubgrub = "0.2"

# Progress display
indicatif = "0.17"

# Terminal colors
console = "0.15"

# Templating (activation scripts)
minijinja = "1"

# Parallelism
rayon = "1"
futures = "0.3"

[dev-dependencies]
assert_cmd = "2"
tempdir = "0.3"
mockito = "1"
```

---

## 15. Milestones & Roadmap

### v0.1 — Foundation (Month 1–2)
Core environment creation, Python detection, basic activation scripts.

- [ ] `pymgr init` — create env, symlink Python, write pyvenv.cfg
- [ ] `pymgr activate` — write and source activation scripts (bash, zsh, PowerShell)
- [ ] `pymgr run <cmd>` — run command in env
- [ ] `pymgr doctor` — environment health check
- [ ] Linux + macOS support

### v0.2 — Package Management (Month 2–3)
Install packages from PyPI with wheel cache.

- [ ] `pymgr add <pkg>` — resolve + install + update lockfile
- [ ] `pymgr install` — install from lockfile
- [ ] `pymgr list` — list installed packages
- [ ] `pymgr remove <pkg>` — uninstall + update lockfile
- [ ] Parallel downloads (tokio)
- [ ] Wheel cache with hardlinks

### v0.3 — Python Version Management (Month 3–4)
Install and pin Python versions per project.

- [ ] `pymgr python install <version>` — download + verify + install
- [ ] `pymgr python list` — list installed versions
- [ ] `pymgr python use <version>` — pin to `.python-version`
- [ ] pyenv-compatible `.python-version` support

### v0.4 — Shell Integration & UX (Month 4–5)
Auto-activation, improved error messages, Windows support.

- [ ] `pymgr shell-init` — auto-activation hooks for bash/zsh/fish/PowerShell
- [ ] Windows support (junctions, `.bat` scripts)
- [ ] Improved error messages with E-code taxonomy
- [ ] `--json` output flag
- [ ] Progress indicators with `indicatif`

### v0.5 — Dependency Resolution (Month 5–6)
Full PubGrub resolver, update command, extras support.

- [ ] PubGrub-based resolver
- [ ] `pymgr update` — update to latest compatible versions
- [ ] `pymgr sync` — sync env exactly to lockfile
- [ ] Extras support (`pymgr add requests[security]`)
- [ ] `--frozen` flag for CI

### v1.0 — Production Release (Month 7–8)
Stability, documentation, ecosystem integrations.

- [ ] Stable CLI API (semver committed)
- [ ] Comprehensive documentation site
- [ ] GitHub Actions integration
- [ ] Docker base image
- [ ] Private index support (Nexus, Artifactory, DevPI)
- [ ] Performance benchmarks published

### v1.x — Future

- [ ] Plugin system
- [ ] `pymgr publish` — build and publish packages
- [ ] Workspace support (monorepos)
- [ ] OS keychain credential storage
- [ ] GUI (Tauri-based, separate repo)

---

## 16. Success Metrics

### Performance KPIs

| Metric | Target | How Measured |
|--------|--------|-------------|
| Cold env creation | < 500ms | Benchmark suite |
| Warm env creation | < 50ms | Benchmark suite |
| 10-package install (cached) | < 100ms | Benchmark suite |
| Binary size | < 10MB | CI build artifact |

### Adoption KPIs (12-month targets)

| Metric | Target |
|--------|--------|
| GitHub stars | 5,000 |
| Monthly downloads (crates.io) | 10,000 |
| Open issues response time (P50) | < 48h |
| Crash rate (telemetry, opt-in) | < 0.1% |

### Quality KPIs

| Metric | Target |
|--------|--------|
| Test coverage (unit + integration) | > 80% |
| CI green rate | > 99% |
| Semver compatibility | 100% within major |

---

## 17. Appendix — Rust Crates Reference

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4.x | CLI argument parsing with derive macros |
| `anyhow` | 1.x | Flexible error handling |
| `thiserror` | 1.x | Typed errors with display |
| `tokio` | 1.x | Async runtime for parallel I/O |
| `reqwest` | 0.12 | Async HTTP client (PyPI, Python downloads) |
| `serde` | 1.x | Serialization framework |
| `serde_json` | 1.x | JSON (PyPI API, metadata cache) |
| `toml` | 0.8 | TOML (pyproject.toml, pymgr.lock) |
| `dirs` | 5.x | OS-specific directory paths |
| `walkdir` | 2.x | Recursive directory traversal |
| `which` | 6.x | Locate executables in PATH |
| `sha2` | 0.10 | SHA-256 wheel verification |
| `zip` | 0.6 | Wheel extraction (.whl = .zip) |
| `tar` + `flate2` | 1.x | Tarball extraction (sdists, Python builds) |
| `pubgrub` | 0.2 | PubGrub dependency solver |
| `indicatif` | 0.17 | Progress bars and spinners |
| `console` | 0.15 | Terminal colors and formatting |
| `minijinja` | 1.x | Activation script templating |
| `rayon` | 1.x | CPU-parallel operations |
| `tempfile` | 3.x | Safe temporary files/dirs |

---

*This document is a living specification. Updates are made as requirements evolve. All breaking changes to the CLI surface require a major version bump and deprecation notice.*