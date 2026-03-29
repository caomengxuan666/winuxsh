use colored::Colorize;
use reedline::{
    default_emacs_keybindings, ColumnarMenu, DefaultCompleter, DefaultPrompt, DefaultPromptSegment,
    Emacs, FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder, Reedline, ReedlineEvent,
    ReedlineMenu,
};
use std::collections::HashMap;
use std::env;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use crate::array::ArrayValue;
use crate::config::ShellConfig;
use crate::error::{Result, ShellError};
use crate::executor::Executor;
use crate::job::JobManager;
use crate::parser::Parser;
use crate::plugin::PluginManager;
use crate::theme::ThemePlugin;
use crate::tokenizer::{CommandInfo, ParsedCommand, Tokenizer};
use glob;

/// Main shell structure
pub struct Shell {
    pub current_dir: PathBuf,
    pub aliases: HashMap<String, String>,
    pub env_vars: HashMap<String, ArrayValue>,
    pub line_editor: Reedline,
    pub history_path: PathBuf,
    pub config: ShellConfig,
    pub plugins: PluginManager,
    pub job_manager: JobManager,
    pub theme_plugin: ThemePlugin,
    pub last_exit_code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptFlow {
    None,
    Break,
    Continue,
}

#[derive(Debug, Default)]
struct ScriptState {
    positional: Vec<String>,
    locals: HashMap<String, String>,
}

impl ScriptState {
    fn new(args: &[String]) -> Self {
        Self {
            positional: args.to_vec(),
            locals: HashMap::new(),
        }
    }

    fn shift(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        if n >= self.positional.len() {
            self.positional.clear();
            return;
        }
        self.positional.drain(0..n);
    }

