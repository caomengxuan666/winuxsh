//! Security utilities for the shell.
//!
//! Provides input validation, path sanitization, and permission checks.

use std::path::Path;
use winsh_core::ShellError;

/// Validate a filename or path for security concerns.
pub fn validate_path(path: &str) -> Result<(), ShellError> {
    if path.contains('\0') {
        return Err(ShellError::ShellError("path contains null byte".to_string()));
    }
    if path.len() > 32768 {
        return Err(ShellError::ShellError("path too long".to_string()));
    }
    Ok(())
}

/// Check if a path tries to escape its base directory.
pub fn is_path_traversal(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    let normalized = path_str.replace("\\", "/");
    let parts: Vec<&str> = normalized.split('/').collect();

    let mut depth: i32 = 0;
    for part in &parts {
        match *part {
            "" | "." => continue,
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            _ => depth += 1,
        }
    }
    false
}

/// Sanitize a path to prevent directory traversal.
pub fn sanitize_path(path: &str) -> String {
    let is_absolute = path.starts_with('/') || path.starts_with('\\');
    let normalized = path.replace("\\", "/");
    let parts: Vec<&str> = normalized.split('/').collect();
    let mut result: Vec<&str> = Vec::new();

    for part in parts {
        match part {
            "" | "." => continue,
            ".." => {
                if !result.is_empty() {
                    result.pop();
                }
            }
            _ => result.push(part),
        }
    }

    let joined = result.join("/");
    if is_absolute {
        format!("/{}", joined)
    } else {
        joined
    }
}

/// Validate shell input for potentially dangerous patterns.
pub fn validate_input(input: &str) -> Result<(), ShellError> {
    if input.contains('\0') {
        return Err(ShellError::ShellError("input contains null byte".to_string()));
    }
    if input.len() > 1048576 {
        return Err(ShellError::ShellError("input too long".to_string()));
    }
    Ok(())
}

/// Check if a config file has secure permissions.
pub fn check_config_permissions(path: &Path) -> Result<(), ShellError> {
    if !path.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mode = metadata.permissions().mode();
            if mode & 0o002 != 0 {
                eprintln!("warning: {} is world-writable", path.display());
            }
        }
    }
    Ok(())
}

/// Secure version of command building that validates inputs.
pub fn secure_command_args(args: &[&str]) -> Result<Vec<String>, ShellError> {
    let mut validated = Vec::new();
    for arg in args {
        validate_input(arg)?;
        validated.push(arg.to_string());
    }
    Ok(validated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_good() {
        assert!(validate_path("/usr/bin/ls").is_ok());
        assert!(validate_path("C:\\Windows\\System32").is_ok());
    }

    #[test]
    fn test_validate_path_null() {
        assert!(validate_path("path\0withnull").is_err());
    }

    #[test]
    fn test_sanitize_path() {
        assert_eq!(sanitize_path("/a/b/../c"), "/a/c");
        assert_eq!(sanitize_path("/a/./b"), "/a/b");
        assert_eq!(sanitize_path("/a/b/c"), "/a/b/c");
        assert_eq!(sanitize_path("a/b/c"), "a/b/c");
    }

    #[test]
    fn test_is_path_traversal() {
        assert!(is_path_traversal(Path::new("../etc/passwd")));
        assert!(is_path_traversal(Path::new("a/../../b")));
        assert!(!is_path_traversal(Path::new("a/b/c")));
        assert!(!is_path_traversal(Path::new("a/b")));
    }

    #[test]
    fn test_validate_input() {
        assert!(validate_input("echo hello").is_ok());
        assert!(validate_input("test\0input").is_err());
    }
}
