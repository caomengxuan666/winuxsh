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
//!
//! Installation:
//!   Run `winsh --install-backend` to auto-download winuxcmd/uutils
//!   Or manually: `pwsh -File tools/install-backend.ps1`

use std::path::PathBuf;
use std::process::Command;

use winsh_core::{BackendType, ShellError, ShellState};

pub struct BackendManager {
    backend_type: BackendType,
    winuxcmd_path: Option<PathBuf>,
    winuxcmd_available: bool,
}

fn find_in_path(cmd: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let full_path = dir.join(cmd);
        if cfg!(windows) {
            for ext in &["", ".exe", ".cmd", ".bat"] {
                let with_ext = if ext.is_empty() { full_path.clone() }
                    else { full_path.with_extension(&ext[1..]) };
                if with_ext.exists() { return Some(with_ext); }
            }
        } else if full_path.exists() {
            return Some(full_path);
        }
    }
    None
}

impl BackendManager {
    pub fn new(backend_type: BackendType, winuxcmd_path: Option<PathBuf>) -> Self {
        let mut manager = Self { backend_type, winuxcmd_path: winuxcmd_path.clone(), winuxcmd_available: false };
        manager.detect();
        manager
    }

    fn detect(&mut self) {
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let paths = vec![
            self.winuxcmd_path.clone().unwrap_or_else(|| PathBuf::from("winuxcmd")),
            PathBuf::from("winuxcmd.exe"), PathBuf::from("uutils.exe"), PathBuf::from("coreutils.exe"),
            PathBuf::from(&local_appdata).join("WinSH").join("winuxcmd").join("winuxcmd.exe"),
            PathBuf::from(&local_appdata).join("WinuxCmd").join("winuxcmd.exe"),
        ];
        for p in &paths {
            if p.exists() { self.winuxcmd_available = true; self.winuxcmd_path = Some(p.clone()); return; }
        }
        for name in &["winuxcmd", "uutils", "coreutils"] {
            if let Some(p) = find_in_path(name) { self.winuxcmd_available = true; self.winuxcmd_path = Some(p); return; }
        }
    }

    pub fn effective_backend(&self) -> BackendType {
        match self.backend_type {
            BackendType::Auto => if self.winuxcmd_available { BackendType::WinuxCmd } else { BackendType::System },
            other => other,
        }
    }

    pub fn should_use_winuxcmd(&self, cmd: &str) -> bool {
        if self.effective_backend() != BackendType::WinuxCmd { return false; }
        const CMDS: &[&str] = &["ls","cat","cp","mv","rm","mkdir","rmdir","touch","head","tail","wc","sort","uniq","cut","paste","tr","sed","grep","find","xargs","tee","basename","dirname","chmod","chown","ln","du","df","echo","printf","date","sleep","true","false","test","[","pwd","whoami","id","env","envsubst","seq","yes","dd","shuf","shred","fmt","fold","nl","cksum","hashsum","md5sum","sha1sum","sha256sum","split","join","comm"];
        CMDS.contains(&cmd)
    }

    pub fn execute_winuxcmd(&self, cmd: &str, args: &[&str], state: &ShellState) -> Result<i32, ShellError> {
        let winuxcmd = self.winuxcmd_path.as_ref().ok_or_else(|| ShellError::command_not_found("winuxcmd"))?;
        let mut command = Command::new(winuxcmd);
        command.arg(cmd).args(args);
        for (k, v) in state.env.exported() { command.env(k, v); }
        command.current_dir(state.current_dir());
        #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
        command.stdin(std::process::Stdio::inherit()).stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit());
        let status = command.spawn().and_then(|mut c| c.wait()).map_err(|e| ShellError::command_not_found(format!("winuxcmd: {}", e)))?;
        Ok(status.code().unwrap_or(1))
    }

    pub fn install_backend(&self) -> Result<String, ShellError> {
        let script_paths = vec![PathBuf::from("tools/install-backend.ps1"), PathBuf::from("../tools/install-backend.ps1")];
        let script = script_paths.iter().find(|p| p.exists()).ok_or_else(|| ShellError::ShellError("install script not found. Use: scoop install uutils-coreutils".to_string()))?;
        let output = Command::new("pwsh").args(["-File", &script.to_string_lossy()]).output().map_err(|e| ShellError::ShellError(format!("failed: {}", e)))?;
        if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).to_string()) }
        else { Err(ShellError::ShellError(format!("install failed: {}", String::from_utf8_lossy(&output.stderr)))) }
    }

    pub fn needs_install(&self) -> bool { !self.winuxcmd_available }
    pub fn wlinuxcmd_path(&self) -> Option<&PathBuf> { self.winuxcmd_path.as_ref() }
    pub fn is_winuxcmd_available(&self) -> bool { self.winuxcmd_available }
    pub fn backend_type(&self) -> BackendType { self.backend_type }

    pub fn describe(&self) -> String {
        match self.effective_backend() {
            BackendType::System if self.winuxcmd_available => "System (WinuxCmd available)".to_string(),
            BackendType::System => "System PATH".to_string(),
            BackendType::WinuxCmd => format!("WinuxCmd ({})", self.winuxcmd_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "not found".to_string())),
            BackendType::Auto => if self.winuxcmd_available { "Auto (WinuxCmd)".to_string() } else { "Auto (System)".to_string() },
        }
    }
}

impl Default for BackendManager {
    fn default() -> Self { Self::new(BackendType::Auto, None) }
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
        let bm = BackendManager { backend_type: BackendType::Auto, winuxcmd_available: true, winuxcmd_path: Some(PathBuf::from("winuxcmd")) };
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
        assert!(!bm.describe().is_empty());
    }
}
