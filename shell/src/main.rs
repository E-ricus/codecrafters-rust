use anyhow::Result;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::{env, fs, process};

const BUILTINS: [&str; 5] = ["type", "exit", "echo", "pwd", "cd"];

fn main() -> Result<()> {
    repl_loop()
}

fn repl_loop() -> Result<()> {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        // Wait for user input
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();
        handle_line(input)?
    }
}

fn handle_line(input: String) -> Result<()> {
    let input = input.trim();
    let lines = input.split_once(" ");
    match lines {
        Some(("echo", rest)) => println!("{rest}"),
        Some(("type", cmd)) => match BUILTINS.contains(&cmd) {
            true => println!("{} is a shell builtin", cmd),
            false => match handle_paths(cmd) {
                Ok(path) => println!("{} is {}", cmd, path),
                Err(_) => {
                    println!("{}: not found", cmd);
                }
            },
        },
        Some(("exit", _)) => process::exit(0),
        Some(("cd", path)) => {
            let home = env::var("HOME")?;
            let path = path.replace("~", home.as_str());
            match env::set_current_dir(Path::new(path.as_str())) {
                Ok(_) => {}
                Err(e) => {
                    if matches!(e.kind(), io::ErrorKind::NotFound) {
                        println!("cd: {path}: No such file or directory")
                    }
                }
            }
        }
        Some((cmd, args)) => match handle_paths(cmd) {
            Ok(path) => {
                let args: Vec<&str> = args.split_whitespace().collect();
                let output = Command::new(path).args(args).output()?;
                io::stdout().write_all(&output.stdout)?;
                io::stderr().write_all(&output.stderr)?;
            }
            Err(_) => println!("{}: command not found", input),
        },
        _ => {
            // Add at the beginning?
            if input == "pwd" {
                let current = env::current_dir()?;
                println!("{}", current.display());
                return Ok(());
            }
            println!("{}: command not found", input)
        }
    }
    Ok(())
}

fn handle_paths(cmd: &str) -> Result<String> {
    let path = env::var("PATH")?;
    let paths = path.split(":");
    for path in paths {
        let entries = fs::read_dir(path)?;
        for e in entries {
            let entry = e?;
            if entry.file_name() == cmd {
                return Ok(entry.path().display().to_string());
            }
        }
    }
    Err(anyhow::anyhow!("Not found"))
}
