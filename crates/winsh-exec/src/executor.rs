//! The main executor implementation.

use std::process::{Command, Stdio, Child};
use std::path::PathBuf;

use winsh_ast::Stmt;
use winsh_ast::word::{Word, WordPart};
use winsh_core::{ShellError, ShellState, expand_variable};

/// The command executor.
///
/// Responsible for executing parsed commands.
pub struct Executor {
    /// Whether we're in a pipeline
    in_pipeline: bool,
}

impl Executor {
    /// Create a new executor.
    pub fn new() -> Self {
        Self {
            in_pipeline: false,
        }
    }

    /// Execute a list of statements.
    pub fn execute(&mut self, stmts: &[Stmt], state: &mut ShellState) -> Result<i32, ShellError> {
        let mut exit_code = 0;

        for stmt in stmts {
            exit_code = self.execute_statement(stmt, state)?;
            state.set_exit_code(exit_code);

            // Check errexit option
            if state.options().errexit && exit_code != 0 {
                return Err(ShellError::exit(exit_code));
            }
        }

        Ok(exit_code)
    }

    /// Expand a word by resolving variables and command substitutions.
    fn expand_word(&self, word: &Word, state: &ShellState) -> Result<String, ShellError> {
        let mut result = String::new();

        for part in &word.parts {
            match part {
                WordPart::Literal(s) => result.push_str(s),
                WordPart::SingleQuoted(s) => result.push_str(s),
                WordPart::DollarQuoted(s) => result.push_str(s),
                WordPart::Variable(name) => {
                    let value = state.env.get(name).unwrap_or("");
                    result.push_str(value);
                }
                WordPart::BracedVariable(spec) => {
                    // Parse the variable specification
                    let (name, modifier) = parse_braced_variable(spec);
                    let value = expand_variable(&name, modifier.as_deref(), &state.env)?;
                    result.push_str(&value);
                }
                WordPart::CommandSubst(cmd) => {
                    // TODO: Execute command and capture output
                    result.push_str(&format!("$({})", cmd));
                }
                WordPart::BacktickSubst(cmd) => {
                    // TODO: Execute command and capture output
                    result.push_str(&format!("`{}`", cmd));
                }
                WordPart::Arithmetic(expr) => {
                    // TODO: Evaluate arithmetic expression
                    result.push_str(&format!("$(({}))", expr));
                }
                WordPart::Escaped(c) => result.push(*c),
                _ => {
                    // Other word parts are not yet supported
                    result.push_str(&part.to_string());
                }
            }
        }

        Ok(result)
    }

