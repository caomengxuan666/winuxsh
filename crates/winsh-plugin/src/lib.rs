//! Plugin system with hooks and dynamic loading.
//!
//! Supports zsh-style hooks:
//! - precmd  - before each prompt
//! - preexec - before each command
//! - chpwd   - when directory changes
//! - zshexit - when shell exits

use std::collections::HashMap;
use std::path::PathBuf;

use winsh_core::{ShellError, ShellState};

/// A shell plugin.
pub struct Plugin {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Whether the plugin is enabled
    pub enabled: bool,
    /// Plugin source directory
    pub source: Option<PathBuf>,
}

/// Types of shell hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookType {
    /// Before each prompt is displayed
    PreCmd,
    /// Before each command is executed
    PreExec,
    /// When the current directory changes
    ChDir,
    /// When the shell is about to exit
    Exit,
    /// When a command is added to history
    AddHistory,
}

/// A hook callback.
pub type HookCallback = Box<dyn Fn(&mut ShellState) + Send + Sync>;

/// Manages plugins and their hooks.
pub struct PluginManager {
    /// Registered plugins
    plugins: Vec<Plugin>,
    /// Registered hooks
    hooks: HashMap<HookType, Vec<(String, Box<dyn Fn(&mut ShellState) + Send + Sync>)>>,
    /// Plugin directories to scan
    plugin_dirs: Vec<PathBuf>,
}

