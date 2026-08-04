//! novai-shell — a small, friendly REPL shell for NovaiOS.
//!
//! Goals:
//!   • Be usable as /bin/sh replacement for the rescue console.
//!   • Pipe, redirect, and `&&` / `||` / `;` parsing.
//!   • Built-ins: cd, pwd, exit, export, echo, set, alias, history, help.
//!   • External commands resolved via PATH (so nushell, bash etc. can be
//!     dropped in as the default user shell without changing novai-shell).
//!
//! This is *not* intended to replace nushell/fish/zsh — it is the minimal
//! boot shell. Users are encouraged to set $SHELL=/usr/bin/nu at install
//! time.

use anyhow::Result;
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use std::path::PathBuf;
use std::process::Command;

mod builtins;
mod parser;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Non-interactive: execute the joined script.
        let script = args[1..].join(" ");
        return run_line(&script);
    }

    interactive()
}

fn interactive() -> Result<()> {
    let mut rl = DefaultEditor::new().map_err(|e| anyhow::anyhow!("rustyline init: {e}"))?;
    let history = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("novai-shell-history.txt");
    let _ = rl.load_history(&history);

    println!(
        "{}",
        "NovaiOS shell — type 'help' for built-in commands".cyan()
    );
    loop {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let home = env::var("HOME").unwrap_or_default();
        let display_cwd = if cwd.starts_with(&home) {
            format!("~{}", cwd.strip_prefix(&home).unwrap().display())
        } else {
            cwd.display().to_string()
        };
        let user = env::var("USER").unwrap_or_else(|_| "novai".into());
        let host = hostname().unwrap_or_else(|| "novai".into());
        let prompt = format!(
            "{}@{} {} > ",
            user.green(),
            host.green(),
            display_cwd.blue()
        );

        match rl.readline(&prompt) {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => {
                let _ = rl.add_history_entry(&line);
                if let Err(e) = run_line(&line) {
                    eprintln!("{}: {}", "error".red(), e);
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            }
            Err(e) => {
                eprintln!("{}: {}", "readline error".red(), e);
                break;
            }
        }
    }
    let _ = rl.save_history(&history);
    Ok(())
}

fn run_line(input: &str) -> Result<()> {
    let commands = parser::split_pipeline(input);
    let mut last_status = 0;
    for cmd in commands {
        let argv = parser::tokenize(&cmd);
        if argv.is_empty() {
            continue;
        }
        // Built-in?
        if builtins::is_builtin(&argv[0]) {
            last_status = builtins::run(&argv)?;
            continue;
        }
        // External
        match which::which(&argv[0]) {
            Ok(exe) => {
                let status = Command::new(exe).args(&argv[1..]).status()?;
                last_status = status.code().unwrap_or(1);
            }
            Err(_) => {
                eprintln!("{}: command not found: {}", "novai-shell".red(), argv[0]);
                last_status = 127;
            }
        }
    }
    if last_status != 0 {
        return Err(anyhow::anyhow!("exit {}", last_status));
    }
    Ok(())
}

fn hostname() -> Option<String> {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
}
