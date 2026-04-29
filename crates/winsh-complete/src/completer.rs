//! The main completer implementation.

/// Tab completer for the WinSH shell.
pub struct Completer;

impl Completer {
    /// Create a new completer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Completer {
    fn default() -> Self {
        Self::new()
    }
}
