// Configuration management for WinSH
use crate::error::{Result, ShellError};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// WinuxCmd configuration
#[derive(Debug, Clone, Deserialize)]
pub struct WinuxCmdConfig {
    #[serde(default = "default_enable_dll")]
    pub enable_dll: bool,

    #[serde(default = "default_auto_start_daemon")]
    pub auto_start_daemon: bool,

    #[serde(default = "default_daemon_timeout")]
    pub daemon_timeout: u64,
}

impl Default for WinuxCmdConfig {
    fn default() -> Self {
        WinuxCmdConfig {
            enable_dll: true,
            auto_start_daemon: true,
            daemon_timeout: 5,
        }
    }
}

fn default_enable_dll() -> bool {
    true
}

fn default_auto_start_daemon() -> bool {
    true
}

fn default_daemon_timeout() -> u64 {
    5
}

/// Terminal color configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TerminalColors {
    #[serde(default = "default_prompt_user_color")]
    pub prompt_user: String,

    #[serde(default = "default_prompt_host_color")]
    pub prompt_host: String,

    #[serde(default = "default_prompt_dir_color")]
    pub prompt_dir: String,

    #[serde(default = "default_prompt_symbol")]
    pub prompt_symbol: String,
}

impl Default for TerminalColors {
    fn default() -> Self {
        TerminalColors {
            prompt_user: "\x1b[1;32m".to_string(), // Green
            prompt_host: "\x1b[1;32m".to_string(), // Green
            prompt_dir: "\x1b[1;34m".to_string(),  // Blue
            prompt_symbol: "\x1b[0m".to_string(),  // Reset
        }
    }
}

fn default_prompt_user_color() -> String {
    "\x1b[1;32m".to_string()
}

fn default_prompt_host_color() -> String {
    "\x1b[1;32m".to_string()
}

fn default_prompt_dir_color() -> String {
    "\x1b[1;34m".to_string()
}

fn default_prompt_symbol() -> String {
    "\x1b[0m".to_string()
}

/// Configuration for the pluggable completion system.
#[derive(Debug, Clone, Deserialize)]
pub struct CompletionsConfig {
    /// Directories that contain per-command completion definition files (*.toml / *.bash).
    /// Defaults to `~/.winsh/completions` when empty.
    #[serde(default)]
    pub completion_dirs: Vec<String>,

    /// Enable the built-in command-name completion plugin (default: true).
    #[serde(default = "default_true")]
    pub enable_command_completion: bool,
}

impl Default for CompletionsConfig {
    fn default() -> Self {
        CompletionsConfig {
            completion_dirs: Vec::new(),
            enable_command_completion: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Shell configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ShellConfig {
    #[serde(default)]
    pub colors: TerminalColors,

    #[serde(default = "default_prompt_format")]
    pub prompt_format: String,

    #[serde(default)]
    pub plugins: Vec<String>,

    #[serde(default)]
    pub winuxcmd: WinuxCmdConfig,

    #[serde(default)]
    pub aliases: std::collections::HashMap<String, String>,

    #[serde(default)]
    pub completions: CompletionsConfig,
}

impl Default for ShellConfig {
    fn default() -> Self {
        ShellConfig {
            colors: TerminalColors::default(),
            prompt_format: "%u@%h %w $ ".to_string(),
            plugins: Vec::new(),
            winuxcmd: WinuxCmdConfig::default(),
            aliases: std::collections::HashMap::new(),
            completions: CompletionsConfig::default(),
        }
    }
}

fn default_prompt_format() -> String {
    "%u@%h %w $ ".to_string()
}

/// Configuration manager
pub struct ConfigManager {
    config: ShellConfig,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        ConfigManager {
            config: ShellConfig::default(),
        }
    }

    /// Load configuration from file
    pub fn load_config(&mut self, path: &Path) -> Result<ShellConfig> {
        let config_content = std::fs::read_to_string(path)
            .map_err(|e| ShellError::Config(format!("Failed to read config: {}", e)))?;

        // Try to parse as TOML first
        if path.extension().map(|e| e == "toml").unwrap_or(false) {
            self.config = toml::from_str(&config_content)
                .map_err(|e| ShellError::Config(format!("Failed to parse TOML: {}", e)))?;
        } else {
            // Fallback to key=value format for legacy support
            self.parse_legacy_config(&config_content)?;
        }

        Ok(self.config.clone())
    }

    /// Parse legacy key=value format
    fn parse_legacy_config(&mut self, content: &str) -> Result<()> {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = self.parse_ansi_escape_sequences(value.trim());

                match key {
                    "prompt_user_color" => {
                        self.config.colors.prompt_user = value;
                    }
                    "prompt_host_color" => {
                        self.config.colors.prompt_host = value;
                    }
                    "prompt_dir_color" => {
                        self.config.colors.prompt_dir = value;
                    }
                    "prompt_symbol" => {
                        self.config.colors.prompt_symbol = value;
                    }
                    "prompt_format" => {
                        self.config.prompt_format = value;
                    }
                    "plugin" => {
                        self.config.plugins.push(value);
                    }
                    _ => {
                        // Check if it's an alias
                        if !key.contains('_') && !key.contains('.') {
                            self.config.aliases.insert(key.to_string(), value);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &ShellConfig {
        &self.config
    }

    /// Parse ANSI escape sequences like \x1b[1;32m
    fn parse_ansi_escape_sequences(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();

        while let Some(&c) = chars.peek() {
            chars.next();

            if c == '\\' {
                if let Some(&'x') = chars.peek() {
                    chars.next();
                    // Parse hex escape sequence
                    let mut hex_str = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_hexdigit() && hex_str.len() < 2 {
                            chars.next();
                            hex_str.push(c);
                        } else {
                            break;
                        }
                    }

                    if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                        result.push(byte as char);
                    }
                } else {
                    // Keep backslash for other escape sequences
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Find configuration file in standard locations
    pub fn find_config_file() -> Option<PathBuf> {
        let home_dir = dirs::home_dir()?;
        let config_paths = vec![
            PathBuf::from(".winshrc.toml"),
            home_dir.join(".winshrc.toml"),
            PathBuf::from(".winshrc"),
            home_dir.join(".winshrc"),
        ];

        config_paths.into_iter().find(|path| path.exists())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ShellConfig::default();
        assert_eq!(config.prompt_format, "%u@%h %w $ ");
        assert_eq!(config.plugins.len(), 0);
        assert!(config.winuxcmd.enable_dll);
        assert!(config.winuxcmd.auto_start_daemon);
        assert_eq!(config.winuxcmd.daemon_timeout, 5);
    }

    #[test]
    fn test_default_colors() {
        let colors = TerminalColors::default();
        assert_eq!(colors.prompt_user, "\x1b[1;32m");
        assert_eq!(colors.prompt_host, "\x1b[1;32m");
        assert_eq!(colors.prompt_dir, "\x1b[1;34m");
        assert_eq!(colors.prompt_symbol, "\x1b[0m");
    }

    #[test]
    fn test_ansi_parsing() {
        let manager = ConfigManager::new();
        let result = manager.parse_ansi_escape_sequences("\\x1b[1;32m");
        assert_eq!(result, "\x1b[1;32m");
    }

    #[test]
    fn test_winuxcmd_default() {
        let config = WinuxCmdConfig::default();
        assert!(config.enable_dll);
        assert!(config.auto_start_daemon);
        assert_eq!(config.daemon_timeout, 5);
    }
}
