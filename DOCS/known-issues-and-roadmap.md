# WinSH Known Issues and Roadmap

## Overview

This document tracks current issues, bugs, and future improvements for WinSH (Windows Shell) project.

**Last Updated**: 2026-04-01  
**Version**: MVP6 (0.6.0)

---

## 🔴 Critical Issues

### 1. Broken Pipeline Functionality

**Severity**: Critical  
**Status**: Open  
**Priority**: P0

**Description**:
Pipeline operations do not work correctly. When piping output from one command to another, the entire output is displayed instead of being filtered/processed.

**Examples**:
```bash
# Expected: Display first 3 lines
# Actual: Displays entire file
cat file.txt | head -n 3

# Expected: Count lines
# Actual: Displays all output then counts
dir | wc -l
```

**Current Workaround**:
Using temporary files for pipe buffering in `src/shell.rs`:
```rust
let temp_pipe_file = format!("winuxsh_pipe_{}.tmp", std::process::id());
```

**Root Cause**:
- Windows process creation and pipe handling differs from Unix
- Current implementation does not properly connect stdin/stdout between processes
- May need to use Windows anonymous pipes or named pipes

**Proposed Solution**:
1. Implement proper Windows anonymous pipes using `CreatePipe`
2. Connect child process stdin/stdout to pipe handles
3. Handle pipe buffering and synchronization correctly
4. Test with various pipe combinations

**Affected Files**:
- `src/executor.rs` - External command execution
- `src/shell.rs` - Pipeline orchestration

---

### 2. Broken Output Redirection

**Severity**: Critical  
**Status**: Open  
**Priority**: P0

**Description**:
Output redirection operators (`>`, `>>`) do not work correctly. Output is displayed in terminal instead of being written to file.

**Examples**:
```bash
# Expected: Write "test" to file.txt (no terminal output)
# Actual: Displays "test" in terminal, file not created/updated
echo "test" > file.txt

# Expected: Append to file
# Actual: Displays in terminal
echo "more" >> file.txt
```

**Current Behavior**:
Redirection directives are parsed but not properly applied during command execution.

**Root Cause**:
- Stdio redirection not properly configured for child processes
- File handles not correctly passed to subprocess
- May be related to pipeline issues (both involve stdio manipulation)

**Proposed Solution**:
1. Ensure stdout/stderr handles are properly configured before spawning processes
2. Use `Stdio::from()` for file-based redirection
3. Test all redirection types: `>`, `>>`, `2>`, `<`

**Affected Files**:
- `src/executor.rs` - Stdio configuration
- `src/tokenizer.rs` - Redirection parsing
- `src/shell.rs` - Redirection application

---

## 🟡 Medium Priority Issues

### 3. Command Compatibility Issues

**Severity**: Medium  
**Status**: Needs Investigation  
**Priority**: P1

**Description**:
Some commands may have compatibility issues or unexpected behavior when executed through WinSH.

**Known Problematic Commands**:
- `grep` - May hang or behave unexpectedly with certain patterns
- `find` - May have issues with complex expressions
- Interactive commands (vi, less, more) - TTY handling may be imperfect

**Current Workaround**:
Forcing input-waiting commands to use PATH execution instead of DLL:
```rust
let input_waiting_commands = vec![
    "grep", "sed", "awk", "perl", "python", "ruby",
    "less", "more", "vi", "vim", "nano", "ed", "emacs"
];
```

**Investigation Needed**:
- Test each problematic command individually
- Determine if issues are with DLL integration or command routing
- Check if proper stdin/stdout/stderr handling is implemented

**Affected Files**:
- `src/command_router.rs` - Command routing logic
- `src/winuxcmd_ffi.rs` - DLL execution
- `src/executor.rs` - PATH execution

---

### 4. Configuration File Path Issues

**Severity**: Medium  
**Status**: Needs Verification  
**Priority**: P1

**Description**:
Root directory configuration loading may have path resolution issues.

**Symptoms**:
- `.winshrc.toml` may not be loaded from expected location
- Configuration changes may not take effect
- Default configuration may be used instead of user config

