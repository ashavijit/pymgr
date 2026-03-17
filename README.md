# pymgr

A blazing-fast Python environment manager written in Rust.

`pymgr` is a unified tool for managing Python installations, virtual environments, and package dependencies. It aims to replace PIP, Virtualenv, Pyenv, and pip-tools with a single cohesive interface, focusing on speed through parallel downloads, strict lockfile reproducibility, and an integrated cache layer.

## Installation

Requires Rust `1.75+`.

```bash
cargo install --path .
```

## Global Options

The following options can be passed before any subcommand:

## Commands

| Command | Arguments | Description |
|---|---|---|
| `init` | `[PYTHON_VERSION]` | Initialize a new environment in the current directory |
| `create` | `<NAME> [PYTHON_VERSION]` | Create a named environment |
| `activate` | | Print activation script |
| `deactivate` | | Deactivate current environment |
| `run` | `<CMD> [ARGS]...` | Run a command inside the environment |
| `shell` | | Spawn a subshell with the environment active |
| `python` | `[SUBCOMMAND]` | Manage Python installations (`list`, `install`, `use`, `remove`) |
| `add` | `<PACKAGES>... [--dev] [--editable]`| Add packages and update lockfile |
| `remove` | `<PACKAGES>...` | Remove packages |
| `install` | `[--frozen]` | Install from lockfile or pyproject.toml |
| `update` | `[PACKAGES]...` | Update packages to latest compatible versions |
| `sync` | | Sync environment exactly to lockfile |
| `list` | | List installed packages |
| `env` | `[SUBCOMMAND]` | Manage environments (`list`, `info`, `remove`) |
| `shell-init` | `<SHELL>` | Print shell integration script |
| `self-update`| | Update pymgr itself |
| `doctor` | | Diagnose environment problems |
| `workspace` | `[SUBCOMMAND]` | Manage workspaces (`init`, `add`, `list`, `run`) |
| `export` | `<FORMAT> [--hashes]` | Export environment (`requirements`, `conda`) |
| `import` | `<FILE>` | Import environment requirement files |
| `snapshot` | `[SUBCOMMAND]` | Manage rollbacks and snapshots (`create`, `list`, `restore`) |
| `audit` | `[--json]` | Audit dependencies for vulnerabilities via OSV |
| `cache` | `[SUBCOMMAND]` | Manage caches (`info`, `clear`, `warm`, `gc`) |
| `ide` | `<NAME>` | Configure IDE integrations (`vscode`, `pycharm`, `pyright`) |

## Project Configuration

`pymgr` relies on `pyproject.toml` (specifically under the `[tool.pymgr]` block). If missing, `pymgr init` will scaffold this configuration. The lockfile (`pymgr.lock`) dictates reproducible builds and tracks exact version hashes.

