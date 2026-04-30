//! The main executor implementation with pipeline support and job control.

use std::process::{Command, Stdio, Child, ChildStdin, ChildStdout};
use std::io::{self, Read, Write};
use std::path::PathBuf;

use winsh_ast::Stmt;
use winsh_ast::word::{Word, WordPart};
use winsh_core::{ShellError, ShellState, expand_variable};
use crate::job::{JobManager, JobStatus};

/// The command executor.
pub struct Executor {
    /// Job manager for background processes
    jobs: JobManager,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            jobs: JobManager::new(),
        }
    }

    pub fn execute(&mut self, stmts: &[Stmt], state: &mut ShellState) -> Result<i32, ShellError> {
        let mut exit_code = 0;

        for stmt in stmts {
            exit_code = self.execute_statement(stmt, state)?;
            state.set_exit_code(exit_code);

            if state.options().errexit && exit_code != 0 {
                return Err(ShellError::exit(exit_code));
            }
        }

        Ok(exit_code)
    }

    /// Expand a word by resolving variables.
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
                    let (name, modifier) = parse_braced_variable(spec);
                    let value = expand_variable(&name, modifier.as_deref(), &state.env)?;
                    result.push_str(&value);
                }
                WordPart::Escaped(c) => result.push(*c),
                _ => result.push_str(&part.to_string()),
            }
        }

        Ok(result)
    }

    pub fn execute_statement(&mut self, stmt: &Stmt, state: &mut ShellState) -> Result<i32, ShellError> {
        match stmt {
            Stmt::Command { words, redirections, background } => {
                if words.is_empty() {
                    return Ok(0);
                }

                let cmd_name = self.expand_word(&words[0], state)?;
                let mut args = Vec::new();
                for word in &words[1..] {
                    args.push(self.expand_word(word, state)?);
                }
                let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

                if *background {
                    self.execute_background(&cmd_name, &arg_refs, state)
                } else {
                    self.execute_foreground(&cmd_name, &arg_refs, state)
                }
            }
            Stmt::Pipeline { commands, negated } => {
                self.execute_pipeline(commands, *negated, state)
            }
            Stmt::And { left, right } => {
                let code = self.execute_statement(left, state)?;
                if code == 0 { self.execute_statement(right, state) } else { Ok(code) }
            }
            Stmt::Or { left, right } => {
                let code = self.execute_statement(left, state)?;
                if code != 0 { self.execute_statement(right, state) } else { Ok(code) }
            }
            Stmt::Sequence(stmts) => {
                let mut code = 0;
                for s in stmts { code = self.execute_statement(s, state)?; }
                Ok(code)
            }
            Stmt::Subshell(stmt) => self.execute_statement(stmt, state),
            Stmt::Group(stmt) => self.execute_statement(stmt, state),
            Stmt::If { condition, then_branch, elif_branches, else_branch } => {
                let cond_code = self.execute_statement(condition, state)?;
                if cond_code == 0 {
                    self.execute_statements(then_branch, state)
                } else {
                    for (elif_cond, elif_body) in elif_branches {
                        if self.execute_statement(elif_cond, state)? == 0 {
                            return self.execute_statements(elif_body, state);
                        }
                    }
                    if let Some(else_body) = else_branch {
                        self.execute_statements(else_body, state)
                    } else { Ok(0) }
                }
            }
            Stmt::For { var, words, body } => {
                let mut code = 0;
                for word in words {
                    state.env.set(var, &word.to_string());
                    code = self.execute_statements(body, state)?;
                }
                Ok(code)
            }
            Stmt::While { condition, body } => {
                let mut code = 0;
                while self.execute_statement(condition, state)? == 0 {
                    code = self.execute_statements(body, state)?;
                }
                Ok(code)
            }
            Stmt::Until { condition, body } => {
                let mut code = 0;
                while self.execute_statement(condition, state)? != 0 {
                    code = self.execute_statements(body, state)?;
                }
                Ok(code)
            }
            Stmt::FunctionDef { name, body } => {
                state.define_function(name.clone(), body.clone());
                Ok(0)
            }
            Stmt::Assign { name, value, .. } => {
                let val = self.expand_word(value, state)?;
                state.env.set(name.clone(), &val);
                Ok(0)
            }
            Stmt::Empty => Ok(0),
            _ => Ok(0),
        }
    }

    fn execute_statements(&mut self, stmts: &[Stmt], state: &mut ShellState) -> Result<i32, ShellError> {
        let mut code = 0;
        for s in stmts { code = self.execute_statement(s, state)?; }
        Ok(code)
    }

    /// Execute a command in the foreground.
    fn execute_foreground(
        &mut self,
        cmd: &str,
        args: &[&str],
        state: &mut ShellState,
    ) -> Result<i32, ShellError> {
        // Check builtins first
        if let Some(code) = self.execute_builtin(cmd, args, state)? {
            return Ok(code);
        }

        // Check functions
        if state.has_function(cmd) {
            return self.execute_function(cmd, state);
        }

        let cmd_path = self.find_command(cmd, state)?;
        let mut command = self.build_command(&cmd_path, args, state);
        command.stdin(Stdio::inherit());
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        let mut child = command.spawn()
            .map_err(|e| ShellError::command_not_found(format!("{}: {}", cmd, e)))?;

        let status = child.wait().map_err(|e| ShellError::Io(e))?;
        Ok(status.code().unwrap_or(1))
    }

    /// Execute a command in the background.
    fn execute_background(
        &mut self,
        cmd: &str,
        args: &[&str],
        state: &mut ShellState,
    ) -> Result<i32, ShellError> {
        let cmd_path = self.find_command(cmd, state)?;
        let mut command = self.build_command(&cmd_path, args, state);
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        let child = command.spawn()
            .map_err(|e| ShellError::command_not_found(format!("{}: {}", cmd, e)))?;

        let pid = child.id();
        let full_cmd = std::iter::once(cmd).chain(args.iter().copied()).collect::<Vec<_>>().join(" ");
        let id = self.jobs.add(pid, &full_cmd, Some(child));
        state.set_last_bg_pid(pid);

        eprintln!("[{}] {}", id, pid);
        Ok(0)
    }

    /// Execute a pipeline with real pipe connections.
    pub fn execute_pipeline(
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

        let mut children: Vec<Child> = Vec::new();
        let mut pipe_status: Vec<i32> = Vec::new();
        let mut last_stdout: Option<ChildStdout> = None;

        for (i, cmd_stmt) in commands.iter().enumerate() {
            let is_last = i == commands.len() - 1;

            let words = match cmd_stmt {
                Stmt::Command { words, .. } => words,
                _ => return Err(ShellError::message("pipeline element must be a command")),
            };

            if words.is_empty() {
                continue;
            }

            let cmd_name = self.expand_word(&words[0], state)?;

            // Check for builtins in pipeline
            if self.is_builtin(&cmd_name) {
                // For builtins, execute them in-process with piped IO
                let mut args = Vec::new();
                for word in &words[1..] {
                    args.push(self.expand_word(word, state)?);
                }

                let input = if let Some(mut stdout) = last_stdout.take() {
                    let mut buf = String::new();
                    stdout.read_to_string(&mut buf).ok();
                    Some(buf)
                } else {
                    None
                };

                if let Some(input) = input {
                    // Feed input to the builtin and capture output
                    let output = self.execute_builtin_with_pipes(&cmd_name, &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), &input, state);
                    if is_last {
                        return output;
                    }
                    // TODO: pipe builtin output to next command
                    pipe_status.push(0);
                } else {
                    let code = self.execute_builtin(&cmd_name, &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), state)?;
                    if let Some(code) = code {
                        pipe_status.push(code);
                    }
                }
                continue;
            }

            let cmd_path = self.find_command(&cmd_name, state)?;
            let args: Vec<&str> = words[1..].iter()
                .filter_map(|w| {
                    self.expand_word(w, state).ok().map(|s| Box::leak(s.into_boxed_str()) as &str)
                })
                .collect();

            let mut command = self.build_command(&cmd_path, &args, state);

            // Configure pipe connections
            if let Some(stdout) = last_stdout.take() {
                command.stdin(Stdio::from(stdout));
            } else {
                command.stdin(Stdio::inherit());
            }

            if is_last {
                command.stdout(Stdio::inherit());
            } else {
                command.stdout(Stdio::piped());
            }

            command.stderr(Stdio::inherit());

            let mut child = command.spawn()
                .map_err(|e| ShellError::command_not_found(format!("{}: {}", cmd_name, e)))?;

            if !is_last {
                last_stdout = child.stdout.take();
            }

            children.push(child);
        }

        // Wait for all children and collect exit codes
        let mut last_exit = 0;
        for mut child in children {
            match child.wait() {
                Ok(status) => {
                    let code = status.code().unwrap_or(1);
                    pipe_status.push(code);
                    last_exit = code;
                }
                Err(_) => {
                    pipe_status.push(1);
                    last_exit = 1;
                }
            }
        }

        state.set_pipe_status(pipe_status);

        if negated {
            Ok((last_exit == 0) as i32)
        } else {
            Ok(last_exit)
        }
    }

    /// Build a Command with environment and working directory.
    fn build_command(&self, cmd_path: &PathBuf, args: &[&str], state: &ShellState) -> Command {
        let mut command;

        // Handle special file types on Windows
        #[cfg(windows)]
        {
            let ext = cmd_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            match ext.to_lowercase().as_str() {
                "cmd" | "bat" => {
                    // Run .cmd/.bat through cmd.exe
                    command = Command::new("cmd.exe");
                    command.arg("/C");
                    command.arg(cmd_path);
                }
                "ps1" => {
                    // Run .ps1 through powershell.exe
                    command = Command::new("powershell.exe");
                    command.args(["-ExecutionPolicy", "Bypass", "-File"]);
                    command.arg(cmd_path);
                }
                _ => {
                    command = Command::new(cmd_path);
                }
            }
        }

        #[cfg(not(windows))]
        {
            command = Command::new(cmd_path);
        }

        command.args(args);

        for (key, value) in state.env.exported() {
            command.env(key, value);
        }
        command.current_dir(state.current_dir());

        // Prevent creating new windows on Windows
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        command
    }

    /// Check if a command is a builtin.
    fn is_builtin(&self, cmd: &str) -> bool {
        matches!(cmd, "echo" | "cd" | "pwd" | "exit" | "set" | "unset" | "export" |
            "alias" | "unalias" | "type" | "history" | "jobs" | "fg" | "bg" |
            "kill" | "wait" | "disown" | "true" | "false" | "source" | "." |
            "printf" | "read" | "test" | "which" | "dirs" | "pushd" | "popd" |
            "eval" | "exec" | "help")
    }

    /// Execute a built-in command.
    fn execute_builtin(
        &mut self,
        cmd: &str,
        args: &[&str],
        state: &mut ShellState,
    ) -> Result<Option<i32>, ShellError> {
        match cmd {
            "echo" => {
                let mut no_newline = false;
                let mut start = 0;
                for (i, arg) in args.iter().enumerate() {
                    match *arg {
                        "-n" => { no_newline = true; start = i + 1; }
                        "-e" => { start = i + 1; }
                        "-E" => { start = i + 1; }
                        _ => break,
                    }
                }
                let output = args[start..].join(" ");
                if no_newline { print!("{}", output); } else { println!("{}", output); }
                Ok(Some(0))
            }
            "exit" => {
                let code = args.first().and_then(|s| s.parse().ok()).unwrap_or(state.exit_code());
                Err(ShellError::exit(code))
            }
            "cd" => {
                let path = args.first().map(PathBuf::from)
                    .unwrap_or_else(|| state.env.home_dir().map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")));
                let abs_path = if path.is_absolute() { path.clone() } else { state.current_dir().join(&path) };
                if abs_path.exists() && abs_path.is_dir() {
                    state.set_current_dir(abs_path);
                    Ok(Some(0))
                } else {
                    eprintln!("cd: {}: No such file or directory", path.display());
                    Ok(Some(1))
                }
            }
            "pwd" => { println!("{}", state.current_dir().display()); Ok(Some(0)) }
            "jobs" => {
                let jobs = self.jobs.list();
                if jobs.is_empty() {
                    return Ok(Some(0));
                }
                for job in &jobs {
                    let marker = if Some(job.id) == self.jobs.current_id() { "+" }
                        else if Some(job.id) == self.jobs.previous_job { "-" }
                        else { " " };
                    eprintln!("[{}]{}  {}       {}", job.id, marker, job.status, job.command);
                }
                Ok(Some(0))
            }
            "fg" => {
                let id = if args.is_empty() {
                    self.jobs.current_id().ok_or_else(|| ShellError::message("fg: no current job"))?
                } else {
                    self.jobs.parse_id(args[0]).map_err(|e| ShellError::message(e))?
                };
                Ok(Some(self.jobs.foreground(id).map_err(|e| ShellError::message(e))?))
            }
            "bg" => {
                let id = if args.is_empty() {
                    self.jobs.current_id().ok_or_else(|| ShellError::message("bg: no current job"))?
                } else {
                    self.jobs.parse_id(args[0]).map_err(|e| ShellError::message(e))?
                };
                self.jobs.background(id).map_err(|e| ShellError::message(e))?;
                if let Some(job) = self.jobs.get(id) {
                    eprintln!("[{}] {} &", id, job.command);
                }
                Ok(Some(0))
            }
            "kill" => {
                if args.is_empty() {
                    return Err(ShellError::message("kill: usage: kill [job_id]"));
                }
                let id = self.jobs.parse_id(args[0]).map_err(|e| ShellError::message(e))?;
                self.jobs.kill(id).map_err(|e| ShellError::message(e))?;
                Ok(Some(0))
            }
            "wait" => {
                let id = if args.is_empty() {
                    self.jobs.current_id().ok_or_else(|| ShellError::message("wait: no current job"))?
                } else {
                    self.jobs.parse_id(args[0]).map_err(|e| ShellError::message(e))?
                };
                Ok(Some(self.jobs.foreground(id).map_err(|e| ShellError::message(e))?))
            }
            "disown" => {
                let id = if args.is_empty() {
                    self.jobs.current_id().ok_or_else(|| ShellError::message("disown: no current job"))?
                } else {
                    self.jobs.parse_id(args[0]).map_err(|e| ShellError::message(e))?
                };
                if let Some(mut job) = self.jobs.get_mut(id) {
                    job.child.take(); // Detach the child so it's no longer tracked
                }
                self.jobs.remove(id);
                Ok(Some(0))
            }
            "history" => {
                // History is handled at the REPL level, just return success
                Ok(Some(0))
            }
            "true" => Ok(Some(0)),
            "false" => Ok(Some(1)),
            "type" => {
                if args.is_empty() {
                    eprintln!("type: missing argument");
                    return Ok(Some(1));
                }
                let cmd = args[0];
                if self.is_builtin(cmd) {
                    println!("{} is a shell builtin", cmd);
                } else if state.has_function(cmd) {
                    println!("{} is a function", cmd);
                } else if state.has_alias(cmd) {
                    println!("{} is aliased to '{}'", cmd, state.get_alias(cmd).unwrap_or(""));
                } else {
                    match self.find_command(cmd, state) {
                        Ok(path) => println!("{} is {}", cmd, path.display()),
                        Err(_) => println!("{}: not found", cmd),
                    }
                }
                Ok(Some(0))
            }
            "which" => {
                if args.is_empty() {
                    return Ok(Some(1));
                }
                for cmd in args {
                    if self.is_builtin(cmd) {
                        println!("{}: shell built-in command", cmd);
                    } else {
                        match self.find_command(cmd, state) {
                            Ok(path) => println!("{}", path.display()),
                            Err(_) => {
                                eprintln!("which: no {} in PATH", cmd);
                                return Ok(Some(1));
                            }
                        }
                    }
                }
                Ok(Some(0))
            }
            "help" => {
                println!("WinSH built-in commands:");
                println!("  cd       echo    exit     pwd     type    which");
                println!("  jobs     fg      bg       kill    wait    disown");
                println!("  alias    export  unset    source  history help");
                println!("  true     false   printf   read    test");
                Ok(Some(0))
            }
            _ => Ok(None),
        }
    }

    /// Execute a builtin with piped input and capture output.
    fn execute_builtin_with_pipes(
        &mut self,
        cmd: &str,
        args: &[&str],
        input: &str,
        state: &mut ShellState,
    ) -> Result<i32, ShellError> {
        match cmd {
            "echo" => {
                let output = args.join(" ");
                println!("{}", output);
                Ok(0)
            }
            _ => {
                // Most builtins don't support piped I/O yet
                Ok(0)
            }
        }
    }

    /// Execute a function.
    fn execute_function(&mut self, name: &str, state: &mut ShellState) -> Result<i32, ShellError> {
        if let Some(func) = state.get_function(name) {
            let body = func.body.clone();
            self.execute(&body, state)
        } else {
            Err(ShellError::command_not_found(name))
        }
    }

    /// Find a command in PATH.
    fn find_command(&self, cmd: &str, state: &ShellState) -> Result<PathBuf, ShellError> {
        if let Some(path) = state.get_hashed_command(cmd) {
            if path.exists() { return Ok(path.clone()); }
        }

        let path = PathBuf::from(cmd);
        if path.is_absolute() || cmd.contains('/') || cmd.contains('\\') {
            if path.exists() { return Ok(path); }
            return Err(ShellError::command_not_found(cmd));
        }

        for dir in state.env.path_dirs() {
            let full_path = PathBuf::from(&dir).join(cmd);

            for ext in &["", ".exe", ".cmd", ".bat", ".ps1"] {
                let with_ext = if ext.is_empty() { full_path.clone() }
                    else { full_path.with_extension(&ext[1..]) };
                if with_ext.exists() { return Ok(with_ext); }
            }
        }
        Err(ShellError::command_not_found(cmd))
    }
}

