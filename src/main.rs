//! # WinSH - Windows Shell
//!
//! A zsh-compatible shell for Windows, written in Rust.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use anyhow::Result;
use tracing::{debug, info, warn, error};

use winsh_core::{ShellState, ShellError};
use winsh_lexer::Lexer;
use winsh_parser::Parser;
use winsh_exec::Executor;
use winsh_history::HistoryManager;
use winsh_prompt::{render_prompt, PromptContext};

fn main() {
    // Initialize tracing subscriber - controlled by RUST_LOG env var
    // Usage: RUST_LOG=debug winsh
    // Levels: error, warn, info, debug, trace
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .with_writer(std::io::stderr) // Log to stderr so it doesn't interfere with output
        .init();

    let args: Vec<String> = std::env::args().collect();
    debug!("args={:?}", args);

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
                debug!("-c mode, cmd={}", cmd);
                let exit_code = execute_command(cmd);
                process::exit(exit_code);
            }
            _ => {
                let script_path = &args[1];
                debug!("script mode, file={}", script_path);
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
        warn!("Failed to load history: {}", e);
    }

    // Load shell configuration
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let home_str = home.to_string_lossy().to_string();
    debug!("home={}", home.display());

    let winshrc = home.join(".winshrc");
    info!("Loading config: {}", winshrc.display());

    if winshrc.exists() {
        debug!("Config file found, reading...");
        match std::fs::read_to_string(&winshrc) {
            Ok(content) => {
                debug!("Config file loaded, {} bytes", content.len());
                process_config_content(&content, &home_str, &mut state, &mut executor);
                info!("Config loaded successfully");
            }
            Err(e) => {
                error!("Failed to read config: {}", e);
            }
        }
    } else {
        info!("No .winshrc found at {}", winshrc.display());
    }

    // Log final prompt state
    debug!("Final PROMPT='{}'", state.config.prompt);
    debug!("Final RPROMPT='{}'", state.config.rprompt);
    debug!("Final theme='{}'", state.config.theme);

    println!("WinSH {} - zsh-compatible shell for Windows", env!("CARGO_PKG_VERSION"));
    println!();

    loop {
        let ctx = build_prompt_context(&state);
        let prompt = render_prompt(&state.config.prompt, &ctx);
        let rprompt = if state.config.rprompt.is_empty() {
            String::new()
        } else {
            render_prompt(&state.config.rprompt, &ctx)
        };

        debug!("prompt='{}'", prompt.replace('\x1b', "\\e"));

        let input = read_line_with_rprompt(&prompt, &rprompt)?;

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

/// Process configuration content, expanding variables as we go.
fn process_config_content(
    content: &str,
    home_str: &str,
    state: &mut ShellState,
    executor: &mut Executor,
) {
    // Process lines, expanding variables from state as we go
    // This allows variables set earlier to be used later (e.g., WINUXSH_THEME)
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Expand known variables in the line
        let expanded = expand_config_line(trimmed, home_str, state);

        // Handle source command
        if let Some(path) = expanded.strip_prefix("source ") {
            let path = path.trim().trim_matches('"').trim_matches('\'');
            let source_path = PathBuf::from(path);
            info!("Sourcing: {}", source_path.display());
            if source_path.exists() {
                if let Ok(source_content) = std::fs::read_to_string(&source_path) {
                    debug!("Source file loaded, {} bytes", source_content.len());
                    process_config_content(&source_content, home_str, state, executor);
                }
            } else {
                warn!("Source file not found: {}", source_path.display());
            }
            continue;
        }

        // Handle variable assignments
        if let Some((name, value)) = expanded.split_once('=') {
            let name = name.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match name {
                "WINUXSH_THEME" => {
                    info!("Setting theme: {}", value);
                    state.config.theme = value.to_string();
                    state.env.set(name, value);
                }
                "PROMPT" | "PS1" => {
                    info!("Setting PROMPT: {}", value);
                    state.config.prompt = value.to_string();
                }
                "RPROMPT" => {
                    info!("Setting RPROMPT: {}", value);
                    state.config.rprompt = value.to_string();
                }
                _ if name.chars().all(|c| c.is_alphanumeric() || c == '_') => {
                    debug!("Setting var: {}={}", name, value);
                    state.env.set(name, value);
                }
                _ => {}
            }
            continue;
        }

        // Handle export
        if let Some(rest) = expanded.strip_prefix("export ") {
            let rest = rest.trim();
            if let Some((name, value)) = rest.split_once('=') {
                debug!("Exporting: {}={}", name.trim(), value.trim());
                state.env.export(name.trim(), value.trim().trim_matches('"').trim_matches('\''));
            }
            continue;
        }

        // Handle setopt
        if let Some(opt) = expanded.strip_prefix("setopt ") {
            let opt = opt.trim();
            debug!("setopt: {}", opt);
            apply_option(opt, true, state);
            continue;
        }

        // Handle alias
        if let Some(rest) = expanded.strip_prefix("alias ") {
            let rest = rest.trim();
            if let Some((name, value)) = rest.split_once('=') {
                let value = value.trim().trim_matches('\'').trim_matches('"');
                debug!("alias {}='{}'", name.trim(), value);
                state.set_alias(name.trim().to_string(), value.to_string());
            }
            continue;
        }
    }
}