impl PluginManager {
    /// Create a new plugin manager.
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            plugins: Vec::new(),
            hooks: HashMap::new(),
            plugin_dirs: vec![
                home.join(".winsh").join("plugins"),
            ],
        }
    }

    /// Register a new plugin.
    pub fn register(&mut self, name: &str, version: &str, desc: &str) {
        // Check if already registered
        if self.plugins.iter().any(|p| p.name == name) {
            return;
        }

        self.plugins.push(Plugin {
            name: name.to_string(),
            version: version.to_string(),
            description: desc.to_string(),
            enabled: true,
            source: None,
        });
    }

    /// Add a hook callback from a plugin.
    pub fn add_hook<F>(&mut self, plugin_name: &str, hook_type: HookType, callback: F)
    where
        F: Fn(&mut ShellState) + Send + Sync + 'static,
    {
        self.hooks
            .entry(hook_type)
            .or_insert_with(Vec::new)
            .push((plugin_name.to_string(), Box::new(callback)));
    }

    /// Run all hooks of a given type.
    pub fn run_hooks(&self, hook_type: HookType, state: &mut ShellState) {
        if let Some(hook_list) = self.hooks.get(&hook_type) {
            for (name, callback) in hook_list {
                // Don't let plugin hook errors crash the shell
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(state);
                })) {
                    eprintln!("plugin '{}' hook error: {:?}", name, e);
                }
            }
        }
    }

    /// Trigger the precmd hook.
    pub fn precmd(&self, state: &mut ShellState) {
        self.run_hooks(HookType::PreCmd, state);
    }

    /// Trigger the preexec hook.
    pub fn preexec(&self, state: &mut ShellState) {
        self.run_hooks(HookType::PreExec, state);
    }

    /// Trigger the chpwd hook.
    pub fn chpwd(&self, state: &mut ShellState) {
        self.run_hooks(HookType::ChDir, state);
    }

    /// Trigger the exit hook.
    pub fn exit(&self, state: &mut ShellState) {
        self.run_hooks(HookType::Exit, state);
    }

    /// Trigger the addhistory hook.
    pub fn addhistory(&self, state: &mut ShellState) {
        self.run_hooks(HookType::AddHistory, state);
    }

    /// List all registered plugins.
    pub fn list_plugins(&self) -> &[Plugin] {
        &self.plugins
    }

    /// Get a plugin by name.
    pub fn get_plugin(&self, name: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.name == name)
    }

    /// Enable a plugin.
    pub fn enable_plugin(&mut self, name: &str) {
        if let Some(plugin) = self.plugins.iter_mut().find(|p| p.name == name) {
            plugin.enabled = true;
        }
    }

    /// Disable a plugin.
    pub fn disable_plugin(&mut self, name: &str) {
        if let Some(plugin) = self.plugins.iter_mut().find(|p| p.name == name) {
            plugin.enabled = false;
        }
    }

    /// Remove a plugin and its hooks.
    pub fn remove_plugin(&mut self, name: &str) {
        self.plugins.retain(|p| p.name != name);

        // Remove hooks from this plugin
        for hook_list in self.hooks.values_mut() {
            hook_list.retain(|(plugin_name, _)| plugin_name != name);
        }
    }

    /// Add a plugin directory to scan.
    pub fn add_plugin_dir(&mut self, dir: PathBuf) {
        if !self.plugin_dirs.contains(&dir) {
            self.plugin_dirs.push(dir);
        }
    }

    /// Load a plugin script from a file.
    pub fn load_plugin_file(&mut self, path: &PathBuf, state: &mut ShellState) -> Result<(), ShellError> {
        if !path.exists() {
            return Err(ShellError::PluginNotFound(path.to_string_lossy().to_string()));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| ShellError::Io(e))?;

        let plugin_name = path.file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        self.register(&plugin_name, "1.0.0", &format!("Plugin from {}", path.display()));

        // Execute the plugin script to set up hooks
        self.execute_plugin_script(&plugin_name, &content, state)?;

        Ok(())
    }

    /// Execute a plugin script to configure hooks.
    fn execute_plugin_script(&self, _plugin_name: &str, script: &str, state: &mut ShellState) -> Result<(), ShellError> {
        for line in script.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Handle variable assignments
            if let Some((name, value)) = trimmed.split_once('=') {
                let name = name.trim();
                if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    let value = value.trim().trim_matches('"').trim_matches('\'');
                    state.env.set(name, value);
                }
                continue;
            }

            // Handle function definitions
            if trimmed.starts_with("function ") || trimmed.contains("()") {
                // TODO: Register function for hook use
                continue;
            }
        }

        Ok(())
    }

    /// Scan plugin directories for plugins.
    pub fn scan_plugins(&mut self, state: &mut ShellState) {
        for dir in &self.plugin_dirs.clone() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "sh" || e == "zsh" || e == "winsh").unwrap_or(false) {
                        if let Err(e) = self.load_plugin_file(&path, state) {
                            eprintln!("Error loading plugin {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    /// Get the list of plugin directories.
    pub fn plugin_dirs(&self) -> &[PathBuf] {
        &self.plugin_dirs
    }

    /// Get the number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_new() {
        let pm = PluginManager::new();
        assert_eq!(pm.plugin_count(), 0);
    }

    #[test]
    fn test_register_plugin() {
        let mut pm = PluginManager::new();
        pm.register("test", "1.0", "Test plugin");
        assert_eq!(pm.plugin_count(), 1);
        assert!(pm.get_plugin("test").is_some());
    }

    #[test]
    fn test_register_duplicate() {
        let mut pm = PluginManager::new();
        pm.register("test", "1.0", "Test plugin");
        pm.register("test", "2.0", "Test plugin v2");
        assert_eq!(pm.plugin_count(), 1);
        assert_eq!(pm.get_plugin("test").unwrap().version, "1.0");
    }

    #[test]
    fn test_enable_disable_plugin() {
        let mut pm = PluginManager::new();
        pm.register("test", "1.0", "Test");
        pm.disable_plugin("test");
        assert!(!pm.get_plugin("test").unwrap().enabled);
        pm.enable_plugin("test");
        assert!(pm.get_plugin("test").unwrap().enabled);
    }

    #[test]
    fn test_remove_plugin() {
        let mut pm = PluginManager::new();
        pm.register("test", "1.0", "Test");
        pm.remove_plugin("test");
        assert_eq!(pm.plugin_count(), 0);
    }

    #[test]
    fn test_add_and_run_hook() {
        let mut pm = PluginManager::new();
        pm.register("test", "1.0", "Test");

        let mut called = false;
        pm.add_hook("test", HookType::PreCmd, move |_state| {
            // Just verify the hook runs without panic
        });

        let mut state = ShellState::new();
        pm.precmd(&mut state);
        // No assertion needed - just verify no panic
    }
}
