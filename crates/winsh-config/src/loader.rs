//! Configuration loader for WinSH.
//!
//! Supports the zsh-style config file hierarchy:
//! - /etc/winshenv (system-wide, always loaded)
//! - ~/.winshenv (user, always loaded)
//! - ~/.winshrc (interactive shells)
//! - ~/.winshprofile (login shells)
//! - ~/.winshlogin (after winshrc, login shells)
//! - ~/.winshlogout (on exit)

use std::path::PathBuf;
use winsh_lexer::Lexer;
use winsh_parser::Parser;
use winsh_core::{ShellError, ShellState};

/// Types of shell sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    /// Login shell (--login flag or started by login)
    Login,
    /// Interactive shell (has a terminal)
    Interactive,
    /// Non-interactive (script or pipe)
    NonInteractive,
}

/// Manages shell configuration loading.
pub struct ConfigLoader {
    /// Whether configuration has been loaded
    loaded: bool,
    /// Shell session type
    shell_type: ShellType,
}

impl ConfigLoader {
    /// Create a new config loader.
    pub fn new() -> Self {
        Self {
            loaded: false,
            shell_type: ShellType::Interactive,
        }
    }

    /// Create a config loader for a specific shell type.
    pub fn with_type(shell_type: ShellType) -> Self {
        Self {
            loaded: false,
            shell_type,
        }
    }

    /// Load all configuration files for the current session type.
    pub fn load(&mut self, state: &mut ShellState) -> Result<(), ShellError> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

        // 1. Load /etc/winshenv (system-wide, always)
        self.load_if_exists(&PathBuf::from("/etc/winshenv"), state)?;

        // 2. Load ~/.winshenv (user, always)
        self.load_if_exists(&home.join(".winshenv"), state)?;

        // 3. For login shells
        if self.shell_type == ShellType::Login {
            // Load ~/.winshprofile
            self.load_if_exists(&home.join(".winshprofile"), state)?;
        }

        // 4. For interactive shells
        if self.shell_type == ShellType::Interactive || self.shell_type == ShellType::Login {
            // Load ~/.winshrc
            self.load_if_exists(&home.join(".winshrc"), state)?;
            // Also support .winshrc.toml for backward compat
            self.load_toml_if_exists(&home.join(".winshrc.toml"), state)?;
        }

        // 5. For login shells (after winshrc)
        if self.shell_type == ShellType::Login {
            self.load_if_exists(&home.join(".winshlogin"), state)?;
        }

