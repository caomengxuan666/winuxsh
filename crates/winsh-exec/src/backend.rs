//! Backend manager for command execution.
//!
//! Supports multiple backends:
//! - System: Uses standard PATH lookup (default)
//! - WinuxCmd: Uses uutils/winuxcmd coreutils implementation
//! - Auto: Prefers winuxcmd if available, falls back to system
//!
//! Configuration:
//!   backend = "auto"    # Auto-detect (default)
//!   backend = "system"  # Use system commands only
//!   backend = "winuxcmd" # Use winuxcmd/uutils coreutils

use std::path::PathBuf;
use std::process::Command;

use winsh_core::{BackendType, ShellError, ShellState};

/// Manages the command execution backend.
pub struct BackendManager {
    /// The selected backend type
    backend_type: BackendType,
    /// Path to winuxcmd binary (if configured)
    winuxcmd_path: Option<PathBuf>,
    /// Whether winuxcmd is available
    winuxcmd_available: bool,
}

/// Search for an executable in PATH.
fn find_in_path(cmd: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let full_path = dir.join(cmd);

        // Try with extensions on Windows
        if cfg!(windows) {
            for ext in &["", ".exe", ".cmd", ".bat"] {
                let with_ext = if ext.is_empty() { full_path.clone() }
                    else { full_path.with_extension(&ext[1..]) };
                if with_ext.exists() {
                    return Some(with_ext);
                }
            }
        } else if full_path.exists() {
            return Some(full_path);
        }
    }
    None
}

impl BackendManager {
    /// Create a new backend manager.
    pub fn new(backend_type: BackendType, winuxcmd_path: Option<PathBuf>) -> Self {
        let mut manager = Self {
            backend_type,
            winuxcmd_path: winuxcmd_path.clone(),
            winuxcmd_available: false,
        };

        // Detect winuxcmd availability
        let paths_to_check = vec![
            winuxcmd_path.unwrap_or_else(|| PathBuf::from("winuxcmd")),
            PathBuf::from("winuxcmd.exe"),
            PathBuf::from("C:\\Program Files\\winuxcmd\\winuxcmd.exe"),
        ];

        for path in &paths_to_check {
            if path.exists() {
                manager.winuxcmd_available = true;
                manager.winuxcmd_path = Some(path.clone());
                break;
            }
        }

        // Also check PATH
        if !manager.winuxcmd_available {
            if let Some(path) = find_in_path("winuxcmd") {
                manager.winuxcmd_available = true;
                manager.winuxcmd_path = Some(path);
            }
        }

        manager
    }

    /// Get the effective backend type after auto-detection.
    pub fn effective_backend(&self) -> BackendType {
        match self.backend_type {
            BackendType::Auto => {
                if self.winuxcmd_available {
                    BackendType::WinuxCmd
                } else {
                    BackendType::System
                }
            }
            other => other,
        }
    }

    /// Check if a command should be routed through winuxcmd.
    pub fn should_use_winuxcmd(&self, cmd: &str) -> bool {
        if self.effective_backend() != BackendType::WinuxCmd {
            return false;
        }

        // These are common coreutils commands that winuxcmd provides
        const WINUXCMD_COMMANDS: &[&str] = &[
            "ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "touch",
            "head", "tail", "wc", "sort", "uniq", "cut", "paste", "tr",
            "sed", "grep", "find", "xargs", "tee", "basename", "dirname",
            "chmod", "chown", "ln", "du", "df", "echo", "printf",
            "date", "sleep", "true", "false", "test", "[",
            "pwd", "whoami", "id", "env", "envsubst", "seq",
            "yes", "dd", "shuf", "shred", "fmt", "fold", "nl",
            "cksum", "hashsum", "md5sum", "sha1sum", "sha256sum",
            "split", "paste", "join", "comm",
        ];

        WINUXCMD_COMMANDS.contains(&cmd)
    }

    /// Execute a command through winuxcmd.
    pub fn execute_winuxcmd(&self, cmd: &str, args: &[&str], state: &ShellState) -> Result<i32, ShellError> {
        let winuxcmd = self.winuxcmd_path.as_ref()
            .ok_or_else(|| ShellError::command_not_found("winuxcmd"))?;

        let mut command = Command::new(winuxcmd);
        command.arg(cmd);
        command.args(args);

        // Set environment
        for (key, value) in state.env.exported() {
            command.env(key, value);
        }
        command.current_dir(state.current_dir());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }

        command.stdin(std::process::Stdio::inherit());
        command.stdout(std::process::Stdio::inherit());
        command.stderr(std::process::Stdio::inherit());

        let status = command.spawn()
            .and_then(|mut child| child.wait())
            .map_err(|e| ShellError::command_not_found(format!("winuxcmd: {}", e)))?;

        Ok(status.code().unwrap_or(1))
    }

    /// Get the path to winuxcmd (if available).
    pub fn wlinuxcmd_path(&self) -> Option<&PathBuf> {
        self.winuxcmd_path.as_ref()
    }

    /// Check if winuxcmd is available.
    pub fn is_winuxcmd_available(&self) -> bool {
        self.winuxcmd_available
    }

    /// Get the configured backend type.
    pub fn backend_type(&self) -> BackendType {
        self.backend_type
    }

    /// Get a description of the current backend.
    pub fn describe(&self) -> String {
        match self.effective_backend() {
            BackendType::System => "System PATH".to_string(),
            BackendType::WinuxCmd => {
                if let Some(path) = &self.winuxcmd_path {
                    format!("WinuxCmd ({})", path.display())
                } else {
                    "WinuxCmd".to_string()
                }
            }
            BackendType::Auto => {
                if self.winuxcmd_available {
                    "Auto (WinuxCmd)".to_string()
                } else {
                    "Auto (System)".to_string()
                }
            }
        }
    }
}

impl Default for BackendManager {
    fn default() -> Self {
        Self::new(BackendType::Auto, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_default() {
        let bm = BackendManager::default();
        assert_eq!(bm.backend_type(), BackendType::Auto);
    }

    #[test]
    fn test_backend_system() {
        let bm = BackendManager::new(BackendType::System, None);
        assert_eq!(bm.effective_backend(), BackendType::System);
        assert!(!bm.should_use_winuxcmd("ls"));
    }

    #[test]
    fn test_backend_winsh_commands() {
        let mut bm = BackendManager::new(BackendType::Auto, None);
        // Force winuxcmd available for testing
        bm.winuxcmd_available = true;
        bm.winuxcmd_path = Some(PathBuf::from("winuxcmd"));

        assert_eq!(bm.effective_backend(), BackendType::WinuxCmd);
        assert!(bm.should_use_winuxcmd("ls"));
        assert!(bm.should_use_winuxcmd("grep"));
        assert!(bm.should_use_winuxcmd("cat"));
        assert!(!bm.should_use_winuxcmd("git"));
        assert!(!bm.should_use_winuxcmd("docker"));
    }

    #[test]
    fn test_backend_describe() {
        let bm = BackendManager::new(BackendType::System, None);
        assert!(bm.describe().contains("System"));

        let bm = BackendManager::default();
        // Auto may resolve to WinuxCmd or System depending on availability
        assert!(!bm.describe().is_empty());
    }
}
