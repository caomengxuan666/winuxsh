//! Signal handling for the shell.
//!
//! Supports signal trapping and handling for various signals.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// Signal types supported by the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    /// Terminal interrupt (Ctrl+C)
    SIGINT,
    /// Terminal quit (Ctrl+\)
    SIGQUIT,
    /// Terminal suspend (Ctrl+Z)
    SIGTSTP,
    /// Hangup detected (terminal closed)
    SIGHUP,
    /// Termination signal
    SIGTERM,
    /// Kill signal (cannot be caught)
    SIGKILL,
    /// Stop signal (cannot be caught)
    SIGSTOP,
    /// Continue if stopped
    SIGCONT,
    /// Window size change
    SIGWINCH,
    /// User-defined signal 1
    SIGUSR1,
    /// User-defined signal 2
    SIGUSR2,
}

impl Signal {
    /// Get the signal name.
    pub fn name(&self) -> &'static str {
        match self {
            Signal::SIGINT => "SIGINT",
            Signal::SIGQUIT => "SIGQUIT",
            Signal::SIGTSTP => "SIGTSTP",
            Signal::SIGHUP => "SIGHUP",
            Signal::SIGTERM => "SIGTERM",
            Signal::SIGKILL => "SIGKILL",
            Signal::SIGSTOP => "SIGSTOP",
            Signal::SIGCONT => "SIGCONT",
            Signal::SIGWINCH => "SIGWINCH",
            Signal::SIGUSR1 => "SIGUSR1",
            Signal::SIGUSR2 => "SIGUSR2",
        }
    }

    /// Get the signal number (POSIX compatible).
    pub fn number(&self) -> i32 {
        match self {
            Signal::SIGHUP => 1,
            Signal::SIGINT => 2,
            Signal::SIGQUIT => 3,
            Signal::SIGKILL => 9,
            Signal::SIGTERM => 15,
            Signal::SIGSTOP => 17,
            Signal::SIGTSTP => 18,
            Signal::SIGCONT => 19,
            Signal::SIGUSR1 => 10,
            Signal::SIGUSR2 => 12,
            Signal::SIGWINCH => 28,
        }
    }

    /// Parse a signal from a name or number.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "SIGINT" | "INT" | "2" => Some(Signal::SIGINT),
            "SIGQUIT" | "QUIT" | "3" => Some(Signal::SIGQUIT),
            "SIGTSTP" | "TSTP" | "18" => Some(Signal::SIGTSTP),
            "SIGHUP" | "HUP" | "1" => Some(Signal::SIGHUP),
            "SIGTERM" | "TERM" | "15" => Some(Signal::SIGTERM),
            "SIGKILL" | "KILL" | "9" => Some(Signal::SIGKILL),
            "SIGSTOP" | "STOP" | "17" => Some(Signal::SIGSTOP),
            "SIGCONT" | "CONT" | "19" => Some(Signal::SIGCONT),
            "SIGWINCH" | "WINCH" | "28" => Some(Signal::SIGWINCH),
            "SIGUSR1" | "USR1" | "10" => Some(Signal::SIGUSR1),
            "SIGUSR2" | "USR2" | "12" => Some(Signal::SIGUSR2),
            _ => None,
        }
    }
}

/// Signal handler function type.
pub type SignalHandler = Box<dyn Fn() + Send + Sync>;

/// Manages signal traps.
pub struct SignalManager {
    /// Registered signal handlers
    handlers: HashMap<Signal, String>,
    /// Whether the shell has been interrupted
    interrupted: AtomicBool,
    /// Whether to exit on interrupt
    exit_on_interrupt: bool,
}

impl SignalManager {
    /// Create a new signal manager.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            interrupted: AtomicBool::new(false),
            exit_on_interrupt: false,
        }
    }

    /// Trap a signal with a handler.
    pub fn trap(&mut self, signal: Signal, handler: &str) {
        if handler.is_empty() || handler == "-" {
            // Reset to default behavior
            self.handlers.remove(&signal);
        } else {
            self.handlers.insert(signal, handler.to_string());
        }
    }

    /// Get the handler for a signal.
    pub fn get_handler(&self, signal: &Signal) -> Option<&str> {
        self.handlers.get(signal).map(|s| s.as_str())
    }

    /// Set the interrupted flag.
    pub fn set_interrupted(&self, interrupted: bool) {
        self.interrupted.store(interrupted, Ordering::SeqCst);
    }

    /// Check if the shell has been interrupted.
    pub fn is_interrupted(&self) -> bool {
        self.interrupted.load(Ordering::SeqCst)
    }

    /// List all current traps.
    pub fn list_traps(&self) -> Vec<(Signal, &str)> {
        self.handlers.iter().map(|(sig, handler)| (*sig, handler.as_str())).collect()
    }

    /// Clear all traps.
    pub fn clear(&mut self) {
        self.handlers.clear();
    }
}

impl Default for SignalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_name() {
        assert_eq!(Signal::SIGINT.name(), "SIGINT");
        assert_eq!(Signal::SIGTERM.name(), "SIGTERM");
        assert_eq!(Signal::SIGKILL.name(), "SIGKILL");
    }

    #[test]
    fn test_signal_number() {
        assert_eq!(Signal::SIGINT.number(), 2);
        assert_eq!(Signal::SIGTERM.number(), 15);
        assert_eq!(Signal::SIGKILL.number(), 9);
    }

    #[test]
    fn test_signal_parse() {
        assert_eq!(Signal::parse("SIGINT"), Some(Signal::SIGINT));
        assert_eq!(Signal::parse("INT"), Some(Signal::SIGINT));
        assert_eq!(Signal::parse("2"), Some(Signal::SIGINT));
        assert_eq!(Signal::parse("INVALID"), None);
    }

    #[test]
    fn test_trap_and_get_handler() {
        let mut mgr = SignalManager::new();
        mgr.trap(Signal::SIGINT, "echo interrupted");
        assert_eq!(mgr.get_handler(&Signal::SIGINT), Some("echo interrupted"));
        assert_eq!(mgr.get_handler(&Signal::SIGTERM), None);
    }

    #[test]
    fn test_trap_remove() {
        let mut mgr = SignalManager::new();
        mgr.trap(Signal::SIGINT, "echo handler");
        mgr.trap(Signal::SIGINT, "-");
        assert_eq!(mgr.get_handler(&Signal::SIGINT), None);
    }

    #[test]
    fn test_list_traps() {
        let mut mgr = SignalManager::new();
        mgr.trap(Signal::SIGINT, "echo int");
        mgr.trap(Signal::SIGTERM, "echo term");
        let traps = mgr.list_traps();
        assert_eq!(traps.len(), 2);
    }
}
