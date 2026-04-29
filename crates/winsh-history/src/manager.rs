//! History manager implementation.

/// Manages shell command history.
pub struct HistoryManager;

impl HistoryManager {
    /// Create a new history manager.
    pub fn new() -> Self {
        Self
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}