        self.loaded = true;
        Ok(())
    }

    /// Load the logout configuration file.
    pub fn load_logout(&self, state: &mut ShellState) -> Result<(), ShellError> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        self.load_if_exists(&home.join(".winshlogout"), state)?;
        Ok(())
    }

    /// Load a config file if it exists.
    fn load_if_exists(&self, path: &PathBuf, state: &mut ShellState) -> Result<(), ShellError> {
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| ShellError::ConfigError(format!("{}: {}", path.display(), e)))?;

        self.execute_config(&content, state)?;
        Ok(())
    }

    /// Load a TOML config file if it exists.
    fn load_toml_if_exists(&self, path: &PathBuf, state: &mut ShellState) -> Result<(), ShellError> {
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| ShellError::ConfigError(format!("{}: {}", path.display(), e)))?;

        // Parse TOML aliases
        if let Ok(toml_data) = content.parse::<toml::Table>() {
            if let Some(aliases) = toml_data.get("aliases") {
                if let Some(alias_table) = aliases.as_table() {
                    for (name, value) in alias_table {
                        if let Some(val_str) = value.as_str() {
                            state.set_alias(name.clone(), val_str.to_string());
                        }
                    }
                }
            }

            if let Some(env) = toml_data.get("environment") {
                if let Some(env_table) = env.as_table() {
                    for (name, value) in env_table {
                        if let Some(val_str) = value.as_str() {
                            state.env.set(name.clone(), val_str.to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a configuration script.
    fn execute_config(&self, script: &str, state: &mut ShellState) -> Result<(), ShellError> {
        for line in script.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Handle simple variable assignments
            if let Some((name, value)) = parse_assignment(trimmed) {
                state.env.set(&name, value);
                continue;
            }

            // Handle export statements
            if let Some(rest) = trimmed.strip_prefix("export ") {
                let rest = rest.trim();
                if let Some((name, value)) = rest.split_once('=') {
                    state.env.export(name.trim(), value.trim());
                } else {
                    state.env.export_existing(rest);
                }
                continue;
            }

            // Handle alias statements
            if let Some(rest) = trimmed.strip_prefix("alias ") {
                let rest = rest.trim();
                if let Some((name, value)) = rest.split_once('=') {
                    let value = value.trim().trim_matches('\'').trim_matches('"');
                    state.set_alias(name.trim().to_string(), value.to_string());
                }
                continue;
            }

            // Handle setopt/unsetopt
            if let Some(opt) = trimmed.strip_prefix("setopt ") {
                let opt = opt.trim();
                self.apply_option(opt, true, state);
                continue;
            }
            if let Some(opt) = trimmed.strip_prefix("unsetopt ") {
                let opt = opt.trim();
                self.apply_option(opt, false, state);
                continue;
            }

            // Handle PS1/PROMPT
            if let Some(rest) = trimmed.strip_prefix("PS1=") {
                state.config.prompt = rest.trim().to_string();
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("PROMPT=") {
                state.config.prompt = rest.trim().to_string();
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("RPROMPT=") {
                state.config.rprompt = rest.trim().to_string();
                continue;
            }

            // Try to execute as a command
            // TODO: Execute as full shell command
        }

        Ok(())
    }

    /// Apply a shell option.
    fn apply_option(&self, option: &str, value: bool, state: &mut ShellState) {
        let opts = state.options_mut();
        match option {
            "errexit" | "e" => opts.errexit = value,
            "nounset" | "u" => opts.nounset = value,
            "noglob" | "f" => opts.noglob = value,
            "extended_glob" => opts.extended_glob = value,
            "null_glob" => opts.null_glob = value,
            "glob_dots" => opts.glob_dots = value,
            "case_glob" => opts.case_glob = value,
            "hist_ignore_dups" => opts.hist_ignore_dups = value,
            "hist_ignore_all_dups" => opts.hist_ignore_all_dups = value,
            "hist_ignore_space" => opts.hist_ignore_space = value,
            "hist_save_no_dups" => opts.hist_save_no_dups = value,
            "hist_verify" => opts.hist_verify = value,
            "prompt_subst" => opts.prompt_subst = value,
            "prompt_percent" => opts.prompt_percent = value,
            "brace_expand" => opts.brace_expand = value,
            "tilde_expand" => opts.tilde_expand = value,
            "variable_expand" => opts.variable_expand = value,
            "command_subst" => opts.command_subst = value,
            "arith_expand" => opts.arith_expand = value,
            "xtrace" | "x" => opts.xtrace = value,
            "verbose" | "v" => opts.verbose = value,
            "monitor" | "m" => opts.monitor = value,
            "notify" => opts.notify = value,
            "vi_mode" => opts.vi_mode = value,
            "emacs_mode" => opts.emacs_mode = value,
            _ => {}
        }
    }

    /// Get the list of all config files for the current session type.
    pub fn config_files(&self) -> Vec<String> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let mut files = vec![
            "/etc/winshenv".to_string(),
            home.join(".winshenv").to_string_lossy().to_string(),
        ];

        if self.shell_type == ShellType::Login {
            files.push(home.join(".winshprofile").to_string_lossy().to_string());
        }

        if self.shell_type == ShellType::Interactive || self.shell_type == ShellType::Login {
            files.push(home.join(".winshrc").to_string_lossy().to_string());
        }

        if self.shell_type == ShellType::Login {
            files.push(home.join(".winshlogin").to_string_lossy().to_string());
        }

        files
    }

    /// Get the path to the logout config file.
    pub fn logout_file(&self) -> String {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".winshlogout").to_string_lossy().to_string()
    }

    /// Check if configuration has been loaded.
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Set the shell type.
    pub fn set_shell_type(&mut self, shell_type: ShellType) {
        self.shell_type = shell_type;
    }
}

/// Parse a simple variable assignment (NAME=value).
fn parse_assignment(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if let Some((name, value)) = line.split_once('=') {
        let name = name.trim();
        // Only match valid variable names
        if name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.is_empty() {
            let value = value.trim();
            // Remove surrounding quotes if present
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };
            return Some((name.to_string(), value.to_string()));
        }
    }
    None
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loader_new() {
        let loader = ConfigLoader::new();
        assert!(!loader.is_loaded());
    }

    #[test]
    fn test_config_files_interactive() {
        let loader = ConfigLoader::with_type(ShellType::Interactive);
        let files = loader.config_files();
        assert!(files.iter().any(|f| f.ends_with(".winshrc")));
        assert!(files.iter().any(|f| f.ends_with(".winshenv")));
    }

    #[test]
    fn test_config_files_login() {
        let loader = ConfigLoader::with_type(ShellType::Login);
        let files = loader.config_files();
        assert!(files.iter().any(|f| f.ends_with(".winshprofile")));
        assert!(files.iter().any(|f| f.ends_with(".winshlogin")));
    }

    #[test]
    fn test_parse_assignment() {
        assert_eq!(parse_assignment("NAME=value"), Some(("NAME".to_string(), "value".to_string())));
        assert_eq!(parse_assignment("NAME=\"quoted value\""), Some(("NAME".to_string(), "quoted value".to_string())));
        assert_eq!(parse_assignment("invalid"), None);
    }

    #[test]
    fn test_execute_config_variables() {
        let mut loader = ConfigLoader::new();
        let mut state = ShellState::new();
        let config = r#"
# Comment
NAME=hello
export EXPORTED=world
alias ll='ls -la'
setopt hist_ignore_dups
"#;
        loader.execute_config(config, &mut state).unwrap();
        assert_eq!(state.env.get("NAME"), Some("hello"));
        assert_eq!(state.env.get("EXPORTED"), Some("world"));
        assert_eq!(state.get_alias("ll"), Some("ls -la"));
    }

    #[test]
    fn test_apply_option() {
        let mut state = ShellState::new();
        let loader = ConfigLoader::new();

        loader.apply_option("errexit", true, &mut state);
        assert!(state.options().errexit);

        loader.apply_option("vi_mode", true, &mut state);
        assert!(state.options().vi_mode);

        loader.apply_option("emacs_mode", false, &mut state);
        assert!(!state.options().emacs_mode);
    }
}
