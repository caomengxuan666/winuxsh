//! # WinSH Executor
//!
//! Command execution engine for the WinSH shell.

pub mod executor;
pub mod pipeline;
pub mod redir;

pub use executor::Executor;
