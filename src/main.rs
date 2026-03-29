// WinSH MVP6 - Array Support and Internationalization
//
// MVP6 Features:
// - Array support (definition, access, expansion)
// - Internationalization (English only)
// - Enhanced config file support (terminal styling)
// - Plugin system support
// - Modular architecture following Rust best practices

use anyhow::Result;
use colored::Colorize;
use reedline::Signal;
use std::env;
use std::path::PathBuf;

mod array;
mod builtins;
mod config;
mod error;
mod executor;
mod job;
mod oh_my_winuxsh;
mod parser;
mod plugin;
mod shell;
mod theme;
mod tokenizer;

use shell::Shell;

fn print_usage() {
    println!("WinSH usage:");
    println!("  winuxsh -c \"command\"");
    println!("  winuxsh script.sh [args...]");
    println!("  winuxsh --help | -h");
    println!("  winuxsh --version");
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Initialize logging (default to error level only)
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Error)
        .init();

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "-c" => {
                if args.len() > 2 {
                    let mut shell = Shell::new(true)?;
                    if let Err(e) = shell.save_history(&args[2]) {
                        eprintln!(
                            "{} {}",
                            "Warning:".yellow(),
                            format!("Failed to save history: {}", e)
                        );
                    }
                    shell.execute_command(&args[2])?;
                } else {
                    eprintln!("{} {}", "Error:".red(), "-c requires an argument");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_usage();
            }
            "--version" => {
                println!(
                    "{}",
                    "WinSH MVP6 - Array Support and Internationalization version 0.6.0".green()
                );
            }
            _ => {
                // Check if it's a script file
                let script_path = PathBuf::from(&args[1]);
                if script_path.exists() {
                    let mut shell = Shell::new(true)?;
                    shell.run_script_file(&script_path, &args[2..])?;
                } else {
                    eprintln!("{} {}", "Unknown argument:".red(), args[1]);
                    print_usage();
                    std::process::exit(1);
                }
            }
        }
        return Ok(());
    }

    let mut shell = Shell::new(true)?;
    shell.run_repl()?;

    Ok(())
}

// Add this to shell module temporarily
impl Shell {
    pub fn run_repl(&mut self) -> Result<()> {
        println!(
            "{}",
            "WinSH MVP6 - Array Support and Internationalization".green()
        );
        println!("Type 'help' for available commands");
        println!();

        loop {
            let prompt = self.get_prompt();

            match self.line_editor.read_line(&prompt) {
                Ok(Signal::Success(buffer)) => {
                    let line = buffer.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if let Err(e) = self.save_history(line) {
                        eprintln!(
                            "{} {}",
                            "Warning:".yellow(),
                            format!("Failed to save history: {}", e)
                        );
                    }

                    // Execute command
                    if let Err(e) = self.execute_command(line) {
                        eprintln!("{} {}", "Error:".red(), e);
                    }
                }
                Ok(Signal::CtrlD) => {
                    println!();
                    println!("Goodbye!");
                    break;
                }
                Ok(Signal::CtrlC) => {
                    println!();
                    continue;
                }
                Err(e) => {
                    eprintln!("{} {}", "Error:".red(), e);
                    break;
                }
            }
        }

        Ok(())
    }
}