fn parse_braced_variable(spec: &str) -> (String, Option<String>) {
    if spec.starts_with('#') {
        return (spec[1..].to_string(), Some("#".to_string()));
    }
    for (i, c) in spec.chars().enumerate() {
        match c {
            ':' | '#' | '%' | '/' => return (spec[..i].to_string(), Some(spec[i..].to_string())),
            _ => {}
        }
    }
    (spec.to_string(), None)
}

impl Default for Executor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_new() {
        let executor = Executor::new();
        assert_eq!(executor.jobs.count(), 0);
    }

    #[test]
    fn test_is_builtin() {
        let executor = Executor::new();
        assert!(executor.is_builtin("echo"));
        assert!(executor.is_builtin("cd"));
        assert!(executor.is_builtin("jobs"));
        assert!(!executor.is_builtin("ls"));
    }

    #[test]
    fn test_builtin_exit() {
        let mut executor = Executor::new();
        let mut state = ShellState::new();
        let result = executor.execute_builtin("exit", &["42"], &mut state);
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_true_false() {
        let mut executor = Executor::new();
        let mut state = ShellState::new();
        assert_eq!(executor.execute_builtin("true", &[], &mut state).unwrap(), Some(0));
        assert_eq!(executor.execute_builtin("false", &[], &mut state).unwrap(), Some(1));
    }
}
