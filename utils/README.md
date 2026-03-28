# Utils Backend Management

This directory contains Unix-style utility backends for WinSH.

## Available Backends

### WinuxCmd (Default)
- **Location**: `utils/winuxcmd/`
- **Description**: C++23 implementation of Unix tools
- **Size**: ~1.7MB
- **Included Tools**: ls, cat, grep, find, cp, mv, rm, mkdir, touch, date, head, tail, wc, chmod, chown, and 100+ more

### UUtils (Future)
- **Location**: `utils/uutils/` (to be added)
- **Description**: Rust implementation of Unix utilities
- **Size**: Varies
- **Status**: Available for future integration

## Switching Backends

### Method 1: Configuration File
Edit `~/.winshrc.toml`:
```toml
[utils]
backend = "winuxcmd"  # or "uutils"
path = "utils/winuxcmd"
```

### Method 2: Environment Variable
```bash
export WINUX_UTILS_BACKEND="winuxcmd"
export WINUX_UTILS_PATH="utils/winuxcmd"
```

### Method 3: Direct Replacement
Simply replace the files in the active backend directory with your preferred utils backend.

## Adding New Backends

To add a new utils backend:

1. Create a new directory: `utils/your-backend/`
2. Place your utils executable and scripts
3. Update `~/.winshrc.toml` to use the new backend
4. Test with basic commands: `ls`, `cat`, `grep`, etc.

## Current Active Backend
- **Backend**: WinuxCmd
- **Version**: 0.5.2
- **Status**: Default

## Backend Comparison

| Backend | Language | Size | Performance | Compatibility |
|---------|----------|------|-------------|---------------|
| WinuxCmd | C++23   | 1.7MB | Fast        | Windows       |
| UUtils   | Rust    | TBD   | Fast        | Cross-platform|

## Notes
- WinSH will automatically add the active backend directory to PATH
- Backend switching requires restarting the shell
- Some backends may have different command behaviors
- File paths are handled consistently across backends