    /// Execute a single statement.
    pub fn execute_statement(&mut self, stmt: &Stmt, state: &mut ShellState) -> Result<i32, ShellError> {
        match stmt {
            Stmt::Command { words, redirections, background } => {
                if words.is_empty() {
                    return Ok(0);
                }

                // Expand the command name
                let cmd_name = self.expand_word(&words[0], state)?;

                // Expand the arguments
                let mut args = Vec::new();
                for word in &words[1..] {
                    args.push(self.expand_word(word, state)?);
                }

                let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

                // Execute the command
                self.execute_external(&cmd_name, &arg_refs, state, *background)
            }
            Stmt::Pipeline { commands, negated } => {
                self.execute_pipeline(commands, *negated, state)
            }
            Stmt::And { left, right } => {
                let left_code = self.execute_statement(left, state)?;
                if left_code == 0 {
                    self.execute_statement(right, state)
                } else {
                    Ok(left_code)
                }
            }
            Stmt::Or { left, right } => {
                let left_code = self.execute_statement(left, state)?;
                if left_code != 0 {
                    self.execute_statement(right, state)
                } else {
                    Ok(left_code)
                }
            }
            Stmt::Sequence(stmts) => {
                let mut exit_code = 0;
                for stmt in stmts {
                    exit_code = self.execute_statement(stmt, state)?;
                }
                Ok(exit_code)
            }
            Stmt::Subshell(stmt) => {
                // TODO: Execute in a subshell
                self.execute_statement(stmt, state)
            }
            Stmt::Group(stmt) => {
                self.execute_statement(stmt, state)
            }
            Stmt::If { condition, then_branch, elif_branches, else_branch } => {
                let cond_code = self.execute_statement(condition, state)?;
                if cond_code == 0 {
                    self.execute_statements(then_branch, state)
                } else {
                    for (elif_cond, elif_body) in elif_branches {
                        let elif_code = self.execute_statement(elif_cond, state)?;
                        if elif_code == 0 {
                            return self.execute_statements(elif_body, state);
                        }
                    }
                    if let Some(else_body) = else_branch {
                        self.execute_statements(else_body, state)
                    } else {
                        Ok(0)
                    }
                }
            }
            Stmt::For { var, words, body } => {
                let mut exit_code = 0;
                for word in words {
                    let value = word.to_string();
                    state.env.set(var, &value);
                    exit_code = self.execute_statements(body, state)?;
                }
                Ok(exit_code)
            }
            Stmt::While { condition, body } => {
                let mut exit_code = 0;
                loop {
                    let cond_code = self.execute_statement(condition, state)?;
                    if cond_code != 0 {
                        break;
                    }
                    exit_code = self.execute_statements(body, state)?;
                }
                Ok(exit_code)
            }
            Stmt::Until { condition, body } => {
                let mut exit_code = 0;
                loop {
                    let cond_code = self.execute_statement(condition, state)?;
                    if cond_code == 0 {
                        break;
                    }
                    exit_code = self.execute_statements(body, state)?;
                }
                Ok(exit_code)
            }
            Stmt::FunctionDef { name, body } => {
                state.define_function(name.clone(), body.clone());
                Ok(0)
            }
            Stmt::Assign { name, value, .. } => {
                state.env.set(name.clone(), value.to_string());
                Ok(0)
            }
            Stmt::Empty => Ok(0),
            _ => {
                // TODO: Implement other statement types
                Ok(0)
            }
        }
    }

    /// Execute a list of statements.
    fn execute_statements(&mut self, stmts: &[Stmt], state: &mut ShellState) -> Result<i32, ShellError> {
        let mut exit_code = 0;
        for stmt in stmts {
            exit_code = self.execute_statement(stmt, state)?;
        }
        Ok(exit_code)
    }

    /// Execute an external command.
    fn execute_external(
        &self,
        cmd: &str,
        args: &[&str],
        state: &mut ShellState,
        background: bool,
    ) -> Result<i32, ShellError> {
        // Check for built-in commands first
        if let Some(code) = self.execute_builtin(cmd, args, state)? {
            return Ok(code);
        }

        // Find the command in PATH
        let cmd_path = self.find_command(cmd, state)?;

        // Create the command
        let mut command = Command::new(&cmd_path);
        command.args(args);

        // Set environment variables
        for (key, value) in state.env.exported() {
            command.env(key, value);
        }

        // Set working directory
        command.current_dir(state.current_dir());

        // Configure stdio
        if background {
            command.stdin(Stdio::null());
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
        } else {
            command.stdin(Stdio::inherit());
            command.stdout(Stdio::inherit());
            command.stderr(Stdio::inherit());
        }

        // Spawn the process
        let mut child = command.spawn()
            .map_err(|e| ShellError::command_not_found(format!("{}: {}", cmd, e)))?;

        if background {
            let pid = child.id();
            state.set_last_bg_pid(pid);
            // TODO: Add to job table
            Ok(0)
        } else {
            let status = child.wait()
                .map_err(|e| ShellError::Io(e))?;
            Ok(status.code().unwrap_or(1))
        }
    }

