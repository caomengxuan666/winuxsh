# WinSH - Windows Shell

[中文](README-zh.md) | English

A modern Unix-style command-line shell for Windows, written in Rust. WinSH provides a powerful shell experience with full compatibility with Windows commands and Unix-style tools.

## Features

### Core Functionality
- **860+ Command Completion**: Auto-discovery of commands from PATH
- **Command Completion** : via config file auto build command completion
- **Wildcard Expansion**: Full support for `*`, `?`, `[]` patterns
- **Command Substitution**: Execute commands within commands using `$(command)`
- **Script Execution**: Run `.sh` scripts with full shell support
- **History Management**: Browse command history with arrow keys

### Advanced Features
- **Array System**: Define, access, and manipulate arrays
- **Plugin Architecture**: Extensible plugin system for custom functionality
- **Theme Management**: 8 built-in themes with color customization
- **Environment Variables**: Full support for environment variable management
- **Emacs Mode**: Powerful keybindings for efficient editing

### Completion System
- **Flag Completion with Descriptions**: Tab-complete flags with inline usage hints (e.g. `--regexp   A pattern to search for.`) — see [TOML Definition Format](#completion-definition-files-toml-format)
- **Bash Script Auto-Import**: Scans `_cmd.bash` / `cmd.bash` files in completion dirs and parses them automatically — see [Bash Auto-Import](#bash-completion-script-auto-import)
- **Auto-Description Enrichment**: Runs `cmd -h` after first load to extract flag descriptions; persisted to cache — see [Auto-Description](#auto-description-enrichment-cmd--h)
- **Environment Variable Completion**: Type `$` to Tab-complete environment variables — see [Env Var Completion](#environment-variable-completion)
- **3-Layer Cache**: In-memory → disk (`.parsed.toml`) → subprocess, with mtime-based invalidation
- **Multiple Completion Dirs**: Configure multiple directories in `~/.winshrc.toml`
- **ListMenu Popup**: Floating completion menu with aligned descriptions

### Built-in Commands

| Command | Description |
|---------|-------------|
| `ls` | List directory contents |
| `cd` | Change directory |
| `pwd` | Print working directory |
| `echo` | Display text |
| `cat` | Display file contents |
| `grep` | Search text |
| `find` | Find files |
| `cp` | Copy files |
| `mv` | Move/rename files |
| `rm` | Remove files |
| `mkdir` | Create directories |
| `jobs` | List background jobs |
| `fg` / `bg` | Foreground / background job control |
| `set` / `unset` / `export` | Variable management |
| `alias` / `unalias` | Command aliases |
| `array` | Array operations |
| `plugin` | Plugin management |
| `theme` | Theme management |
| `history` | Command history |
| `source` | Execute script in current shell |

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/winuxsh.git
cd winuxsh

# Build release version
cargo build --release

# The executable will be at target/release/mvp6-array.exe
```

### Setup

1. Add the executable directory to your PATH
2. Configure utils backend (default: winuxcmd)
3. Configure Windows Terminal to use WinSH as default shell

### Utils Backend Configuration

WinSH supports multiple Unix utils backends:

**Default Backend (WinuxCmd)**:
```bash
# Already configured in utils/winuxcmd/
# Available commands: ls, cat, grep, find, cp, mv, rm, mkdir, etc.
```

**Switching Backends**:
```bash
# Edit ~/.winshrc.toml
[utils]
backend = "winuxcmd"  # or "uutils"
path = "utils/winuxcmd"
```

For more details, see `utils/README.md`.

## Usage

add this to windows terminal settings
replace the default profile with this one and $env:PATH to your PATH variable
```json
{
    "guid": "{9acb9455-ca63-5af2-ba0c-1fa3a891bd59}",
    "commandline":"${env:PATH}\\winuxsh.exe",
    "hidden": false,
    "name": "winuxsh",
}
```

### Interactive Mode

```bash
./winuxsh.exe
```

### Execute Single Command

```bash
./winuxsh.exe -c "echo Hello World"
```

### Execute Script

```bash
./winuxsh.exe script.sh
```

### Command Examples

```bash
# Wildcard expansion
ls *.rs
echo *.toml

# Command substitution
echo "Current user: $(whoami)"

# Array operations
array define colors red green blue
array get colors 0
array len colors

# Theme management
theme list
theme set cyberpunk

# Tab completion with descriptions
rg -<Tab>
# 0: --regexp       A pattern to search for.
# 1: --file         Search for patterns from the given file.
# 2: --after-context   Show NUM lines after each match.
# ...

# Environment variable completion
echo $WIN<Tab>
# → $WINDIR, $WINUXSH_*, ...
```

## Architecture

WinSH follows a modular architecture with clear separation of concerns:

```
src/
├── main.rs               # Entry point and REPL loop
├── shell.rs              # Shell state and execution
├── tokenizer.rs          # Lexical analysis
├── parser.rs             # Syntax analysis
├── executor.rs           # Command execution
├── builtins.rs           # Built-in commands
├── array.rs              # Array system
├── plugin.rs             # Plugin system
├── theme.rs              # Theme management
├── config.rs             # Configuration
├── job.rs                # Job control
├── error.rs              # Error handling
├── oh_my_winuxsh.rs      # Oh-My-Winuxsh plugin
└── completion/
    ├── mod.rs            # CompletionContext / CompletionResult
    ├── completer.rs      # WinuxshCompleter (reedline integration)
    ├── external.rs       # External command completion plugin (TOML + bash + cache)
    ├── bash_import.rs    # Bash completion script parser
    ├── command.rs        # Command name completion
    ├── path.rs           # Path completion
    └── variables.rs      # Environment variable completion
```


## Configuration

Configuration is stored in `~/.winshrc.toml`:

```toml
[shell]
prompt_format = "{user}@{host} {cwd} {symbol}"

[theme]
current_theme = "default"

[aliases]
ll = "ls -la"
la = "ls -a"

[completions]
# Multiple completion definition directories
completion_dirs = [
    "D:/shellTools/ripgrep/complete",
    "D:/shellTools/fd/autocomplete",
    "D:/shellTools/bat/autocomplete",
]
```

### Completion Definition Files (TOML Format)

Create `<command>.toml` inside any completion directory:

```toml
command = "mytool"
description = "My custom tool"

[[flags]]
short = "-v"
long = "--verbose"
description = "Enable verbose output"

[[flags]]
long = "--output"
description = "Output file path"
takes_value = true
values_from = "path"

[[flags]]
long = "--format"
description = "Output format"
takes_value = true
values = ["json", "yaml", "toml"]
```

### Bash Completion Script Auto-Import

At startup WinSH scans all configured completion directories for bash completion scripts (`_cmd.bash` / `cmd.bash`) and parses them automatically.

**How it works:**

1. Scan for `*.bash` files in each completion directory
2. Parse `opts="..."` fields to extract short (`-x`) and long (`--xxx`) flags
3. Serialize the result to `~/.winsh/completions/cache/<cmd>.parsed.toml` (invalidated when the bash file's mtime changes)
4. Subsequent starts read from cache — no re-parsing

**Where to get the scripts:** Most modern CLI tools (ripgrep, fd, bat, btm, …) ship a `complete/` or `autocomplete/` directory in their release archive containing bash completion scripts. Point `completion_dirs` at those directories.

> If both `rg.toml` and `_rg.bash` exist in a directory, the TOML file takes priority and the bash script is skipped.

### Auto-Description Enrichment (`cmd -h`)

Bash scripts carry no description text. After loading all definitions WinSH automatically runs `cmd -h` for every command that has flags without descriptions.

**How it works:**

1. After all completion definitions are loaded, run `cmd -h` for each command missing descriptions
2. Parse help output — flag lines are identified by the following format:
   ```
     -s, --case-sensitive             Description text
         --long-only                  Description text
     -e, --regexp=PATTERN             Description text
   ```
   Two or more consecutive spaces separate the flag name(s) from the description.
3. Write extracted descriptions into `FlagDef.description`
4. **Persist to cache**: overwrite the `.parsed.toml` with the enriched definitions — next start reads from cache without re-running `cmd -h`

### Environment Variable Completion

Type a `$` prefix and press Tab to complete environment variables:

```bash
$ echo $PATH<Tab>
$ echo $HOME<Tab>
$ echo $USERPROFILE<Tab>

# Partial match also works
$ echo $WIN<Tab>
# → $WINDIR, $WINUXSH_*, ...
```

Variables set via `export` / `set` as well as system environment variables are all available for completion.

## Theme System

WinSH includes 8 built-in themes:
- `default` - Classic green/blue theme
- `dark` - Minimal dark theme
- `light` - Light color theme
- `colorful` - Vibrant colors
- `minimal` - Plain text
- `cyberpunk` - Neon colors
- `ocean` - Blue tones
- `forest` - Green tones

## Plugin System

WinSH supports a plugin system for extending functionality:

### Built-in Plugins
- **Welcome Plugin**: Displays welcome message on startup
- **Oh-My-Winuxsh**: Theme and plugin management

### Creating Plugins

Implement the `Plugin` trait:

```rust
pub trait Plugin {
    fn name(&self) -> &str;
    fn init(&mut self) -> Result<()>;
    fn execute(&self, args: &[String], shell: &mut Shell) -> Result<bool>;
    fn description(&self) -> &str;
}
```

## Compatibility

- **OS**: Windows 10/11
- **Rust**: 2021 edition
- **Terminal**: Windows Terminal recommended
- **Architecture**: x64

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```


## Performance

WinSH features intelligent command routing with WinuxCmd DLL integration for optimal performance.

### Command Routing Priority

Commands are routed based on priority:
1. **Built-in Commands** - Native WinSH commands (fastest)
2. **WinuxCmd DLL** - Unix tools via DLL (very fast)
3. **PATH Execution** - External executables (standard performance)

### Performance Benchmarks

Testing results comparing WinuxCmd DLL vs PATH execution:

**Single Execution (with shell startup overhead):**
- WinuxCmd DLL: 28.4ms
- PATH Execution: 55.3ms
- **DLL Speedup: 49% faster**

**Batch Execution (10 commands):**
- WinuxCmd DLL: 4.6ms per command
- PATH Execution: 31.7ms per command
- **DLL Speedup: ~7x faster**

### Performance Advantages

- **DLL Integration**: Direct DLL calls avoid process creation overhead
- **Efficient FFI**: Foreign function interface minimizes overhead
- **Smart Routing**: Automatic command classification ensures optimal execution path
- **Memory Efficiency**: Shared DLL reduces memory usage

### Daemon Management

WinSH automatically manages the WinuxCmd daemon:
- Auto-starts daemon if not running
- Persists across shell sessions
- Multiple shell instances share the same daemon
- No manual configuration required
## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License - see LICENSE file for details

## Acknowledgments

- **reedline**: Line editing library by Nushell
- **winuxcmd**: Unix-style tools for Windows
- **colored**: Terminal color support

## Version History

### MVP6 (Current)
- Array support
- Plugin system
- Theme management
- 860+ command completion
- Full wildcard expansion
- Command substitution
- Script execution
- TOML-driven external command completion
- Bash completion script auto-import
- Flag descriptions from `cmd -h` with disk cache
- ListMenu popup with aligned descriptions
- Multi-directory completion config

### MVP5
- Job control
- Pipeline support
- Vi mode basics

### MVP4
- Basic shell functionality
- File operations
- Command execution

## Support

For issues and questions:
- GitHub Issues: https://github.com/caomengxuan666/winuxsh/issues
- Documentation: See inline code documentation

## Roadmap

### MVP7 (Planned)
- Vi mode editing
- History search (Ctrl+R)
- Smart completion
- Pipeline improvements
- Background job control

### Future
- Cross-platform support (Linux, macOS)
- More plugins
- Advanced scripting features
- Performance optimizations