    fn positional(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(|s| s.as_str())
    }
}

impl Shell {
    /// Create a new shell instance
    pub fn new(load_config: bool) -> Result<Self> {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let history_path = home_dir.join(".winsh_history");

        // Get commands from PATH for completion
        fn get_path_commands() -> Vec<String> {
            let mut commands = Vec::new();

            if let Ok(path_env) = std::env::var("PATH") {
                for path in env::split_paths(&path_env) {
                    if let Ok(entries) = std::fs::read_dir(path) {
                        for entry in entries.flatten() {
                            if let Ok(file_type) = entry.file_type() {
                                if file_type.is_file() {
                                    let file_name = entry.file_name().to_string_lossy().to_string();
                                    // Check if it's executable by extension
                                    let is_executable = file_name.ends_with(".exe")
                                        || file_name.ends_with(".bat")
                                        || file_name.ends_with(".cmd")
                                        || file_name.ends_with(".ps1");

                                    if is_executable {
                                        // Remove extension for cleaner completion
                                        let name_without_ext =
                                            if let Some(pos) = file_name.rfind('.') {
                                                file_name[..pos].to_string()
                                            } else {
                                                file_name.clone()
                                            };
                                        commands.push(name_without_ext);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            commands
        }

        let path_commands = get_path_commands();

        // Create command list for completion (built-in + PATH commands)
        let builtin_commands = vec![
            "ls".to_string(),
            "cd".to_string(),
            "pwd".to_string(),
            "echo".to_string(),
            "exit".to_string(),
            "clear".to_string(),
            "cat".to_string(),
            "grep".to_string(),
            "find".to_string(),
            "cp".to_string(),
            "mv".to_string(),
            "rm".to_string(),
            "mkdir".to_string(),
            "jobs".to_string(),
            "fg".to_string(),
            "bg".to_string(),
            "set".to_string(),
            "unset".to_string(),
            "export".to_string(),
            "env".to_string(),
            "help".to_string(),
            "history".to_string(),
            "alias".to_string(),
            "unalias".to_string(),
            "source".to_string(),
            "array".to_string(),
            "plugin".to_string(),
            "theme".to_string(),
            "oh-my-winuxsh".to_string(),
        ];

        let all_commands: Vec<String> = builtin_commands.into_iter().chain(path_commands).collect();

        // Sort and deduplicate commands
        let mut unique_commands: Vec<_> = all_commands.into_iter().collect();
        unique_commands.sort();
        unique_commands.dedup();

        // Create completer with all commands
        let completer = Box::new(DefaultCompleter::new_with_wordlen(unique_commands, 2));

        // Create completion menu (exactly like MVP4)
        let completion_menu = Box::new(
            ColumnarMenu::default()
                .with_name("completion_menu")
                .with_marker("? "),
        );

        // Setup TAB key binding for completion (exactly like MVP4)
        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("completion_menu".to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );

        let edit_mode = Box::new(Emacs::new(keybindings));

        // Create history with a graceful fallback when file access is denied.
        // This keeps tests and restricted environments from panicking.
        let history = match FileBackedHistory::with_file(1000, history_path.clone()) {
            Ok(h) => h,
            Err(e) => {
                eprintln!(
                    "{} {}",
                    "Warning:".yellow(),
                    format!(
                        "Failed to open history file, using in-memory history: {}",
                        e
                    )
                );
                match FileBackedHistory::new(1000) {
                    Ok(h) => h,
                    Err(fallback_err) => {
                        return Err(crate::error::ShellError::Config(format!(
                            "Failed to initialize history: {}; fallback failed: {}",
                            e, fallback_err
                        )));
                    }
                }
            }
        };

        // Create line editor with edit mode and completion (exactly like MVP4)
        let line_editor = Reedline::create()
            .with_completer(completer)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
            .with_edit_mode(edit_mode)
            .with_history(Box::new(history))
            .with_quick_completions(true)
            .with_partial_completions(true);

        let mut shell = Shell {
            current_dir: std::env::current_dir()?,
            aliases: HashMap::new(),
            env_vars: HashMap::new(),
            line_editor,
            history_path,
            config: ShellConfig::default(),
            plugins: PluginManager::new(),
            job_manager: JobManager::new(),
            theme_plugin: ThemePlugin::new(),
            last_exit_code: 0,
        };

        // Load default aliases
        shell.aliases.insert("ll".to_string(), "ls -la".to_string());
        shell.aliases.insert("la".to_string(), "ls -a".to_string());
        shell.aliases.insert("l".to_string(), "ls".to_string());

        // Load environment variables
        for (key, value) in std::env::vars() {
            shell.env_vars.insert(key, ArrayValue::String(value));
        }

        // Automatically add winuxcmd directory to PATH (MVP5 compatibility)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let winuxcmd_dir = exe_dir.join("winuxcmd");
                if winuxcmd_dir.exists() {
                    let winuxcmd_path = winuxcmd_dir.to_string_lossy().to_string();
                    if let Some(path_value) = shell
                        .env_vars
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
                        .map(|(_, v)| v.clone())
                    {
                        // Add winuxcmd to end of PATH to avoid affecting system commands
                        let new_path = format!("{};{}", path_value, winuxcmd_path);
                        shell
                            .env_vars
                            .insert("PATH".to_string(), ArrayValue::String(new_path.clone()));
                        std::env::set_var("PATH", &new_path);
                    } else {
                        shell.env_vars.insert(
                            "PATH".to_string(),
                            ArrayValue::String(winuxcmd_path.clone()),
                        );
                        std::env::set_var("PATH", &winuxcmd_path);
                    }
                }
            }
        }

        // Load configuration
        if load_config {
            if let Err(e) = shell.load_config() {
                eprintln!(
                    "{} {}",
                    "Warning:".yellow(),
                    format!("Failed to load config: {}", e)
                );
            }
        }

        // Initialize plugins
        use crate::oh_my_winuxsh::OhMyWinuxsh;
        use crate::plugin::WelcomePlugin;

        // Add welcome plugin
        if let Err(e) = shell.plugins.add_plugin(Box::new(WelcomePlugin)) {
            eprintln!(
                "{} {}",
                "Warning:".yellow(),
                format!("Failed to load welcome plugin: {}", e)
            );
        }

        // Add oh-my-winuxsh plugin
        if let Err(e) = shell.plugins.add_plugin(Box::new(OhMyWinuxsh)) {
            eprintln!(
                "{} {}",
                "Warning:".yellow(),
                format!("Failed to load oh-my-winuxsh plugin: {}", e)
            );
        }

        Ok(shell)
    }

    /// Load configuration
    fn load_config(&mut self) -> Result<()> {
        use crate::config::ConfigManager;

        // Load shell config
        if let Some(config_path) = ConfigManager::find_config_file() {
            if config_path
                .extension()
                .map(|e| e == "toml")
                .unwrap_or(false)
            {
                println!(
                    "{} {}",
                    "Loading shell config:".cyan(),
                    config_path.display()
                );
                let mut config_manager = ConfigManager::new();
                self.config = config_manager.load_config(&config_path)?;
            } else {
                println!("{} {}", "Loading config:".cyan(), config_path.display());
                self.parse_config_file(&config_path)?;
            }
        }

        Ok(())
    }

    /// Parse configuration file
    pub fn parse_config_file(&mut self, path: &Path) -> Result<()> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Execute config commands
            // TODO: Implement proper command execution
            let _ = line;
        }

        Ok(())
    }

    /// Get environment variable
    pub fn get_env_var(&self, key: &str, default: &str) -> String {
        self.env_vars
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| ArrayValue::String(default.to_string()))
            .as_string()
            .unwrap_or(default)
            .to_string()
    }

    /// Get the prompt string
    pub fn get_prompt(&self) -> DefaultPrompt {
        let username = self.get_env_var("USERNAME", "user");
        let hostname = self.get_env_var("COMPUTERNAME", "localhost");
        let dir = self.current_dir.display().to_string();

        // Use theme plugin if available, otherwise use default colors
        let prompt_text = if let ThemePlugin::Theme(ref theme) = self.theme_plugin {
            theme.generate_prompt(&username, &hostname, &dir, "$ ")
        } else {
            // Default colored prompt
            format!(
                "\x1b[1;32m{}@{}\x1b[0m \x1b[1;34m{}\x1b[0m $ ",
                username, hostname, dir
            )
        };

        DefaultPrompt::new(
            DefaultPromptSegment::Basic(prompt_text),
            DefaultPromptSegment::Empty,
        )
    }

    /// Parse ANSI escape sequences from config format
    fn parse_ansi_sequence(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(&c) = chars.peek() {
            chars.next();

            if c == '\\' {
                if let Some(&'x') = chars.peek() {
                    chars.next(); // consume 'x'
                    if let Some(&'1') = chars.peek() {
                        chars.next(); // consume '1'
                        if let Some(&'b') = chars.peek() {
                            chars.next(); // consume 'b'
                                          // This is \x1b, add actual ANSI escape
                            result.push('\x1b');
                        } else {
                            result.push_str("\\x1");
                        }
                    } else {
                        result.push_str("\\x");
                    }
                } else {
                    result.push(c);
                }
            } else if c == '\x1b' {
                // Already an escape sequence, keep it
                result.push(c);
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Save command to history
    pub fn save_history(&mut self, command: &str) -> Result<()> {
        let clean_command =
            command.trim_matches(|c: char| c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace());

        if clean_command.is_empty() {
            return Ok(());
        }

        let mut history = if self.history_path.exists() {
            std::fs::read_to_string(&self.history_path)?
        } else {
            String::new()
        };

        if !history.is_empty() {
            history.push('\n');
        }
        history.push_str(clean_command);

        std::fs::write(&self.history_path, history)?;

        Ok(())
    }

    /// Execute a command
    pub fn execute_command(&mut self, command: &str) -> Result<()> {
        // Tokenize the command
        let tokens = Tokenizer::tokenize(command)?;

        // Parse the tokens into an AST
        let parsed = Parser::parse(&tokens)?;

        // Execute the parsed command
        self.execute_parsed(&parsed)?;

        Ok(())
    }

    /// Execute a parsed command
    pub fn execute_parsed(&mut self, parsed: &ParsedCommand) -> Result<()> {
        match parsed {
            ParsedCommand::Single(cmd) => {
                self.execute_single_command(cmd)?;
            }
            ParsedCommand::Pipeline(cmds) => {
                self.execute_pipeline(cmds)?;
            }
            ParsedCommand::And(left, right) => {
                // Execute left command, only execute right if exit code is 0
                self.execute_parsed(left)?;
                if self.last_exit_code == 0 {
                    self.execute_parsed(right)?;
                }
            }
            ParsedCommand::Or(left, right) => {
                // Execute left command, only execute right if exit code is non-zero
                self.execute_parsed(left)?;
                if self.last_exit_code != 0 {
                    self.execute_parsed(right)?;
                }
            }
            ParsedCommand::Sequence(cmds) => {
                // Execute commands in sequence
                for cmd in cmds {
                    self.execute_parsed(cmd)?;
                }
            }
        }
        Ok(())
    }

    /// Execute a single command
    pub fn execute_single_command(&mut self, cmd: &CommandInfo) -> Result<()> {
        // Skip empty commands
        if cmd.args.is_empty() {
            return Ok(());
        }

        // Clone the command info for modification
        let mut cmd_clone = cmd.clone();

        // Expand aliases
        let first_arg = &cmd_clone.args[0];
        if let Some(alias_cmd) = self.aliases.get(first_arg) {
            let alias_parts: Vec<String> = alias_cmd
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            if !alias_parts.is_empty() {
                cmd_clone.args[0] = alias_parts[0].clone();
                cmd_clone
                    .args
                    .splice(1..1, alias_parts[1..].iter().cloned());
            }
        }

        // Get command name
        let clean_command = cmd_clone.args[0]
            .trim_matches(|c: char| c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace())
            .to_string();

        // Expand command substitution in arguments
        let args_with_substitution: Vec<String> = cmd_clone.args[1..]
            .iter()
            .map(|arg| self.expand_command_substitution(arg))
            .collect();

        // Expand wildcards in arguments (skip the command name)
        let expanded_args = self.expand_wildcards(&args_with_substitution);

        // Combine command name with expanded arguments
        let all_args: Vec<String> = vec![clean_command.clone()]
            .into_iter()
            .chain(expanded_args)
            .collect();

        if self.try_builtin_with_redirection(&clean_command, &all_args, &cmd_clone)? {
            self.last_exit_code = 0;
            return Ok(());
        }

        // Now check if it's a built-in command with expanded arguments
        if let Some(result) = self.handle_builtin(&all_args) {
            match result {
                Ok(()) => {
                    self.last_exit_code = 0;
                    return Ok(());
                }
                Err(e) => {
                    self.last_exit_code = 1;
                    eprintln!("{} {}", "Error:".red(), e);
                    return Ok(());
                }
            }
        }

        let args: Vec<String> = all_args[1..].to_vec();

        // Convert environment variables to the format expected by Executor
        let env_vars: Vec<(String, ArrayValue)> = self
            .env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Create executor
        let executor = Executor::new(&env_vars, &self.current_dir);

        // Execute the external command
        let mut cmd_info = cmd_clone;
        cmd_info.args = all_args;

        match executor.execute(&clean_command, &args, &cmd_info) {
            Ok(exit_code) => {
                self.last_exit_code = exit_code;
                Ok(())
            }
            Err(e) => {
                self.last_exit_code = match e {
                    ShellError::CommandNotFound(_) => 127,
                    _ => 1,
                };
                eprintln!("{} {}", "Error:".red(), e);
                Ok(())
            }
        }
    }

    /// Execute a pipeline
    pub fn execute_pipeline(&mut self, cmds: &[CommandInfo]) -> Result<()> {
        // Simplified pipeline execution
        // TODO: Implement proper pipeline with process pipes
        for cmd in cmds {
            self.execute_single_command(cmd)?;
        }
        Ok(())
    }

    /// Expand wildcards in arguments
    pub fn expand_wildcards(&self, args: &[String]) -> Vec<String> {
        let mut expanded = Vec::new();

        for arg in args {
            if arg.contains('*') || arg.contains('?') || arg.contains('[') {
                // Expand wildcard
                if let Ok(matches) = glob::glob(arg) {
                    for entry in matches.flatten() {
                        expanded.push(entry.to_string_lossy().to_string());
                    }
                    // If no matches found, keep the original pattern
                    if expanded.is_empty() || expanded.last() != Some(arg) {
                        expanded.push(arg.clone());
                    }
                } else {
                    // Invalid pattern, keep original
                    expanded.push(arg.clone());
                }
            } else {
                // No wildcard, keep as is
                expanded.push(arg.clone());
            }
        }

        expanded
    }

    /// Expand command substitution $(...)
    pub fn expand_command_substitution(&mut self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                if let Some(&'(') = chars.peek() {
                    // Start of command substitution
                    chars.next(); // consume '('
                    let mut command = String::new();
                    let mut depth = 1;

                    while let Some(&c) = chars.peek() {
                        chars.next(); // consume char
                        if c == '(' {
                            depth += 1;
                            command.push(c);
                        } else if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break; // End of command substitution
                            } else {
                                command.push(c);
                            }
                        } else {
                            command.push(c);
                        }
                    }

                    // Execute the command and capture output
                    let output = self.execute_and_capture(&command);
                    result.push_str(&output.trim());
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Execute command and capture output
    fn execute_and_capture(&mut self, command: &str) -> String {
        use std::process::Command;

        match Command::new("cmd").args(["/C", command]).output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(_) => String::new(),
        }
    }

    fn try_builtin_with_redirection(
        &mut self,
        command: &str,
        args: &[String],
        cmd_info: &CommandInfo,
    ) -> Result<bool> {
        let has_redirection = cmd_info.stdin_redir.is_some()
            || cmd_info.stdout_redir.is_some()
            || cmd_info.stderr_redir.is_some()
            || cmd_info.stderr_to_stdout
            || cmd_info.stdout_to_stderr;
        if !has_redirection {
            return Ok(false);
        }

        match command {
            "echo" => {
                let mut output = args[1..].join(" ");
                output.push('\n');
                self.write_builtin_output(&output, cmd_info)?;
                Ok(true)
            }
            "pwd" => {
                let output = format!("{}\n", self.current_dir.display());
                self.write_builtin_output(&output, cmd_info)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn write_builtin_output(&self, output: &str, cmd_info: &CommandInfo) -> Result<()> {
        // If stdout is redirected to stderr (1>&2), emit to stderr destination.
        if cmd_info.stdout_to_stderr {
            if let Some(ref stderr_file) = cmd_info.stderr_redir {
                self.write_to_file(stderr_file, cmd_info.stderr_append, output)?;
            } else {
                eprint!("{}", output);
            }
        } else if let Some(ref stdout_file) = cmd_info.stdout_redir {
            self.write_to_file(stdout_file, cmd_info.stdout_append, output)?;
        } else {
            print!("{}", output);
        }

        // If stderr is redirected to file for a builtin with no stderr output,
        // still materialize the file (bash-compatible touch/truncate behavior).
        if let Some(ref stderr_file) = cmd_info.stderr_redir {
            if !cmd_info.stdout_to_stderr {
                let _ = self.open_file_for_redirect(stderr_file, cmd_info.stderr_append)?;
            }
        }

        Ok(())
    }

    fn write_to_file(&self, path: &str, append: bool, content: &str) -> Result<()> {
        use std::io::Write;
        let mut file = self.open_file_for_redirect(path, append)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    fn open_file_for_redirect(&self, path: &str, append: bool) -> Result<std::fs::File> {
        let file = if append {
            std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)?
        } else {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?
        };
        Ok(file)
    }

    /// Execute a script file with basic script semantics and positional args.
    pub fn run_script_file(&mut self, script_path: &Path, script_args: &[String]) -> Result<()> {
        let script_content = std::fs::read_to_string(script_path)?;
        let lines: Vec<String> = script_content.lines().map(|s| s.to_string()).collect();
        let mut state = ScriptState::new(script_args);
        let _ = self.execute_script_lines(&lines, 0, lines.len(), &mut state)?;
        Ok(())
    }

    fn execute_script_lines(
        &mut self,
        lines: &[String],
        start: usize,
        end: usize,
        state: &mut ScriptState,
    ) -> Result<ScriptFlow> {
        let mut i = start;
        while i < end {
            let line = Self::normalize_script_line(&lines[i]);
            if line.is_empty() || line.starts_with('#') {
                i += 1;
                continue;
            }

            if line == "break" {
                return Ok(ScriptFlow::Break);
            }
            if line == "continue" {
                return Ok(ScriptFlow::Continue);
            }

            if line.starts_with("while ") {
                let (condition, body_start, body_end, next_index) =
                    self.parse_while_block(lines, i, end)?;
                loop {
                    let expanded_condition = self.expand_script_vars(&condition, state);
                    self.execute_command(&expanded_condition)?;
                    if self.last_exit_code != 0 {
                        break;
                    }

                    match self.execute_script_lines(lines, body_start, body_end, state)? {
                        ScriptFlow::None => {}
                        ScriptFlow::Break => break,
                        ScriptFlow::Continue => continue,
                    }
                }
                i = next_index;
                continue;
            }

            if line.starts_with("case ") {
                i = self.execute_case_block(lines, i, end, state)?;
                continue;
            }

            self.execute_script_simple_line(&line, state)?;
            i += 1;
        }

        Ok(ScriptFlow::None)
    }

    fn parse_while_block(
        &self,
        lines: &[String],
        start: usize,
        end: usize,
    ) -> Result<(String, usize, usize, usize)> {
        let header = Self::normalize_script_line(&lines[start]);
        let mut condition = header.trim_start_matches("while").trim().to_string();
        let mut body_start = start + 1;

        if condition.ends_with("; do") {
            condition = condition.trim_end_matches("; do").trim().to_string();
        } else if condition.ends_with(" do") {
            condition = condition.trim_end_matches(" do").trim().to_string();
        } else {
            let mut cursor = start + 1;
            while cursor < end {
                let candidate = Self::normalize_script_line(&lines[cursor]);
                if candidate.is_empty() || candidate.starts_with('#') {
                    cursor += 1;
                    continue;
                }
                if candidate == "do" {
                    body_start = cursor + 1;
                    break;
                }
                return Err(ShellError::InvalidCommand(
                    "while syntax expects 'do'".to_string(),
                ));
            }
        }

        if condition.ends_with(';') {
            condition.pop();
            condition = condition.trim().to_string();
        }
        if condition.is_empty() {
            return Err(ShellError::InvalidCommand(
                "while condition cannot be empty".to_string(),
            ));
        }

        let mut depth = 1usize;
        let mut cursor = body_start;
        while cursor < end {
            let candidate = Self::normalize_script_line(&lines[cursor]);
            if candidate.starts_with("while ") {
                depth += 1;
            } else if candidate == "done" {
                depth -= 1;
                if depth == 0 {
                    let body_end = cursor;
                    let next_index = cursor + 1;
                    return Ok((condition, body_start, body_end, next_index));
                }
            }
            cursor += 1;
        }

        Err(ShellError::InvalidCommand(
            "while block missing 'done'".to_string(),
        ))
    }

    fn execute_case_block(
        &mut self,
        lines: &[String],
        start: usize,
        end: usize,
        state: &mut ScriptState,
    ) -> Result<usize> {
        let header = Self::normalize_script_line(&lines[start]);
        if !header.ends_with(" in") {
            return Err(ShellError::InvalidCommand(
                "case syntax expects 'case <word> in'".to_string(),
            ));
        }

        let word_expr = header
            .trim_start_matches("case")
            .trim_end_matches(" in")
            .trim();
        let word_expanded = self.expand_script_vars(word_expr, state);
        let case_word = Self::strip_quotes(word_expanded.trim());

        let mut depth = 1usize;
        let mut esac_index = None;
        let mut i = start + 1;
        while i < end {
            let line = Self::normalize_script_line(&lines[i]);
            if line.starts_with("case ") {
                depth += 1;
            } else if line == "esac" {
                depth -= 1;
                if depth == 0 {
                    esac_index = Some(i);
                    break;
                }
            }
            i += 1;
        }

        let esac = esac_index
            .ok_or_else(|| ShellError::InvalidCommand("case block missing 'esac'".to_string()))?;

        let mut cursor = start + 1;
        let mut matched = false;
        while cursor < esac {
            let line = Self::normalize_script_line(&lines[cursor]);
            if line.is_empty() || line.starts_with('#') {
                cursor += 1;
                continue;
            }

            if let Some(close_paren) = line.find(')') {
                let patterns = line[..close_paren].trim();
                let remainder = line[close_paren + 1..].trim();
                let is_match = !matched && self.case_pattern_matches(patterns, &case_word);

                if remainder.ends_with(";;") {
                    if is_match {
                        let cmd = remainder.trim_end_matches(";;").trim();
                        if !cmd.is_empty() {
                            self.execute_script_simple_line(cmd, state)?;
                        }
                        matched = true;
                    }
                    cursor += 1;
                    continue;
                }

                cursor += 1;
                while cursor < esac {
                    let branch_line = Self::normalize_script_line(&lines[cursor]);
                    if branch_line.ends_with(";;") {
                        if is_match {
                            let cmd = branch_line.trim_end_matches(";;").trim();
                            if !cmd.is_empty() {
                                self.execute_script_simple_line(cmd, state)?;
                            }
                            matched = true;
                        }
                        break;
                    }

                    if is_match {
                        self.execute_script_simple_line(&branch_line, state)?;
                    }
                    cursor += 1;
                }
            }
            cursor += 1;
        }

        Ok(esac + 1)
    }

    fn case_pattern_matches(&self, patterns: &str, value: &str) -> bool {
        for pattern in patterns.split('|') {
            let pat = Self::strip_quotes(pattern.trim());
            if pat == "*" {
                return true;
            }
            if pat.contains('*') || pat.contains('?') || pat.contains('[') {
                if let Ok(glob_pattern) = glob::Pattern::new(&pat) {
                    if glob_pattern.matches(value) {
                        return true;
                    }
                }
            } else if pat == value {
                return true;
            }
        }
        false
    }

    fn execute_script_simple_line(&mut self, line: &str, state: &mut ScriptState) -> Result<()> {
        let trimmed = line.trim();
        if trimmed.starts_with("shift") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            let shift_n = if parts.len() > 1 {
                parts[1].parse::<usize>().unwrap_or(1)
            } else {
                1
            };
            state.shift(shift_n);
            return Ok(());
        }

        let expanded = self.expand_script_vars(trimmed, state);
        if expanded.is_empty() {
            return Ok(());
        }

        let parts: Vec<&str> = expanded.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let mut idx = 0usize;
        while idx < parts.len() && Self::is_assignment_token(parts[idx]) {
            if let Some((key, value)) = parts[idx].split_once('=') {
                state.locals.insert(key.to_string(), value.to_string());
                self.env_vars
                    .insert(key.to_string(), ArrayValue::String(value.to_string()));
                std::env::set_var(key, value);
            }
            idx += 1;
        }

        if idx >= parts.len() {
            return Ok(());
        }

        let command = parts[idx..].join(" ");
        self.execute_command(&command)
    }

    fn is_assignment_token(token: &str) -> bool {
        if let Some((key, _)) = token.split_once('=') {
            if key.is_empty() {
                return false;
            }
            let mut chars = key.chars();
            if let Some(first) = chars.next() {
                if !(first.is_ascii_alphabetic() || first == '_') {
                    return false;
                }
            }
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        } else {
            false
        }
    }

    fn expand_script_vars(&self, input: &str, state: &ScriptState) -> String {
        let mut out = String::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch != '$' {
                out.push(ch);
                continue;
            }

            let Some(next) = chars.peek().copied() else {
                out.push('$');
                break;
            };

            match next {
                '{' => {
                    chars.next();
                    let mut name = String::new();
                    while let Some(c) = chars.peek().copied() {
                        chars.next();
                        if c == '}' {
                            break;
                        }
                        name.push(c);
                    }
                    out.push_str(&self.resolve_script_var(&name, state));
                }
                '#' => {
                    chars.next();
                    out.push_str(&state.positional.len().to_string());
                }
                '@' | '*' => {
                    chars.next();
                    out.push_str(&state.positional.join(" "));
                }
                c if c.is_ascii_digit() => {
                    let mut index = String::new();
                    while let Some(d) = chars.peek().copied() {
                        if d.is_ascii_digit() {
                            chars.next();
                            index.push(d);
                        } else {
                            break;
                        }
                    }
                    let idx = index.parse::<usize>().unwrap_or(0);
                    if idx > 0 {
                        if let Some(value) = state.positional(idx - 1) {
                            out.push_str(value);
                        }
                    }
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let mut name = String::new();
                    while let Some(c2) = chars.peek().copied() {
                        if c2.is_ascii_alphanumeric() || c2 == '_' {
                            chars.next();
                            name.push(c2);
                        } else {
                            break;
                        }
                    }
                    out.push_str(&self.resolve_script_var(&name, state));
                }
                _ => out.push('$'),
            }
        }

        out
    }

    fn resolve_script_var(&self, name: &str, state: &ScriptState) -> String {
        if let Some(value) = state.locals.get(name) {
            return value.clone();
        }

        if let Some((_, value)) = self
            .env_vars
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
        {
            if let Some(s) = value.as_string() {
                return s.to_string();
            }
        }

        String::new()
    }

    fn normalize_script_line(line: &str) -> String {
        line.trim_matches(|c: char| c == '\u{feff}' || c == '\u{fffe}' || c.is_whitespace())
            .to_string()
    }

    fn strip_quotes(s: &str) -> String {
        if s.len() >= 2 {
            let bytes = s.as_bytes();
            if (bytes[0] == b'"' && bytes[s.len() - 1] == b'"')
                || (bytes[0] == b'\'' && bytes[s.len() - 1] == b'\'')
            {
                return s[1..s.len() - 1].to_string();
            }
        }
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_shell_creation() {
        let shell = Shell::new(false);
        assert!(shell.is_ok());
        let shell = shell.unwrap();
        assert_eq!(shell.current_dir, std::env::current_dir().unwrap());
    }

    #[test]
    fn test_get_env_var() {
        let shell = Shell::new(false).unwrap();
        let value = shell.get_env_var("USERNAME", "default");
        assert!(value != "default" || std::env::var("USERNAME").is_err());
    }

    #[test]
    fn test_and_short_circuit_on_failure() {
        let mut shell = Shell::new(false).unwrap();
        shell.env_vars.remove("SHOULD_NOT_RUN");
        shell
            .execute_command("notexistcmd && set SHOULD_NOT_RUN=1")
            .unwrap();
        assert!(shell.env_vars.get("SHOULD_NOT_RUN").is_none());
        assert_ne!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_or_runs_on_failure() {
        let mut shell = Shell::new(false).unwrap();
        shell.env_vars.remove("SHOULD_RUN");
        shell
            .execute_command("notexistcmd || set SHOULD_RUN=1")
            .unwrap();
        assert_eq!(
            shell.env_vars.get("SHOULD_RUN"),
            Some(&ArrayValue::String("1".to_string()))
        );
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_or_short_circuit_on_success() {
        let mut shell = Shell::new(false).unwrap();
        shell.env_vars.remove("SHOULD_NOT_RUN_OR");
        shell
            .execute_command("echo hi || set SHOULD_NOT_RUN_OR=1")
            .unwrap();
        assert!(shell.env_vars.get("SHOULD_NOT_RUN_OR").is_none());
        assert_eq!(shell.last_exit_code, 0);
    }

    #[test]
    fn test_builtin_echo_redirection() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir = std::env::temp_dir().join(format!("winuxsh_redir_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        let out_path = test_dir.join("out.txt");
        let cmd = format!("echo hello > {}", out_path.to_string_lossy());
        shell.execute_command(&cmd).unwrap();
        let out = fs::read_to_string(out_path).unwrap();
        assert_eq!(out, "hello\n");
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_script_control_flow_and_positional_args() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir = std::env::temp_dir().join(format!("winuxsh_script_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();

        let script_path = test_dir.join("verify.sh");
        let out_path = test_dir.join("out.txt");
        let script = "\
#!/bin/bash
while true; do
  echo loop
  break
done
RUN_CMD=echo
case x in
  x) echo ok ;;
  *) echo bad ;;
esac
echo first=$1
shift
echo second=$1
echo redirected > __OUT_PATH__
";
        let script = script.replace("__OUT_PATH__", &out_path.to_string_lossy());
        fs::write(&script_path, script).unwrap();

        let args = vec!["A".to_string(), "B".to_string()];
        shell.run_script_file(&script_path, &args).unwrap();

        let out = fs::read_to_string(out_path).unwrap();
        assert_eq!(out, "redirected\n");
        assert_eq!(
            shell.env_vars.get("RUN_CMD"),
            Some(&ArrayValue::String("echo".to_string()))
        );
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_builtin_stderr_redirection_file_materialized() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir = std::env::temp_dir().join(format!("winuxsh_stderr_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        let err_path = test_dir.join("err.txt");
        let cmd = format!("echo hello 2> {}", err_path.to_string_lossy());
        shell.execute_command(&cmd).unwrap();
        let err = fs::read_to_string(err_path).unwrap();
        assert_eq!(err, "");
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_builtin_stdout_to_stderr_file() {
        let mut shell = Shell::new(false).unwrap();
        let test_dir =
            std::env::temp_dir().join(format!("winuxsh_stdout_to_stderr_{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();
        let err_path = test_dir.join("err.txt");
        let cmd = format!("echo redirected 1>&2 2> {}", err_path.to_string_lossy());
        shell.execute_command(&cmd).unwrap();
        let err = fs::read_to_string(err_path).unwrap();
        assert_eq!(err, "redirected\n");
        let _ = fs::remove_dir_all(test_dir);
    }
}
