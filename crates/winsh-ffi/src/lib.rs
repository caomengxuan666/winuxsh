//! FFI bindings for optimized command execution.
//!
//! This module reserves the FFI interface for future use with
//! winuxcmd DLL or other native libraries for performance-critical
//! commands (ls, grep, cat, find, etc.).
//!
//! The FFI system will be used to bypass process spawning overhead
//! for commonly-used commands by calling the implementations directly.
//!
//! Architecture:
//!   Shell → FFI Router → winuxcore.dll (or other native lib)
//!                     → Parse results directly in Rust
//!
//! This provides:
//!   - Lower latency (no process spawn overhead)
//!   - Rich structured output
//!   - Better pipe integration

use std::path::PathBuf;

/// FFI library handle.
pub struct FfiLibrary {
    /// Path to the native library
    path: PathBuf,
    /// Whether the library is loaded
    loaded: bool,
}

impl FfiLibrary {
    /// Create a new FFI library reference.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            loaded: false,
        }
    }

    /// Check if the FFI library is available.
    pub fn is_available(&self) -> bool {
        self.path.exists()
    }

    /// Load the native library (future implementation).
    pub fn load(&mut self) -> Result<(), String> {
        if !self.is_available() {
            return Err(format!("FFI library not found: {}", self.path.display()));
        }
        // TODO: Use libloading to dynamically load the DLL
        // unsafe {
        //     let lib = libloading::Library::new(&self.path)?;
        //     // Load function pointers...
        // }
        self.loaded = true;
        Ok(())
    }

    /// Get the path to the library.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// A command that can be executed via FFI.
#[derive(Debug, Clone)]
pub struct FfiCommand {
    /// The command name (e.g., "ls", "grep")
    pub name: String,
    /// The function exported by the DLL
    pub export_name: String,
    /// Whether this command supports structured output
    pub structured: bool,
}

/// List of commands that are planned to be supported via FFI.
pub fn planned_ffi_commands() -> Vec<FfiCommand> {
    vec![
        FfiCommand { name: "ls".into(), export_name: "winsh_ls".into(), structured: true },
        FfiCommand { name: "cat".into(), export_name: "winsh_cat".into(), structured: false },
        FfiCommand { name: "grep".into(), export_name: "winsh_grep".into(), structured: true },
        FfiCommand { name: "find".into(), export_name: "winsh_find".into(), structured: true },
        FfiCommand { name: "wc".into(), export_name: "winsh_wc".into(), structured: true },
        FfiCommand { name: "head".into(), export_name: "winsh_head".into(), structured: false },
        FfiCommand { name: "tail".into(), export_name: "winsh_tail".into(), structured: false },
        FfiCommand { name: "sort".into(), export_name: "winsh_sort".into(), structured: false },
        FfiCommand { name: "date".into(), export_name: "winsh_date".into(), structured: true },
        FfiCommand { name: "du".into(), export_name: "winsh_du".into(), structured: true },
        FfiCommand { name: "df".into(), export_name: "winsh_df".into(), structured: true },
        FfiCommand { name: "stat".into(), export_name: "winsh_stat".into(), structured: true },
    ]
}
