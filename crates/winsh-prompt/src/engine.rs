//! Prompt engine implementation.

/// Generates shell prompts.
pub struct PromptEngine;

impl PromptEngine {
    /// Create a new prompt engine.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PromptEngine {
    fn default() -> Self {
        Self::new()
    }
}
