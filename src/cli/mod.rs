use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "pymgr",
    about = "A blazing-fast Python environment manager",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(long, global = true, help = "Machine-readable JSON output")]
    pub json: bool,

    #[arg(long, global = true, help = "Disable color output")]
    pub no_color: bool,

    #[arg(long, global = true, help = "Override environment path")]
    pub env_path: Option<PathBuf>,

    #[arg(long, global = true, help = "Run in offline mode")]
    pub offline: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Initialize a new environment in the current directory")]
    Init {
        #[arg(long, help = "Python version to use")]
        python: Option<String>,
    },

    #[command(about = "Create a named environment")]
    Create {
        #[arg(help = "Name of the environment")]
        name: String,
        #[arg(long, help = "Python version to use")]
        python: Option<String>,
    },

    #[command(about = "Print activation script")]
    Activate,

    #[command(about = "Deactivate current environment")]
    Deactivate,

    #[command(about = "Run a command inside the environment")]
    Run {
        #[arg(help = "Command to run")]
        cmd: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, help = "Arguments for the command")]
        args: Vec<String>,
    },

    #[command(about = "Spawn a subshell with the environment active")]
    Shell,

    #[command(subcommand, about = "Manage Python installations")]
    Python(PythonCommands),

    #[command(about = "Add packages and update lockfile")]
    Add {
        #[arg(required = true, num_args = 1..)]
        packages: Vec<String>,
        #[arg(long, help = "Add as dev dependency")]
        dev: bool,
        #[arg(short, long, help = "Install in editable mode")]
        editable: bool,
    },

    #[command(about = "Remove packages")]
    Remove {
        #[arg(required = true, num_args = 1..)]
        packages: Vec<String>,
    },

    #[command(about = "Install from lockfile or pyproject.toml")]
    Install {
        #[arg(long, help = "Fail if lockfile is stale")]
        frozen: bool,
    },

    #[command(about = "Update packages to latest compatible versions")]
    Update {
        #[arg(num_args = 0..)]
        packages: Vec<String>,
    },

    #[command(about = "Sync environment exactly to lockfile")]
    Sync,

    #[command(about = "List installed packages")]
    List,

    #[command(subcommand, about = "Manage environments")]
    Env(EnvCommands),

    #[command(about = "Print shell integration script")]
    ShellInit {
        #[arg(help = "Shell type (bash, zsh, fish, powershell)")]
        shell: String,
    },

    #[command(about = "Update pymgr itself")]
    SelfUpdate,

    #[command(about = "Diagnose environment problems")]
    Doctor,

    #[command(subcommand, about = "Manage workspaces")]
    Workspace(WorkspaceCommands),

    #[command(about = "Export environment")]
    Export {
        #[arg(help = "Format (requirements, conda, poetry)")]
        format: Option<String>,
        #[arg(long, help = "Include hashes")]
        hashes: bool,
    },

    #[command(about = "Import environment")]
    Import {
        #[arg(help = "File to import")]
        file: String,
    },

    #[command(subcommand, about = "Manage rollbacks and snapshots")]
    Snapshot(SnapshotCommands),

    #[command(about = "Audit dependencies for vulnerabilities")]
    Audit {
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },

    #[command(subcommand, about = "Manage caches")]
    Cache(CacheCommands),

    #[command(about = "Configure IDE integrations")]
    Ide {
        #[arg(help = "IDE name (vscode, pycharm, pyright)")]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum PythonCommands {
    #[command(about = "List installed Python versions")]
    List,

    #[command(about = "Install a Python version")]
    Install {
        #[arg(help = "Python version to install (e.g. 3.12)")]
        version: String,
    },

    #[command(name = "use", about = "Pin a Python version for this project")]
    Use {
        #[arg(help = "Python version to pin")]
        version: String,
    },

    #[command(about = "Remove an installed Python version")]
    Remove {
        #[arg(help = "Python version to remove")]
        version: String,
    },
}

#[derive(Subcommand)]
pub enum EnvCommands {
    #[command(about = "List all environments")]
    List,

    #[command(about = "Remove an environment")]
    Remove {
        #[arg(help = "Environment name or path")]
        name: String,
    },

    #[command(about = "Show environment details")]
    Info,
}

#[derive(Subcommand)]
pub enum WorkspaceCommands {
    #[command(about = "Initialize workspace")]
    Init,
    #[command(about = "List members")]
    List,
}

#[derive(Subcommand)]
pub enum SnapshotCommands {
    #[command(about = "List snapshots")]
    List,
    #[command(about = "Rollback to snapshot")]
    Rollback {
        id: Option<String>,
    },
    #[command(about = "Show diff")]
    Diff {
        id: String,
    },
    #[command(about = "Garbage collect snapshots")]
    Gc,
}

#[derive(Subcommand)]
pub enum CacheCommands {
    #[command(about = "Clear cache")]
    Clear {
        #[arg(help = "Cache type")]
        target: Option<String>,
    },
    #[command(about = "Garbage collect cache")]
    Gc {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        aggressive: bool,
    },
    #[command(about = "Show cache info")]
    Info,
    #[command(about = "Warm cache")]
    Warm {
        packages: Vec<String>,
    },
}
