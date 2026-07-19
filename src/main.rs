//! pinpoint — your ~/.bash_history but for AI prompts.
//!
//! Save prompts you want to remember, search them, replay them.
//! Single 1.1 MB Rust binary + SQLite FTS5, local-first, zero cloud.
//!
//! Subcommands:
//!   log <text>       Save a prompt (text from arg or stdin)
//!   list             List saved prompts (newest first)
//!   find <query>     FTS5 search through saved prompts
//!   replay <name>    Print saved prompt to stdout (--copy to clipboard)
//!   rm <name>        Remove a saved prompt
//!   shell-init bash  Emit shell function for `pp <text>` one-liner save
//!
//! Data dir: $XDG_DATA_HOME/pinpoint (fallback ~/.local/share/pinpoint)
//! DB file:  <data-dir>/pinpoint.db

use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod storage;
mod shell;
mod output;

#[derive(Parser, Debug)]
#[command(name = "pinpoint", version, about = "Your ~/.bash_history but for AI prompts.")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Save a prompt. Reads from stdin if no <text> argument given.
    Log {
        /// Prompt text (omit to read from stdin)
        text: Vec<String>,
        /// Optional name (e.g. "sql-window-fn"); auto-generated if omitted
        #[arg(long, short = 'n')]
        name: Option<String>,
        /// Comma-separated tags, e.g. "sql,postgres,window"
        #[arg(long, short = 't', default_value = "")]
        tags: String,
        /// Source label, e.g. "claude-code", "codex", "chatgpt-web"
        #[arg(long, short = 's', default_value = "")]
        source: String,
    },

    /// List saved prompts (newest first).
    List {
        /// Filter by tag substring
        #[arg(long)]
        tag: Option<String>,
        /// Filter by source substring
        #[arg(long)]
        source: Option<String>,
        /// Limit
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },

    /// Full-text FTS5 search across saved prompts.
    Find {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },

    /// Print a saved prompt to stdout. `--copy` for clipboard.
    Replay {
        name: String,
        #[arg(long)]
        copy: bool,
    },

    /// Remove a saved prompt by name (or matching prefix).
    Rm {
        name: String,
    },

    /// Emit shell init code (sourced in ~/.bashrc / ~/.zshrc).
    ShellInit {
        /// Shell to emit for (bash | zsh | fish)
        #[arg(value_enum, default_value_t = Shell::Bash)]
        shell: Shell,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Shell { Bash, Zsh, Fish }

fn main() -> Result<()> {
    let cli = Cli::parse();
    let data_dir = data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    let db_path = data_dir.join("pinpoint.db");
    let mut store = storage::Store::open(&db_path)
        .with_context(|| format!("opening db {}", db_path.display()))?;

    match cli.cmd {
        Cmd::Log { text, name, tags, source } => {
            let prompt = if text.is_empty() {
                // read from stdin
                let mut buf = String::new();
                use std::io::Read;
                std::io::stdin().read_to_string(&mut buf)?;
                buf.trim_end().to_string()
            } else {
                text.join(" ")
            };
            if prompt.trim().is_empty() {
                anyhow::bail!("empty prompt — pass text as arg or pipe via stdin");
            }
            let p = store.log_prompt(&prompt, name.as_deref(), &tags, &source)?;
            eprintln!("[pinpoint] saved `{}` ({} chars, {} tokens≈)", p.name, p.content.len(), p.content.len() / 4);
        }
        Cmd::List { tag, source, limit } => {
            let items = store.list_prompts(tag.as_deref(), source.as_deref(), limit)?;
            output::print_list(&items);
        }
        Cmd::Find { query, limit } => {
            let items = store.find_prompts(&query, limit)?;
            output::print_find(&items, &query);
        }
        Cmd::Replay { name, copy } => {
            let p = store.get_prompt(&name)?
                .with_context(|| format!("prompt {:?} not found — run `pinpoint list`", name))?;
            if copy {
                shell::copy_to_clipboard(&p.content)?;
                eprintln!("[pinpoint] copied `{}` ({} chars) to clipboard", p.name, p.content.len());
            } else {
                print!("{}", p.content);
                if !p.content.ends_with('\n') { println!(); }
            }
        }
        Cmd::Rm { name } => {
            if store.rm_prompt(&name)? {
                eprintln!("[pinpoint] removed `{}`", name);
            } else {
                anyhow::bail!("prompt {:?} not found", name);
            }
        }
        Cmd::ShellInit { shell } => {
            shell::emit_init(shell);
        }
    }
    Ok(())
}

fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("PINPOINT_DATA_DIR") {
        return PathBuf::from(p);
    }
    if let Some(p) = std::env::var_os("XDG_DATA_HOME") {
        let mut pp = PathBuf::from(p);
        pp.push("pinpoint");
        return pp;
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut p = home;
    p.push(".local");
    p.push("share");
    p.push("pinpoint");
    p
}