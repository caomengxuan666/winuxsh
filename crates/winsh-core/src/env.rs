//! Environment variable management.

use std::collections::HashMap;
use std::env;
use std::fmt;

/// Manages shell environment variables.
///
/// Environment variables are stored in a HashMap and can be exported
/// to the process environment.
#[derive(Debug, Clone)]
pub struct Env {
    /// Shell variables (not exported to child processes)
    vars: HashMap<String, String>,
    /// Exported environment variables
    exports: HashMap<String, String>,
}

impl Env {
    /// Create a new environment manager.
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            exports: HashMap::new(),
        }
    }

    /// Create an environment initialized from the current process environment.
    pub fn from_process() -> Self {
        let mut env = Self::new();
        // Import all environment variables
        for (key, value) in env::vars() {
            env.exports.insert(key, value);
        }
        // Ensure PATH is always available (some contexts might not export it)
        if env.exports.get("PATH").is_none() {
            if let Ok(path) = env::var("PATH") {
                env.exports.insert("PATH".to_string(), path);
            }
        }
        env
    }

    /// Get a variable value (checks exports first, then vars).
    pub fn get(&self, name: &str) -> Option<&str> {
        self.exports
            .get(name)
            .or_else(|| self.vars.get(name))
            .map(|s| s.as_str())
    }

    /// Set a shell variable (not exported).
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(name.into(), value.into());
    }

    /// Export a variable (makes it available to child processes).
    pub fn export(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();
        self.exports.insert(name.clone(), value.clone());
        self.vars.remove(&name);
        // Also set in process environment
        env::set_var(&name, &value);
    }

    /// Export an existing variable.
    pub fn export_existing(&mut self, name: &str) {
        if let Some(value) = self.vars.remove(name) {
            self.exports.insert(name.to_string(), value.clone());
            env::set_var(name, &value);
        }
    }

    /// Unset a variable.
    pub fn unset(&mut self, name: &str) {
        self.vars.remove(name);
        if self.exports.remove(name).is_some() {
            env::remove_var(name);
        }
    }

    /// Check if a variable is set.
    pub fn has(&self, name: &str) -> bool {
        self.exports.contains_key(name) || self.vars.contains_key(name)
    }

    /// Check if a variable is exported.
    pub fn is_exported(&self, name: &str) -> bool {
        self.exports.contains_key(name)
    }

    /// Get all variables (both exported and non-exported).
    pub fn all(&self) -> HashMap<String, String> {
        let mut result = self.exports.clone();
        result.extend(self.vars.clone());
        result
    }

    /// Get all exported variables.
    pub fn exported(&self) -> &HashMap<String, String> {
        &self.exports
    }

    /// Get all shell variables (non-exported).
    pub fn shell_vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    /// Get the number of variables.
    pub fn len(&self) -> usize {
        self.vars.len() + self.exports.len()
    }

    /// Check if there are no variables.
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty() && self.exports.is_empty()
    }

    /// Get the PATH as a vector of directories.
    pub fn path_dirs(&self) -> Vec<String> {
        self.get("PATH")
            .map(|path| {
                path.split(if cfg!(windows) { ';' } else { ':' })
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the HOME directory.
    pub fn home_dir(&self) -> Option<String> {
        self.get("HOME")
            .or_else(|| self.get("USERPROFILE"))
            .map(|s| s.to_string())
    }

    /// Get the current working directory.
    pub fn current_dir(&self) -> Option<String> {
        self.get("PWD").map(|s| s.to_string())
    }

    /// Set the current working directory.
    pub fn set_current_dir(&mut self, path: &str) {
        self.set("PWD", path);
    }

    /// Get the previous working directory.
    pub fn previous_dir(&self) -> Option<String> {
        self.get("OLDPWD").map(|s| s.to_string())
    }

    /// Set the previous working directory.
    pub fn set_previous_dir(&mut self, path: &str) {
        self.set("OLDPWD", path);
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::from_process()
    }
}

impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in &self.exports {
            writeln!(f, "declare -x {}=\"{}\"", key, value)?;
        }
        for (key, value) in &self.vars {
            writeln!(f, "{}={}", key, value)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_new() {
        let env = Env::new();
        assert!(env.is_empty());
    }

    #[test]
    fn test_env_set_get() {
        let mut env = Env::new();
        env.set("FOO", "bar");
        assert_eq!(env.get("FOO"), Some("bar"));
        assert!(!env.is_exported("FOO"));
    }

    #[test]
    fn test_env_export() {
        let mut env = Env::new();
        env.export("FOO", "bar");
        assert_eq!(env.get("FOO"), Some("bar"));
        assert!(env.is_exported("FOO"));
    }

    #[test]
    fn test_env_unset() {
        let mut env = Env::new();
        env.set("FOO", "bar");
        assert!(env.has("FOO"));
        env.unset("FOO");
        assert!(!env.has("FOO"));
    }

    #[test]
    fn test_env_all() {
        let mut env = Env::new();
        env.set("A", "1");
        env.export("B", "2");
        let all = env.all();
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("A").map(|s| s.as_str()), Some("1"));
        assert_eq!(all.get("B").map(|s| s.as_str()), Some("2"));
    }

    #[test]
    fn test_env_path_dirs() {
        let mut env = Env::new();
        let separator = if cfg!(windows) { ";" } else { ":" };
        let path = format!("/usr/bin{}/usr/local/bin", separator);
        env.set("PATH", &path);
        let dirs = env.path_dirs();
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0], "/usr/bin");
        assert_eq!(dirs[1], "/usr/local/bin");
    }

    #[test]
    fn test_env_home_dir() {
        let mut env = Env::new();
        env.set("HOME", "/home/user");
        assert_eq!(env.home_dir(), Some("/home/user".to_string()));
    }

    #[test]
    fn test_env_display() {
        let mut env = Env::new();
        env.set("A", "1");
        env.export("B", "2");
        let display = env.to_string();
        assert!(display.contains("declare -x B=\"2\""));
        assert!(display.contains("A=1"));
    }
}