/// Expand variables in a config line.
fn expand_config_line(line: &str, home_str: &str, state: &ShellState) -> String {
    let mut result = line.to_string();

    // Expand $HOME (but not inside %{} sequences which are prompt escapes)
    result = result.replace("$HOME", home_str);

    // Expand shell variables from state
    let mut vars: Vec<(String, String)> = state.env.all().into_iter().collect();
    vars.sort_by(|a, b| b.0.len().cmp(&a.0.len())); // Longest first

    for (name, value) in &vars {
        result = result.replace(&format!("${{{}}}", name), value);
        result = result.replace(&format!("${}", name), value);
    }

    // Don't expand ~ inside prompt escape sequences (%~ is a valid prompt escape)
    // Only expand ~ at the beginning of paths
    // This is handled by the caller if needed

    result
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
        username: std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string()),
        hostname: std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "localhost".to_string()),
    }
}

/// Read a line with right prompt support.
fn read_line_with_rprompt(prompt: &str, rprompt: &str) -> Result<Option<String>> {
    print!("{}", prompt);
    if !rprompt.is_empty() {
        print!("\x1b[s"); // Save cursor
        let width = term_width().unwrap_or(80);
        let rprompt_len = strip_ansi(rprompt).len();
        if rprompt_len < width as usize {
            let col = width as usize - rprompt_len;
            print!("\x1b[{}G{}", col, rprompt);
        }
        print!("\x1b[u"); // Restore cursor
    }
    io::stdout().flush()?;

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(input.trim_end().to_string())),
        Err(e) => Err(e.into()),
    }
}

/// Get terminal width.
fn term_width() -> Option<u16> {
    if let Some((w, _)) = terminal_size::terminal_size() {
        Some(w.0)
    } else {
        None
    }
}

/// Strip ANSI escape sequences.
fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == 'm' {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Apply a shell option.
fn apply_option(option: &str, value: bool, state: &mut ShellState) {
    let opts = state.options_mut();
    match option {
        "errexit" | "e" => opts.errexit = value,
        "nounset" | "u" => opts.nounset = value,
        "noglob" | "f" => opts.noglob = value,
        "extended_glob" => opts.extended_glob = value,
        "null_glob" => opts.null_glob = value,
        "glob_dots" => opts.glob_dots = value,
        "case_glob" => opts.case_glob = value,
        "hist_ignore_dups" => opts.hist_ignore_dups = value,
        "hist_ignore_all_dups" => opts.hist_ignore_all_dups = value,
        "hist_ignore_space" => opts.hist_ignore_space = value,
        "prompt_subst" => opts.prompt_subst = value,
        "prompt_percent" => opts.prompt_percent = value,
        "brace_expand" => opts.brace_expand = value,
        "tilde_expand" => opts.tilde_expand = value,
        "variable_expand" => opts.variable_expand = value,
        "command_subst" => opts.command_subst = value,
        "arith_expand" => opts.arith_expand = value,
        "monitor" | "m" => opts.monitor = value,
        _ => {}
    }
}

/// Execute a single config line (unused, kept for reference).
fn execute_config_line(line: &str, state: &mut ShellState, _executor: &mut Executor) {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("PROMPT=") {
        let value = rest.trim().trim_matches('"').trim_matches('\'');
        state.config.prompt = value.to_string();
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("PS1=") {
        let value = rest.trim().trim_matches('"').trim_matches('\'');
        state.config.prompt = value.to_string();
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("RPROMPT=") {
        let value = rest.trim().trim_matches('"').trim_matches('\'');
        state.config.rprompt = value.to_string();
        return;
    }
    if let Some((name, value)) = trimmed.split_once('=') {
        let name = name.trim();
        if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match name {
                "WINUXSH_THEME" => state.config.theme = value.to_string(),
                "PROMPT" | "PS1" => state.config.prompt = value.to_string(),
                "RPROMPT" => state.config.rprompt = value.to_string(),
                _ => { state.env.set(name, value); }
            }
        }
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
    println!("LOGGING");
    println!("{}", "=".repeat(50));
    println!("  Set RUST_LOG env var to control log level:");
    println!("    RUST_LOG=error winsh    Only errors");
    println!("    RUST_LOG=warn  winsh    Warnings + errors");
    println!("    RUST_LOG=info  winsh    Info + above");
    println!("    RUST_LOG=debug winsh    Debug + above");
    println!("    RUST_LOG=trace winsh    Everything");
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
    println!("CONFIGURATION");
    println!("{}", "=".repeat(50));
    println!("  ~/.winshrc          Interactive shell configuration");
    println!("  ~/.winsh_history    Command history file");
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
