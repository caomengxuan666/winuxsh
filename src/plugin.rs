// Plugin system for WinSH
use crate::error::Result;
use crate::shell::Shell;

/// Plugin trait for extensibility
pub trait Plugin: std::fmt::Debug {
    /// Get plugin name
    fn name(&self) -> &str;

    /// Initialize plugin
    fn init(&mut self) -> Result<()>;

    /// Execute plugin command
    fn execute(&self, args: &[String], shell: &mut Shell) -> Result<bool>; // Return true if handled

    /// Get plugin description
    fn description(&self) -> &str {
        "No description available"
    }
}

/// Welcome plugin
#[derive(Debug)]
pub struct WelcomePlugin;

impl Plugin for WelcomePlugin {
    fn name(&self) -> &str {
        "welcome"
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn execute(&self, args: &[String], shell: &mut Shell) -> Result<bool> {
        if args.get(0).map(|s| s.as_str()) == Some("welcome") {
            println!("Welcome to WinSH MVP6!");
            println!("Type 'help' for available commands.");
            println!("Type 'plugin list' to see loaded plugins.");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn description(&self) -> &str {
        "Welcome message plugin"
    }
}

/// Plugin manager
#[derive(Debug)]
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        PluginManager {
            plugins: Vec::new(),
        }
    }

    /// Add a plugin
    pub fn add_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let mut plugin = plugin;
        plugin.init()?;
        self.plugins.push(plugin);
        Ok(())
    }

    /// Execute plugins
    pub fn execute(&mut self, args: &[String], shell: &mut Shell) -> Result<bool> {
        for plugin in &mut self.plugins {
            if plugin.execute(args, shell)? {
                return Ok(true); // Plugin handled the command
            }
        }
        Ok(false) // No plugin handled the command
    }

    /// Execute plugins with immutable self reference
    pub fn execute_readonly(&self, args: &[String], shell: &mut Shell) -> Result<bool> {
        for plugin in &self.plugins {
            if plugin.execute(args, shell)? {
                return Ok(true); // Plugin handled the command
            }
        }
        Ok(false) // No plugin handled the command
    }
    /// List all plugins
    pub fn list_plugins(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }

    /// Get number of loaded plugins
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
    fn test_welcome_plugin() {
        let plugin = WelcomePlugin;
        assert_eq!(plugin.name(), "welcome");
        assert_eq!(plugin.description(), "Welcome message plugin");
    }

    #[test]
    fn test_welcome_plugin_execute() {
        let mut plugin = WelcomePlugin;
        assert!(plugin.init().is_ok());
    }

    #[test]
    fn test_plugin_manager() {
        let mut manager = PluginManager::new();
        assert_eq!(manager.plugin_count(), 0);

        manager.add_plugin(Box::new(WelcomePlugin)).unwrap();
        assert_eq!(manager.plugin_count(), 1);
        assert_eq!(manager.list_plugins(), vec!["welcome"]);
    }
}
