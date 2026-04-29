//! Shell state management.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::ShellConfig;
use crate::env::Env;
use crate::value::Value;

/// The main shell state.
///
/// This contains all the state needed for the shell to operate,
/// including environment variables, functions, aliases, jobs, etc.
#[derive(Debug)]
pub struct ShellState {
    /// Environment variables
    pub env: Env,

    /// Shell configuration
    pub config: ShellConfig,

    /// Function definitions
    functions: HashMap<String, FunctionDef>,

    /// Aliases
    aliases: HashMap<String, String>,

    /// Current working directory
    current_dir: PathBuf,

    /// Exit code of the last command
    exit_code: i32,

    /// Exit codes of the last pipeline
    pipe_status: Vec<i32>,

    /// Shell options
    options: ShellOptions,

    /// Positional parameters ($1, $2, ...)
    positional_args: Vec<String>,

    /// Shell PID
    shell_pid: u32,

    /// PID of the last background command
    last_bg_pid: u32,

    /// Directory stack
    directory_stack: Vec<PathBuf>,

    /// Command hash table (command name -> full path)
    command_hash: HashMap<String, PathBuf>,

    /// Read-only variables
    readonly_vars: HashMap<String, Value>,

    /// Shell level (for nested shells)
    shell_level: u32,
}

/// A function definition.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    /// The function name
    pub name: String,
    /// The function body (as AST)
    pub body: Vec<winsh_ast::Stmt>,
    /// Whether the function is autoloaded
    pub autoload: bool,
    /// Source file where the function was defined
    pub source: Option<PathBuf>,
}

/// Shell options that control behavior.
#[derive(Debug, Clone)]
pub struct ShellOptions {
    /// Exit on error (-e / errexit)
    pub errexit: bool,
    /// Treat unset variables as errors (-u / nounset)
    pub nounset: bool,
    /// Enable globbing (-f / noglob)
    pub noglob: bool,
    /// Enable extended globbing (EXTENDED_GLOB)
    pub extended_glob: bool,
    /// Null glob - remove patterns that don't match (NULL_GLOB)
    pub null_glob: bool,
    /// Include dotfiles in globbing (GLOB_DOTS)
    pub glob_dots: bool,
    /// Case-insensitive globbing (CASE_GLOB)
    pub case_glob: bool,
    /// Ignore duplicate history entries (HIST_IGNORE_DUPS)
    pub hist_ignore_dups: bool,
    /// Ignore all duplicate history entries (HIST_IGNORE_ALL_DUPS)
    pub hist_ignore_all_dups: bool,
    /// Ignore entries starting with space (HIST_IGNORE_SPACE)
    pub hist_ignore_space: bool,
    /// Don't save duplicate entries to file (HIST_SAVE_NO_DUPS)
    pub hist_save_no_dups: bool,
    /// Verify history expansion before execution (HIST_VERIFY)
    pub hist_verify: bool,
    /// Enable prompt substitution (PROMPT_SUBST)
    pub prompt_subst: bool,
    /// Enable prompt percent sequences (PROMPT_PERCENT)
    pub prompt_percent: bool,
    /// Enable brace expansion (BRACE_EXPAND)
    pub brace_expand: bool,
    /// Enable tilde expansion (TILDE_EXPAND)
    pub tilde_expand: bool,
    /// Enable variable expansion (VARIABLE_EXPAND)
    pub variable_expand: bool,
    /// Enable command substitution (COMMAND_SUBST)
    pub command_subst: bool,
    /// Enable arithmetic expansion (ARITH_EXPAND)
    pub arith_expand: bool,
    /// Print commands before execution (xtrace / -x)
    pub xtrace: bool,
    /// Verbose mode (verbose / -v)
    pub verbose: bool,
    /// Monitor mode for job control (MONITOR / -m)
    pub monitor: bool,
    /// Notify of job completion immediately (NOTIFY)
    pub notify: bool,
    /// Vi mode for line editing
    pub vi_mode: bool,
    /// Emacs mode for line editing (default)
    pub emacs_mode: bool,
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            errexit: false,
            nounset: false,
            noglob: false,
            extended_glob: false,
            null_glob: false,
            glob_dots: false,
            case_glob: false,
            hist_ignore_dups: true,
            hist_ignore_all_dups: false,
            hist_ignore_space: true,
            hist_save_no_dups: false,
            hist_verify: false,
            prompt_subst: true,
            prompt_percent: true,
            brace_expand: true,
            tilde_expand: true,
            variable_expand: true,
            command_subst: true,
            arith_expand: true,
            xtrace: false,
            verbose: false,
            monitor: true,
            notify: false,
            vi_mode: false,
            emacs_mode: true,
        }
    }
}

