//! Process substitution support.
//!
//! Supports:
//! - <(command) - creates a named pipe/file with command output
//! - >(command) - creates a named pipe/file that feeds into command
//!
//! On Windows, this uses temporary files instead of named pipes
//! since Windows doesn't have POSIX named pipes.

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Child;

use winsh_core::ShellError;

/// Direction of process substitution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstDirection {
    /// <(command) - read from command output
    Input,
    /// >(command) - write to command input
    Output,
}

/// A process substitution.
pub struct ProcessSubst {
    /// The temporary file path
    path: PathBuf,
    /// The child process (if still running)
    child: Option<Child>,
    /// Direction
    direction: SubstDirection,
}

impl ProcessSubst {
    /// Create a new process substitution.
    pub fn new(
        command: &str,
        direction: SubstDirection,
    ) -> Result<Self, ShellError> {
        // Create a temporary file
        let tmp_dir = std::env::temp_dir();
        let tmp_file = tmp_dir.join(format!("winsh_proc_subst_{}", std::process::id()));
        let path = tmp_file.clone();

        match direction {
            SubstDirection::Input => {
                // Run the command and capture its output to the temp file
                let output = std::process::Command::new("cmd")
                    .args(["/C", command])
                    .output()
                    .map_err(|e| ShellError::Io(e))?;

                fs::write(&path, &output.stdout)
                    .map_err(|e| ShellError::Io(e))?;

                Ok(Self {
                    path,
                    child: None,
                    direction,
                })
            }
            SubstDirection::Output => {
                // Create an empty temp file for writing
                fs::write(&path, b"")
                    .map_err(|e| ShellError::Io(e))?;

                let child = std::process::Command::new("cmd")
                    .args(["/C", command])
                    .stdin(std::process::Stdio::from(
                        fs::File::open(&path).map_err(|e| ShellError::Io(e))?
                    ))
                    .spawn()
                    .map_err(|e| ShellError::Io(e))?;

                Ok(Self {
                    path,
                    child: Some(child),
                    direction,
                })
            }
        }
    }

    /// Get the path that can be used in the command line.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the path as a string.
    pub fn path_str(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    /// Wait for the substitution to complete (for output direction).
    pub fn wait(&mut self) -> Result<i32, ShellError> {
        if let Some(ref mut child) = self.child {
            match child.wait() {
                Ok(status) => Ok(status.code().unwrap_or(1)),
                Err(e) => Err(ShellError::Io(e)),
            }
        } else {
            Ok(0)
        }
    }

    /// Read the content of the substitution (for input direction).
    pub fn read_content(&self) -> Result<String, ShellError> {
        fs::read_to_string(&self.path)
            .map_err(|e| ShellError::Io(e))
    }
}

impl Drop for ProcessSubst {
    fn drop(&mut self) {
        // Clean up the temporary file
        let _ = fs::remove_file(&self.path);
        // Ensure child process is cleaned up
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
    }
}

/// Check if a string is a process substitution.
pub fn is_process_substitution(arg: &str) -> Option<SubstDirection> {
    if arg.starts_with("<(") && arg.ends_with(')') {
        Some(SubstDirection::Input)
    } else if arg.starts_with(">(") && arg.ends_with(')') {
        Some(SubstDirection::Output)
    } else {
        None
    }
}

/// Extract the command from a process substitution string.
pub fn extract_subst_command(arg: &str) -> Option<&str> {
    if arg.len() >= 4 {
        if arg.starts_with("<(") && arg.ends_with(')') {
            Some(&arg[2..arg.len()-1])
        } else if arg.starts_with(">(") && arg.ends_with(')') {
            Some(&arg[2..arg.len()-1])
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_process_substitution() {
        assert_eq!(is_process_substitution("<(ls -la)"), Some(SubstDirection::Input));
        assert_eq!(is_process_substitution(">(wc -l)"), Some(SubstDirection::Output));
        assert_eq!(is_process_substitution("normal_arg"), None);
    }

    #[test]
    fn test_extract_subst_command() {
        assert_eq!(extract_subst_command("<(ls -la)"), Some("ls -la"));
        assert_eq!(extract_subst_command(">(wc -l)"), Some("wc -l"));
        assert_eq!(extract_subst_command("normal_arg"), None);
    }
}
