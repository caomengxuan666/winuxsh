//! Configuration loader.

/// Loads shell configuration files.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Create a new config loader.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