**Expected Behavior**:
- Load `.winshrc.toml` from user home directory
- Fall back to working directory if not found
- Use default configuration if no config file exists

**Investigation Needed**:
- Verify config file search path logic
- Test with config files in different locations
- Check environment variable expansion in config

**Affected Files**:
- `src/config.rs` - Configuration loading
- `src/shell.rs` - Config initialization
- `.winshrc.toml` - Configuration file

---

## 🟢 Low Priority Issues

### 5. Compiler Warnings

**Severity**: Low  
**Status**: Cosmetic  
**Priority**: P2

**Description**:
37 compiler warnings during build. These do not affect functionality but should be cleaned up for code quality.

**Warning Categories**:
- Unused imports (5 warnings)
- Unused variables (3 warnings)
- Irrefutable if-let patterns (2 warnings)
- Unnecessary unsafe blocks (2 warnings)
- Unused mut (2 warnings)
- Shared reference to mutable static (1 warning)
- Other miscellaneous warnings (22 warnings)

**Examples**:
```rust
// Warning: unused import
use std::sync::Mutex;

// Warning: unused variable
let mut shell = ...

// Warning: unnecessary unsafe block
unsafe { FFI_LIBRARY.is_some() }
```

**Proposed Solution**:
1. Remove unused imports
2. Remove or use unused variables
3. Replace irrefutable if-let with direct assignment
4. Remove unnecessary unsafe blocks
5. Use cargo fix: `cargo fix --bin "winuxsh" -p winuxsh`

**Impact**:
- Cosmetic only, no functional impact
- Improves code quality and maintainability

---

### 6. DLL Path Warning on Startup

**Severity**: Low  
**Status**: Cosmetic  
**Priority**: P2

**Description**:
Warning message displayed on every startup about `winuxcmd.dll` not found in development path.

**Warning Message**:
```
warning: winuxsh@0.6.0: winuxcmd.dll not found at utils/winuxcmd/winuxcmd.dll for development
```

**Current Behavior**:
- DLL is found and works correctly at runtime
- Warning appears during build.rs execution
- Does not affect functionality

**Root Cause**:
`build.rs` checks for DLL at hardcoded development path, but DLL may be in system path or runtime location.

**Proposed Solution**:
1. Make DLL path check optional or less strict
2. Improve error message to clarify that DLL may be in system path
3. Add proper runtime DLL search logic
4. Consider using rpath or manifest for DLL loading

**Impact**:
- Cosmetic only, DLL works correctly
- Improves user experience (no confusing warnings)

---

## ✅ Recently Resolved Issues

### 7. Ctrl+C Killing Entire Shell - FIXED ✓

**Severity**: Critical (Previously)  
**Status**: Resolved  
**Resolution Date**: 2026-04-01

**Description**:
When user pressed Ctrl+C during command execution (e.g., `grep .`), the entire shell would terminate with error code `3221225786 (0xc000013a - STATUS_CONTROL_C_EXIT)`.

**Solution Implemented**:
- Added Windows console control handler using Win32 API
- Installed custom Ctrl+C handler to intercept signals
- Track child process PID during command execution
- Terminate only child process on Ctrl+C, preserve shell session
- Added `windows-sys` dependency for Windows API bindings

**Technical Details**:
```rust
// Win32 API signal handler
unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> BOOL {
    match ctrl_type {
        CTRL_C_EVENT => {
            if CURRENT_CHILD_PID != 0 {
                // Terminate child only, not shell
                TerminateProcess(handle, 1);
                return 1; // Signal handled
            }
            return 0;
        }
        _ => 0,
    }
}
```

**Commit**: `1f92036ff429747f56bafb1e7aefd84a12bfe8f2`

---

### 8. ANSI Color Codes Lost in DLL Output - FIXED ✓

**Severity**: Medium (Previously)  
**Status**: Resolved  
**Resolution Date**: 2026-03-31

**Description**:
When executing commands via DLL, ANSI color codes were stripped from output, resulting in plain text instead of colored output.

