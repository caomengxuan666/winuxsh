//! Redirection handling.

/// Redirection handler.
pub struct RedirectionHandler;

impl RedirectionHandler {
    /// Create a new redirection handler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RedirectionHandler {
    fn default() -> Self {
        Self::new()
    }
}
