//! Built-in commands for novai-shell.

use std::env;
use std::path::PathBuf;

pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "cd" | "pwd"
            | "exit"
            | "export"
            | "echo"
            | "set"
            | "alias"
            | "history"
            | "help"
            | "source"
            | "which"
            | "true"
            | "false"
    )
}

pub fn run(argv: &[String]) -> anyhow::Result<i32> {
    match argv[0].as_str() {
        "cd" => cd(argv),
        "pwd" => {
            println!("{}", env::current_dir()?.display());
            Ok(0)
        }
        "exit" => {
            let code = argv.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            std::process::exit(code);
        }
        "export" => export(argv),
        "echo" => {
            println!("{}", argv[1..].join(" "));
            Ok(0)
        }
        "set" => set(argv),
        "alias" => {
            for a in &argv[1..] {
                println!("alias {}", a);
            }
            Ok(0)
        }
        "history" => {
            println!("(history is in $XDG_CACHE_HOME/novai-shell-history.txt)");
            Ok(0)
        }
        "source" => source(argv),
        "which" => {
            for n in &argv[1..] {
                match which::which(n) {
                    Ok(p) => println!("{}", p.display()),
                    Err(_) => eprintln!("not found: {}", n),
                }
            }
            Ok(0)
        }
        "true" => Ok(0),
        "false" => Ok(1),
        "help" => {
            print_help();
            Ok(0)
        }
        _ => Ok(127),
    }
}

fn cd(argv: &[String]) -> anyhow::Result<i32> {
    let target = match argv.get(1) {
        Some(p) if p == "~" => env::var("HOME").unwrap_or_else(|_| "/".into()),
        Some(p) => p.clone(),
        None => env::var("HOME").unwrap_or_else(|_| "/".into()),
    };
    let path = PathBuf::from(&target);
    env::set_current_dir(&path).map_err(|e| anyhow::anyhow!("cd {}: {}", target, e))?;
    Ok(0)
}

fn export(argv: &[String]) -> anyhow::Result<i32> {
    for a in &argv[1..] {
        if let Some((k, v)) = a.split_once('=') {
            env::set_var(k, v);
        }
    }
    Ok(0)
}

fn set(argv: &[String]) -> anyhow::Result<i32> {
    for a in &argv[1..] {
        if let Some((k, v)) = a.split_once('=') {
            env::set_var(k, v);
        } else {
            println!("{}", env::var(a).unwrap_or_else(|_| "".into()));
        }
    }
    Ok(0)
}

fn source(argv: &[String]) -> anyhow::Result<i32> {
    let path = argv
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("source: missing file"))?;
    let content = std::fs::read_to_string(path)?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Use the same logic as run_line — split into argv and dispatch.
        let argv: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
        if argv.is_empty() {
            continue;
        }
        let _ = run(&argv);
    }
    Ok(0)
}

fn print_help() {
    println!("NovaiOS shell — built-ins:");
    println!("  cd [dir]         change directory");
    println!("  pwd              print working directory");
    println!("  exit [code]      exit shell");
    println!("  export KEY=VAL   set env var");
    println!("  set KEY=VAL      set env var");
    println!("  echo [args...]   print args");
    println!("  source <file>    execute a script in the current shell");
    println!("  which <cmd>      locate a command");
    println!("  history          show where history is stored");
    println!("  help             show this help");
}
