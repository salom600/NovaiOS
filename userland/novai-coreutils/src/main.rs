//! novai-coreutils — a tiny facade that prefers uutils-coreutils when
//! installed, and otherwise implements the most essential commands in pure
//! Rust so the live ISO is usable without a full coreutils.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "novai-coreutils", version, about = "NovaiOS coreutils facade")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Concatenate files to stdout.
    Cat { files: Vec<PathBuf> },
    /// List directory contents.
    Ls { path: Option<PathBuf> },
    /// Remove files.
    Rm { files: Vec<PathBuf> },
    /// Make directories.
    Mkdir { dirs: Vec<PathBuf> },
    /// Print working directory.
    Pwd,
    /// Print last N lines of a file.
    Tail {
        file: PathBuf,
        #[arg(short = 'n', default_value_t = 10)]
        n: usize,
    },
    /// Print first N lines of a file.
    Head {
        file: PathBuf,
        #[arg(short = 'n', default_value_t = 10)]
        n: usize,
    },
    /// Copy files.
    Cp { from: PathBuf, to: PathBuf },
    /// Move / rename files.
    Mv { from: PathBuf, to: PathBuf },
    /// Print the system uptime.
    Uptime,
    /// Show free memory.
    Free,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Cat { files } => cat(&files),
        Cmd::Ls { path } => ls(path.as_deref().unwrap_or(std::path::Path::new("."))),
        Cmd::Rm { files } => {
            for f in &files {
                let _ = fs::remove_file(f);
            }
            Ok(())
        }
        Cmd::Mkdir { dirs } => {
            for d in &dirs {
                fs::create_dir_all(d)?;
            }
            Ok(())
        }
        Cmd::Pwd => {
            println!("{}", std::env::current_dir()?.display());
            Ok(())
        }
        Cmd::Tail { file, n } => tail(&file, n),
        Cmd::Head { file, n } => head(&file, n),
        Cmd::Cp { from, to } => {
            fs::copy(&from, &to)?;
            Ok(())
        }
        Cmd::Mv { from, to } => {
            fs::rename(&from, &to)?;
            Ok(())
        }
        Cmd::Uptime => uptime(),
        Cmd::Free => free(),
    }
}

fn cat(files: &[PathBuf]) -> Result<()> {
    use std::io::Write;
    let out = std::io::stdout();
    let mut out = out.lock();
    for f in files {
        let buf = fs::read(f)?;
        out.write_all(&buf)?;
    }
    Ok(())
}

fn ls(path: &std::path::Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let e = entry?;
        let name = e.file_name();
        println!("{}", name.to_string_lossy());
    }
    Ok(())
}

fn head(file: &std::path::Path, n: usize) -> Result<()> {
    let text = fs::read_to_string(file)?;
    for (i, line) in text.lines().enumerate() {
        if i >= n {
            break;
        }
        println!("{}", line);
    }
    Ok(())
}

fn tail(file: &std::path::Path, n: usize) -> Result<()> {
    let text = fs::read_to_string(file)?;
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    for line in &lines[start..] {
        println!("{}", line);
    }
    Ok(())
}

fn uptime() -> Result<()> {
    let raw = fs::read_to_string("/proc/uptime")?;
    let secs: f64 = raw
        .split_whitespace()
        .next()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0.0);
    let d = (secs / 86400.0) as u64;
    let h = ((secs % 86400.0) / 3600.0) as u64;
    let m = ((secs % 3600.0) / 60.0) as u64;
    println!("up {}d {}h {}m", d, h, m);
    Ok(())
}

fn free() -> Result<()> {
    let raw = fs::read_to_string("/proc/meminfo")?;
    let mut total = 0u64;
    let mut avail = 0u64;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total = v
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
        }
        if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail = v
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
        }
    }
    println!(
        "{:<12} {:>12} {:>12} {:>12}",
        "", "total", "used", "available"
    );
    println!(
        "{:<12} {:>12} {:>12} {:>12}",
        "Mem:",
        kb_to_human(total),
        kb_to_human(total - avail),
        kb_to_human(avail)
    );
    Ok(())
}

fn kb_to_human(kb: u64) -> String {
    if kb >= 1024 * 1024 {
        format!("{:.2}G", kb as f64 / (1024.0 * 1024.0))
    } else if kb >= 1024 {
        format!("{:.1}M", kb as f64 / 1024.0)
    } else {
        format!("{}K", kb)
    }
}
