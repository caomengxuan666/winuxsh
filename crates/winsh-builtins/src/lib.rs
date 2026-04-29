//! # WinSH Builtins
//!
//! Built-in commands for the WinSH shell.

pub mod core;

/// Trait for built-in commands.
pub trait Builtin {
    /// Get the name of the command.
    fn name(&self) -> &str;

    /// Execute the command.
    fn execute(&self, args: &[&str]) -> Result<i32, winsh_core::ShellError>;

    /// Get help text for the command.
    fn help(&self) -> &str;
}