**Solution Implemented**:
- Changed response type from `String` to `Vec<u8>` in `WinuxCmdResponse`
- Preserve raw bytes from DLL output
- Properly handle UTF-8 encoding and ANSI escape sequences

**Affected Files**:
- `src/winuxcmd_ffi.rs` - Response type change
- `src/shell.rs` - Output handling

---

### 9. Command Routing Performance - OPTIMIZED ✓

**Severity**: Enhancement  
**Status**: Resolved  
**Resolution Date**: 2026-03-31

**Description**:
Implemented intelligent command routing with significant performance improvements.

**Results**:
- WinuxCmd DLL execution: 4.6ms per command
- PATH execution: 31.7ms per command
- DLL speedup: ~7x faster for batch operations
- Single execution: 49% faster with DLL

**Features Implemented**:
- Command classification system (137 commands)
- Priority routing: builtin > DLL > PATH
- Interactive command detection for TTY handling
- Fallback mechanism for failed DLL calls

---

## 📋 Future Enhancements

### Performance Improvements

1. **HashSet for Command Lookups**
   - Replace linear Vec search with HashSet
   - Expected speedup: O(n) → O(1)
   - Priority: P1

2. **Lazy Command Classification Loading**
   - Load classification config only when needed
   - Reduce startup time
   - Priority: P2

### Feature Additions

3. **Proper Pipeline Implementation**
   - Use Windows anonymous pipes
   - Support multi-command pipelines
   - Priority: P0 (Critical)

4. **Output Redirection Support**
   - Implement `>`, `>>`, `2>`, `<` operators
   - Proper stdio handle configuration
   - Priority: P0 (Critical)

5. **Command Substitution**
   - Support `$(command)` syntax
   - Nested command execution
   - Priority: P1

6. **Glob Pattern Expansion**
   - Support `*`, `?`, `[...]` patterns
   - Shell-level expansion
   - Priority: P1

7. **Job Control**
   - Background job management (`&`)
   - Job control commands (`jobs`, `fg`, `bg`)
   - Priority: P1

8. **Array Operations**
   - Array indexing and slicing
   - Array expansion in commands
   - Priority: P2

### UX Improvements

9. **Tab Completion Enhancement**
   - Context-aware suggestions
   - Command option completion
   - Priority: P2

10. **Syntax Highlighting**
    - Colorize command input
    - Highlight special characters
    - Priority: P2

11. **Better Error Messages**
    - User-friendly error descriptions
    - Suggested fixes
    - Priority: P2

### Testing

12. **Comprehensive Test Suite**
    - Unit tests for all modules
    - Integration tests for shell behavior
    - Priority: P1

13. **Automated Regression Testing**
    - CI/CD integration
    - Performance benchmarking
    - Priority: P1

---

## 📊 Issue Statistics

| Category | Count | Priority |
|----------|-------|----------|
| Critical | 2 | P0 |
| Medium | 2 | P1 |
| Low | 2 | P2 |
| Resolved | 3 | N/A |
| Future | 11 | Various |

---

## 🔄 Maintenance Guidelines

### Issue Reporting

When reporting new issues, include:
1. **Description**: Clear explanation of the problem
2. **Severity**: Critical/Medium/Low
3. **Reproduction Steps**: How to trigger the issue
4. **Expected Behavior**: What should happen
5. **Actual Behavior**: What actually happens
6. **Environment**: Windows version, WinSH version
7. **Examples**: Code snippets or command examples

### Issue Resolution

When resolving issues:
1. Update this document with resolution details
2. Reference commit hash where fix was applied
3. Move issue from "Open" to "Resolved" section
4. Document any breaking changes
5. Update test coverage if needed

### Document Maintenance

- Update "Last Updated" date on any changes
- Keep issue counts accurate
- Review and prioritize monthly
- Archive resolved issues older than 6 months

---

## 📞 Contact

For questions or issues:
- GitHub Issues: https://github.com/caomengxuan666/winuxsh/issues
- Documentation: See README.md and AGENTS.md

---

**Document Version**: 1.0  
**Maintainer**: WinSH Development Team