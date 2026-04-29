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
use winsh_history::HistoryManager;
use winsh_prompt::{render_prompt, PromptContext};

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

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
                if args.len() < 3 {
                    eprintln!("winsh: -c requires an argument");
                    process::exit(1);
                }
                let cmd = &args[2];
                let exit_code = execute_command(cmd);
                process::exit(exit_code);
            }
            _ => {
                let script_path = &args[1];
                let exit_code = execute_script(script_path);
                process::exit(exit_code);
            }
        }
    }

    if let Err(e) = run_repl() {
        eprintln!("winsh: {}", e);
        process::exit(1);
    }
}

/// Run the interactive REPL.
fn run_repl() -> Result<()> {
    let mut state = ShellState::new();
    let mut executor = Executor::new();
    let mut history = HistoryManager::new();

    if let Err(e) = history.load() {
        debug!("Failed to load history: {}", e);
    }

    println!("WinSH {} - zsh-compatible shell for Windows", env!("CARGO_PKG_VERSION"));

    loop {
        let ctx = build_prompt_context(&state);
        let prompt = render_prompt(&state.config.prompt, &ctx);
        let input = read_line(&prompt)?;

        if input.is_none() {
            println!();
            break;
        }

        let input = input.unwrap();
        let trimmed = input.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('#') {
            continue;
        }

        history.add(trimmed);

        let expanded = if trimmed.contains('!') || trimmed.starts_with('^') {
            history.expand(trimmed).unwrap_or_else(|| trimmed.to_string())
        } else {
            trimmed.to_string()
        };

        if expanded.trim() == "exit" {
            break;
        }

        if expanded.trim() == "help" || expanded.trim() == "--help" {
            print_help();
            continue;
        }

        match execute_line(&expanded, &mut state, &mut executor) {
            Ok(code) => {
                debug!("Command exited with code: {}", code);
            }
            Err(ShellError::Exit(code)) => {
                history.save()?;
                process::exit(code);
            }
            Err(e) => {
                eprintln!("winsh: {}", e);
            }
        }
    }

    history.save()?;
    Ok(())
}

/// Build the prompt context for rendering.
fn build_prompt_context(state: &ShellState) -> PromptContext {
    PromptContext {
        cwd: state.current_dir().clone(),
        home: dirs::home_dir(),
        exit_code: state.exit_code(),
        job_count: 0,
        line_number: 1,
        shell_name: "winsh".to_string(),
        username: std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "user".to_string()),
        hostname: std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "localhost".to_string()),
    }
}

/// Read a line of input from the user.
fn read_line(prompt: &str) -> Result<Option<String>> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(input.trim_end().to_string())),
        Err(e) => Err(e.into()),
    }
}

/// Execute a line of input.
fn execute_line(input: &str, state: &mut ShellState, executor: &mut Executor) -> Result<i32, ShellError> {
    let tokens = Lexer::tokenize(input)?;
    let stmts = Parser::parse(tokens)?;
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
            Ok(code) => exit_code = code,
            Err(ShellError::Exit(code)) => return code,
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
    println!("{}", "=".repeat(50));
    println!("USAGE");
    println!("{}", "=".repeat(50));
    println!("  winsh               Start interactive shell");
    println!("  winsh -c 'CMD'      Execute command and exit");
    println!("  winsh SCRIPT        Execute script file");
    println!("  winsh --help        Show this help");
    println!("  winsh --version     Show version");
    println!();
    println!("{}", "=".repeat(50));
    println!("BUILT-IN COMMANDS");
    println!("{}", "=".repeat(50));
    println!("  cd [DIR]            Change directory");
    println!("  echo [-neE] [TXT]   Display text");
    println!("  exit [N]            Exit the shell");
    println!("  pwd                 Print working directory");
    println!("  printf FMT [ARGS]   Formatted output");
    println!("  read [-r] [-p P] N  Read input into variable");
    println!("  type CMD            Show command type");
    println!("  which CMD           Locate a command");
    println!("  alias [N[=V]]       Define or show aliases");
    println!("  unalias [-a] [N]    Remove aliases");
    println!("  export [N[=V]]      Export variables");
    println!("  unset [N]           Unset variables");
    println!("  source FILE         Execute script in current shell");
    println!("  true / false        Return success/failure");
    println!("  test EXPR           Evaluate conditional expression");
    println!("  history             Show command history");
    println!("  jobs                List background jobs");
    println!("  fg [%N]             Bring job to foreground");
    println!("  bg [%N]             Continue job in background");
    println!("  kill [%N]           Kill a job");
    println!("  wait [%N]           Wait for job to finish");
    println!("  disown [%N]         Remove job from job table");
    println!("  help                Show this help");
    println!();
    println!("{}", "=".repeat(50));
    println!("KEY FEATURES");
    println!("{}", "=".repeat(50));
    println!("  Zsh-style prompt with escape sequences (PS1/PROMPT)");
    println!("  History management with expansion (!!, !$, !n, ^old^new)");
    println!("  Vi and Emacs keybinding modes");
    println!("  Tab completion (commands, paths, variables)");
    println!("  Variable expansion (\\$VAR, \\${{VAR:-default}}, \\${{VAR#pattern}})");
    println!("  Arithmetic expansion \\$((expr))");
    println!("  Conditional expressions [[ ... ]]");
    println!("  Here Documents (<<EOF)");
    println!("  Pipeline support (cmd1 | cmd2)");
    println!("  Redirections (<, >, >>, 2>, 2>&1)");
    println!("  Background execution (cmd &)");
    println!("  Job control (fg, bg, jobs, kill, wait)");
    println!("  Function definitions (name() {{ ... }})");
    println!("  Control flow (if/for/while/until/case)");
    println!("  Shell options (setopt/unsetopt)");
    println!("  Plugin system with hooks (precmd, preexec, chpwd)");
    println!();
    println!("{}", "=".repeat(50));
    println!("CONFIGURATION");
    println!("{}", "=".repeat(50));
    println!("  ~/.winshrc        Interactive shell configuration");
    println!("  ~/.winshenv        Always-loaded configuration");
    println!("  ~/.winshprofile    Login shell configuration");
    println!("  ~/.winshlogout     Executed on exit");
    println!("  ~/.winsh_history   Command history file");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_echo() {
        assert_eq!(execute_command("echo hello"), 0);
    }

    #[test]
    fn test_execute_exit() {
        assert_eq!(execute_command("exit 42"), 42);
    }

    #[test]
    fn test_execute_unknown_command() {
        assert_ne!(execute_command("nonexistent_command_12345"), 0);
    }
}