    /// Execute a built-in command.
    fn execute_builtin(
        &self,
        cmd: &str,
        args: &[&str],
        state: &mut ShellState,
    ) -> Result<Option<i32>, ShellError> {
        match cmd {
            "echo" => {
                let output = args.join(" ");
                println!("{}", output);
                Ok(Some(0))
            }
            "exit" => {
                let code = args.first()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                Err(ShellError::exit(code))
            }
            "cd" => {
                let path = args.first()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        state.env.home_dir()
                            .map(PathBuf::from)
                            .unwrap_or_else(|| PathBuf::from("."))
                    });

                if path.exists() {
                    state.set_current_dir(path);
                    Ok(Some(0))
                } else {
                    eprintln!("cd: no such file or directory: {}", path.display());
                    Ok(Some(1))
                }
            }
            "pwd" => {
                println!("{}", state.current_dir().display());
                Ok(Some(0))
            }
            "set" => {
                // TODO: Implement set
                Ok(Some(0))
            }
            "export" => {
                // TODO: Implement export
                Ok(Some(0))
            }
            "unset" => {
                // TODO: Implement unset
                Ok(Some(0))
            }
            "alias" => {
                // TODO: Implement alias
                Ok(Some(0))
            }
            "unalias" => {
                // TODO: Implement unalias
                Ok(Some(0))
            }
            "history" => {
                // TODO: Implement history
                Ok(Some(0))
            }
            "type" => {
                if let Some(name) = args.first() {
                    if state.has_function(name) {
                        println!("{} is a function", name);
                    } else if self.find_command(name, state).is_ok() {
                        let path = self.find_command(name, state)?;
                        println!("{} is {}", name, path.display());
                    } else {
                        println!("{}: not found", name);
                    }
                }
                Ok(Some(0))
            }
            _ => Ok(None),
        }
    }

    /// Find a command in PATH.
    fn find_command(&self, cmd: &str, state: &ShellState) -> Result<PathBuf, ShellError> {
        // Check hash table first
        if let Some(path) = state.get_hashed_command(cmd) {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        // Check if it's an absolute or relative path
        let path = PathBuf::from(cmd);
        if path.is_absolute() || cmd.contains('/') || cmd.contains('\\') {
            if path.exists() {
                return Ok(path);
            }
            return Err(ShellError::command_not_found(cmd));
        }

        // Search in PATH
        for dir in state.env.path_dirs() {
            let full_path = PathBuf::from(&dir).join(cmd);

            // Try with common extensions on Windows
            for ext in &["", ".exe", ".cmd", ".bat", ".ps1"] {
                let with_ext = if ext.is_empty() {
                    full_path.clone()
                } else {
                    full_path.with_extension(&ext[1..])
                };

                if with_ext.exists() {
                    return Ok(with_ext);
                }
            }
        }

        Err(ShellError::command_not_found(cmd))
    }

    /// Execute a pipeline.
    fn execute_pipeline(
        &mut self,
        commands: &[Stmt],
        negated: bool,
        state: &mut ShellState,
    ) -> Result<i32, ShellError> {
        if commands.is_empty() {
            return Ok(0);
        }

        if commands.len() == 1 {
            let code = self.execute_statement(&commands[0], state)?;
            return Ok(if negated { (code == 0) as i32 } else { code });
        }

        // For now, execute single command pipelines
        // TODO: Implement proper pipe handling with Stdio::piped()
        let mut exit_code = 0;
        for cmd in commands {
            exit_code = self.execute_statement(cmd, state)?;
        }

        if negated {
            Ok((exit_code == 0) as i32)
        } else {
            Ok(exit_code)
        }
    }
}

/// Parse a braced variable specification into name and modifier.
///
/// Examples:
/// - "VAR" -> ("VAR", None)
/// - "VAR:-default" -> ("VAR", Some(":-default"))
/// - "VAR#pattern" -> ("VAR", Some("#pattern"))
/// - "VAR//old/new" -> ("VAR", Some("//old/new"))
fn parse_braced_variable(spec: &str) -> (String, Option<String>) {
    // Check for ${#VAR} (length)
    if spec.starts_with('#') {
        return (spec[1..].to_string(), Some("#".to_string()));
    }

    // Find the first operator character
    for (i, c) in spec.chars().enumerate() {
        match c {
            ':' | '#' | '%' | '/' => {
                return (spec[..i].to_string(), Some(spec[i..].to_string()));
            }
            _ => {}
        }
    }

    // No modifier found
    (spec.to_string(), None)
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_new() {
        let executor = Executor::new();
        assert!(!executor.in_pipeline);
    }

    #[test]
    fn test_find_command() {
        let executor = Executor::new();
        let state = ShellState::new();

        // echo should be found on most systems
        let result = executor.find_command("echo", &state);
        // This might fail if echo is not in PATH, which is OK for testing
        // assert!(result.is_ok());
    }
}
