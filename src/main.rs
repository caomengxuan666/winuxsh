//! # WinSH - Windows Shell
//!
//! A zsh-compatible shell for Windows, written in Rust.

use std::io::{self, Write};
use std::process;

use anyhow::Result;
use log::debug;

use winsh_core::{ShellState, ShellError};
use winsh_lexer::Lexer;
use winsh_parser::Parser;
use winsh_exec::Executor;

fn main() {
    // Initialize logging
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Handle --help and --version
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-V" => {
                println!("WinSH {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-c" => {
                // Execute command from argument
                if args.len() < 3 {
                    eprintln!("winsh: -c requires an argument");
                    process::exit(1);
                }
                let cmd = &args[2];
                let exit_code = execute_command(cmd);
                process::exit(exit_code);
            }
            _ => {
                // Assume it's a script file
                let script_path = &args[1];
                let exit_code = execute_script(script_path);
                process::exit(exit_code);
            }
        }
    }

    // Interactive mode - start REPL
    if let Err(e) = run_repl() {
        eprintln!("winsh: {}", e);
        process::exit(1);
    }
}

/// Run the interactive REPL.
fn run_repl() -> Result<()> {
    let mut state = ShellState::new();
    let mut executor = Executor::new();

    // Print welcome message
    println!("WinSH {}", env!("CARGO_PKG_VERSION"));
    println!("Type 'help' for help, 'exit' to exit.");
    println!();

    loop {
        // Get the prompt
        let prompt = get_prompt(&state);

        // Read input
        let input = read_line(&prompt)?;

        // Check for EOF
        if input.is_none() {
            println!();
            break;
        }

        let input = input.unwrap();

        // Skip empty input
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Add to history
        // TODO: Implement history

        // Check for exit
        if trimmed == "exit" {
            break;
        }

        // Execute the command
        match execute_line(trimmed, &mut state, &mut executor) {
            Ok(code) => {
                debug!("Command exited with code: {}", code);
            }
            Err(ShellError::Exit(code)) => {
                process::exit(code);
            }
            Err(e) => {
                eprintln!("winsh: {}", e);
            }
        }
    }

    Ok(())
}

/// Get the shell prompt.
fn get_prompt(state: &ShellState) -> String {
    let dir = state.current_dir();
    let dir_str = dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.display().to_string());

    format!("{} $ ", dir_str)
}

/// Read a line of input from the user.
fn read_line(prompt: &str) -> Result<Option<String>> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => Ok(None), // EOF
        Ok(_) => Ok(Some(input)),
        Err(e) => Err(e.into()),
    }
}

/// Execute a line of input.
fn execute_line(
    input: &str,
    state: &mut ShellState,
    executor: &mut Executor,
) -> Result<i32, ShellError> {
    // Tokenize
    let tokens = Lexer::tokenize(input)?;

    // Parse
    let stmts = Parser::parse(tokens)?;

    // Execute
    executor.execute(&stmts, state)
}

/// Execute a command string.
fn execute_command(cmd: &str) -> i32 {
    let mut state = ShellState::new();
    let mut executor = Executor::new();

    match execute_line(cmd, &mut state, &mut executor) {
        Ok(code) => code,
        Err(ShellError::Exit(code)) => code,
        Err(e) => {
            eprintln!("winsh: {}", e);
            1
        }
    }
}

/// Execute a script file.
fn execute_script(path: &str) -> i32 {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("winsh: {}: {}", path, e);
            return 1;
        }
    };

    let mut state = ShellState::new();
    let mut executor = Executor::new();
    let mut exit_code = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match execute_line(trimmed, &mut state, &mut executor) {
            Ok(code) => {
                exit_code = code;
            }
            Err(ShellError::Exit(code)) => {
                return code;
            }
            Err(e) => {
                eprintln!("winsh: {}", e);
                exit_code = 1;
            }
        }
    }

    exit_code
}

/// Print help message.
fn print_help() {
    println!("WinSH - A zsh-compatible shell for Windows");
    println!();
    println!("USAGE:");
    println!("    winsh              Start interactive shell");
    println!("    winsh -c 'CMD'     Execute command and exit");
    println!("    winsh SCRIPT       Execute script file");
    println!("    winsh --help       Show this help");
    println!("    winsh --version    Show version");
    println!();
    println!("BUILT-IN COMMANDS:");
    println!("    cd [DIR]           Change directory");
    println!("    echo [TEXT]        Display text");
    println!("    exit [N]           Exit the shell");
    println!("    pwd                Print working directory");
    println!("    type COMMAND       Show command type");
    println!("    help               Show this help");
    println!();
    println!("For more information, see the documentation.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_echo() {
        let code = execute_command("echo hello");
        assert_eq!(code, 0);
    }

    #[test]
    fn test_execute_exit() {
        let code = execute_command("exit 42");
        assert_eq!(code, 42);
    }

    #[test]
    fn test_execute_unknown_command() {
        let code = execute_command("nonexistent_command_12345");
        assert_ne!(code, 0);
    }
}
