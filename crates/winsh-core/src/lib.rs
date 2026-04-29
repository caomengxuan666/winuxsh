//! # WinSH Core
//!
//! Core types, state management, and error definitions for the WinSH shell.

pub mod error;
pub mod state;
pub mod value;
pub mod config;
pub mod env;
pub mod expansion;
pub mod arithmetic;
pub mod conditional;
pub mod heredoc;
pub mod glob;

pub use error::ShellError;
pub use state::ShellState;
pub use value::Value;
pub use config::ShellConfig;
pub use env::Env;
pub use expansion::expand_variable;
pub use arithmetic::eval_arithmetic;
pub use conditional::eval_conditional;
pub use heredoc::{HereDoc, read_heredoc, parse_heredocs};
pub use glob::{expand_globs, match_pattern, GlobOptions};