impl ShellState {
    /// Create a new shell state with default settings.
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let pid = std::process::id();

        Self {
            env: Env::from_process(),
            config: ShellConfig::default(),
            functions: HashMap::new(),
            aliases: HashMap::new(),
            current_dir,
            exit_code: 0,
            pipe_status: Vec::new(),
            options: ShellOptions::default(),
            positional_args: Vec::new(),
            shell_pid: pid,
            last_bg_pid: 0,
            directory_stack: Vec::new(),
            command_hash: HashMap::new(),
            readonly_vars: HashMap::new(),
            shell_level: 0,
        }
    }

    /// Get the current working directory.
    pub fn current_dir(&self) -> &PathBuf {
        &self.current_dir
    }

    /// Set the current working directory.
    pub fn set_current_dir(&mut self, path: PathBuf) {
        let old_dir = self.current_dir.clone();
        self.current_dir = path.clone();
        self.env.set_previous_dir(&old_dir.to_string_lossy());
        self.env.set_current_dir(&path.to_string_lossy());
    }

    /// Get the exit code of the last command.
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// Set the exit code.
    pub fn set_exit_code(&mut self, code: i32) {
        self.exit_code = code;
        self.env.set("?", code.to_string());
    }

    /// Get the pipe status.
    pub fn pipe_status(&self) -> &[i32] {
        &self.pipe_status
    }

    /// Set the pipe status.
    pub fn set_pipe_status(&mut self, status: Vec<i32>) {
        self.pipe_status = status.clone();
        let status_str: Vec<String> = status.iter().map(|s| s.to_string()).collect();
        self.env.set("PIPESTATUS", status_str.join(" "));
    }

    /// Get the shell options.
    pub fn options(&self) -> &ShellOptions {
        &self.options
    }

    /// Get a mutable reference to the shell options.
    pub fn options_mut(&mut self) -> &mut ShellOptions {
        &mut self.options
    }

    /// Define a function.
    pub fn define_function(&mut self, name: String, body: Vec<winsh_ast::Stmt>) {
        self.functions.insert(
            name.clone(),
            FunctionDef {
                name,
                body,
                autoload: false,
                source: None,
            },
        );
    }

    /// Get a function definition.
    pub fn get_function(&self, name: &str) -> Option<&FunctionDef> {
        self.functions.get(name)
    }

    /// Check if a function exists.
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Remove a function.
    pub fn remove_function(&mut self, name: &str) {
        self.functions.remove(name);
    }

    /// Get all function names.
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }

    /// Set an alias.
    pub fn set_alias(&mut self, name: String, value: String) {
        self.aliases.insert(name, value);
    }

    /// Get an alias.
    pub fn get_alias(&self, name: &str) -> Option<&str> {
        self.aliases.get(name).map(|s| s.as_str())
    }

    /// Check if an alias exists.
    pub fn has_alias(&self, name: &str) -> bool {
        self.aliases.contains_key(name)
    }

    /// Remove an alias.
    pub fn remove_alias(&mut self, name: &str) {
        self.aliases.remove(name);
    }

    /// Get all aliases.
    pub fn aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    /// Set a positional argument.
    pub fn set_positional_args(&mut self, args: Vec<String>) {
        self.positional_args = args;
    }

    /// Get a positional argument ($1, $2, etc.).
    pub fn positional_arg(&self, index: usize) -> Option<&str> {
        self.positional_args.get(index - 1).map(|s| s.as_str())
    }

    /// Get all positional arguments.
    pub fn positional_args(&self) -> &[String] {
        &self.positional_args
    }

    /// Get the number of positional arguments.
    pub fn positional_arg_count(&self) -> usize {
        self.positional_args.len()
    }

    /// Get the shell PID.
    pub fn shell_pid(&self) -> u32 {
        self.shell_pid
    }

    /// Get the last background PID.
    pub fn last_bg_pid(&self) -> u32 {
        self.last_bg_pid
    }

    /// Set the last background PID.
    pub fn set_last_bg_pid(&mut self, pid: u32) {
        self.last_bg_pid = pid;
        self.env.set("!", pid.to_string());
    }

    /// Push a directory onto the directory stack.
    pub fn pushd(&mut self, path: PathBuf) {
        self.directory_stack.push(path);
    }

    /// Pop a directory from the directory stack.
    pub fn popd(&mut self) -> Option<PathBuf> {
        self.directory_stack.pop()
    }

    /// Get the directory stack.
    pub fn directory_stack(&self) -> &[PathBuf] {
        &self.directory_stack
    }

    /// Add a command to the hash table.
    pub fn hash_command(&mut self, name: String, path: PathBuf) {
        self.command_hash.insert(name, path);
    }

    /// Get a command from the hash table.
    pub fn get_hashed_command(&self, name: &str) -> Option<&PathBuf> {
        self.command_hash.get(name)
    }

    /// Clear the command hash table.
    pub fn clear_command_hash(&mut self) {
        self.command_hash.clear();
    }

    /// Get the shell level.
    pub fn shell_level(&self) -> u32 {
        self.shell_level
    }

    /// Increment the shell level.
    pub fn increment_shell_level(&mut self) {
        self.shell_level += 1;
        self.env.set("SHLVL", self.shell_level.to_string());
    }

    /// Check if a variable is read-only.
    pub fn is_readonly(&self, name: &str) -> bool {
        self.readonly_vars.contains_key(name)
    }

    /// Make a variable read-only.
    pub fn set_readonly(&mut self, name: String, value: Value) {
        self.readonly_vars.insert(name, value);
    }

    /// Get the prompt string.
    pub fn prompt(&self) -> String {
        if self.options.prompt_subst {
            // TODO: Implement prompt substitution
            self.config.prompt.clone()
        } else {
            self.config.prompt.clone()
        }
    }

    /// Get the right prompt string.
    pub fn rprompt(&self) -> String {
        self.config.rprompt.clone()
    }

    /// Get the continuation prompt string.
    pub fn ps2(&self) -> String {
        self.config.ps2.clone()
    }
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_state_new() {
        let state = ShellState::new();
        assert_eq!(state.exit_code(), 0);
        assert!(state.positional_args().is_empty());
    }

    #[test]
    fn test_shell_state_exit_code() {
        let mut state = ShellState::new();
        state.set_exit_code(42);
        assert_eq!(state.exit_code(), 42);
    }

    #[test]
    fn test_shell_state_functions() {
        let mut state = ShellState::new();
        state.define_function("greet".to_string(), vec![]);
        assert!(state.has_function("greet"));
        assert!(!state.has_function("other"));
    }

    #[test]
    fn test_shell_state_aliases() {
        let mut state = ShellState::new();
        state.set_alias("ll".to_string(), "ls -la".to_string());
        assert_eq!(state.get_alias("ll"), Some("ls -la"));
        assert!(state.has_alias("ll"));
    }

    #[test]
    fn test_shell_state_positional_args() {
        let mut state = ShellState::new();
        state.set_positional_args(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(state.positional_arg(1), Some("a"));
        assert_eq!(state.positional_arg(2), Some("b"));
        assert_eq!(state.positional_arg(3), Some("c"));
        assert_eq!(state.positional_arg(4), None);
        assert_eq!(state.positional_arg_count(), 3);
    }

    #[test]
    fn test_shell_state_directory_stack() {
        let mut state = ShellState::new();
        state.pushd(PathBuf::from("/tmp"));
        state.pushd(PathBuf::from("/home"));
        assert_eq!(state.directory_stack().len(), 2);
        assert_eq!(state.popd(), Some(PathBuf::from("/home")));
        assert_eq!(state.directory_stack().len(), 1);
    }

    #[test]
    fn test_shell_state_command_hash() {
        let mut state = ShellState::new();
        state.hash_command("ls".to_string(), PathBuf::from("/usr/bin/ls"));
        assert_eq!(
            state.get_hashed_command("ls"),
            Some(&PathBuf::from("/usr/bin/ls"))
        );
    }

    #[test]
    fn test_shell_options_default() {
        let options = ShellOptions::default();
        assert!(!options.errexit);
        assert!(!options.nounset);
        assert!(options.prompt_subst);
        assert!(options.emacs_mode);
    }
}
