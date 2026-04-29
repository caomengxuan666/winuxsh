//! Core built-in commands.

use winsh_core::{ShellError, ShellState};
use crate::Builtin;

/// `echo` command - display text
pub struct EchoBuiltin;

impl Builtin for EchoBuiltin {
    fn name(&self) -> &str {
        "echo"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        let mut no_newline = false;
        let mut enable_escape = false;
        let mut start_idx = 0;

        // Parse options
        for (i, arg) in args.iter().enumerate() {
            match *arg {
                "-n" => {
                    no_newline = true;
                    start_idx = i + 1;
                }
                "-e" => {
                    enable_escape = true;
                    start_idx = i + 1;
                }
                "-E" => {
                    enable_escape = false;
                    start_idx = i + 1;
                }
                _ if arg.starts_with('-') => {
                    // Unknown option, stop parsing
                    break;
                }
                _ => break,
            }
        }

        let output: Vec<&str> = args[start_idx..].to_vec();
        let text = output.join(" ");

        if enable_escape {
            print!("{}", process_escapes(&text));
        } else {
            print!("{}", text);
        }

        if !no_newline {
            println!();
        }

        Ok(0)
    }

    fn help(&self) -> &str {
        "echo [-neE] [arg ...]
Display the ARGs, separated by a space and followed by a newline.

Options:
  -n    do not append a newline
  -e    enable interpretation of backslash escapes
  -E    disable interpretation of backslash escapes (default)"
    }
}

/// `printf` command - formatted output
pub struct PrintfBuiltin;

impl Builtin for PrintfBuiltin {
    fn name(&self) -> &str {
        "printf"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            return Err(ShellError::message("printf: format string required"));
        }

        let format = args[0];
        let mut arg_idx = 1;
        let mut result = String::new();
        let mut chars = format.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '%' {
                match chars.next() {
                    Some('%') => result.push('%'),
                    Some('s') => {
                        if let Some(arg) = args.get(arg_idx) {
                            result.push_str(arg);
                            arg_idx += 1;
                        }
                    }
                    Some('d') | Some('i') => {
                        if let Some(arg) = args.get(arg_idx) {
                            match arg.parse::<i64>() {
                                Ok(n) => result.push_str(&n.to_string()),
                                Err(_) => result.push_str("0"),
                            }
                            arg_idx += 1;
                        }
                    }
                    Some('f') => {
                        if let Some(arg) = args.get(arg_idx) {
                            match arg.parse::<f64>() {
                                Ok(f) => result.push_str(&format!("{}", f)),
                                Err(_) => result.push_str("0.000000"),
                            }
                            arg_idx += 1;
                        }
                    }
                    Some('c') => {
                        if let Some(arg) = args.get(arg_idx) {
                            if let Some(c) = arg.chars().next() {
                                result.push(c);
                            }
                            arg_idx += 1;
                        }
                    }
                    Some('x') => {
                        if let Some(arg) = args.get(arg_idx) {
                            match arg.parse::<i64>() {
                                Ok(n) => result.push_str(&format!("{:x}", n)),
                                Err(_) => result.push_str("0"),
                            }
                            arg_idx += 1;
                        }
                    }
                    Some('X') => {
                        if let Some(arg) = args.get(arg_idx) {
                            match arg.parse::<i64>() {
                                Ok(n) => result.push_str(&format!("{:X}", n)),
                                Err(_) => result.push_str("0"),
                            }
                            arg_idx += 1;
                        }
                    }
                    Some('o') => {
                        if let Some(arg) = args.get(arg_idx) {
                            match arg.parse::<i64>() {
                                Ok(n) => result.push_str(&format!("{:o}", n)),
                                Err(_) => result.push_str("0"),
                            }
                            arg_idx += 1;
                        }
                    }
                    Some(c) => {
                        result.push('%');
                        result.push(c);
                    }
                    None => {
                        result.push('%');
                    }
                }
            } else if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some(c) => {
                        result.push('\\');
                        result.push(c);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(c);
            }
        }

        print!("{}", result);
        Ok(0)
    }

    fn help(&self) -> &str {
        "printf FORMAT [ARG ...]
Display ARGs according to FORMAT.

Format specifiers:
  %s    string
  %d    integer
  %f    float
  %c    character
  %x    hexadecimal (lowercase)
  %X    hexadecimal (uppercase)
  %o    octal
  %%    literal percent"
    }
}

/// `read` command - read input
pub struct ReadBuiltin;

impl Builtin for ReadBuiltin {
    fn name(&self) -> &str {
        "read"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        let mut prompt = String::new();
        let mut var_name = "REPLY".to_string();
        let mut start_idx = 0;

        // Parse options
        for (i, arg) in args.iter().enumerate() {
            match *arg {
                "-p" => {
                    if let Some(p) = args.get(i + 1) {
                        prompt = p.to_string();
                        start_idx = i + 2;
                    }
                }
                "-r" => {
                    // Raw mode - don't interpret backslashes
                    start_idx = i + 1;
                }
                _ if arg.starts_with('-') => {
                    // Unknown option
                    start_idx = i + 1;
                }
                _ => {
                    var_name = arg.to_string();
                    start_idx = i + 1;
                    break;
                }
            }
        }

        // Print prompt if specified
        if !prompt.is_empty() {
            print!("{}", prompt);
            use std::io::Write;
            std::io::stdout().flush().unwrap_or(());
        }

        // Read input
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(0) => return Ok(1), // EOF
            Ok(_) => {
                // Remove trailing newline
                if input.ends_with('\n') {
                    input.pop();
                    if input.ends_with('\r') {
                        input.pop();
                    }
                }
                // TODO: Set the variable in the shell state
                println!("{}={}", var_name, input);
                Ok(0)
            }
            Err(e) => Err(ShellError::Io(e)),
        }
    }

    fn help(&self) -> &str {
        "read [-r] [-p prompt] [name]
Read a line from standard input.

Options:
  -r    raw mode - do not interpret backslashes
  -p    display prompt before reading"
    }
}

/// `true` command - always return success
pub struct TrueBuiltin;

impl Builtin for TrueBuiltin {
    fn name(&self) -> &str {
        "true"
    }

    fn execute(&self, _args: &[&str]) -> Result<i32, ShellError> {
        Ok(0)
    }

    fn help(&self) -> &str {
        "true
Return a successful result."
    }
}

/// `false` command - always return failure
pub struct FalseBuiltin;

impl Builtin for FalseBuiltin {
    fn name(&self) -> &str {
        "false"
    }

    fn execute(&self, _args: &[&str]) -> Result<i32, ShellError> {
        Ok(1)
    }

    fn help(&self) -> &str {
        "false
Return an unsuccessful result."
    }
}

/// `test` command - evaluate conditional expressions
pub struct TestBuiltin;

impl Builtin for TestBuiltin {
    fn name(&self) -> &str {
        "test"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            return Ok(1); // Empty test is false
        }

        let expr = args.join(" ");
        let env = winsh_core::Env::new();

        match winsh_core::eval_conditional(&expr, &env) {
            Ok(true) => Ok(0),
            Ok(false) => Ok(1),
            Err(e) => Err(e),
        }
    }

    fn help(&self) -> &str {
        "test EXPR
Evaluate conditional expression.
Returns 0 if EXPR is true, 1 if false."
    }
}

/// Process escape sequences in a string.
fn process_escapes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('a') => result.push('\x07'),
                Some('b') => result.push('\x08'),
                Some('e') => result.push('\x1b'),
                Some('f') => result.push('\x0c'),
                Some('v') => result.push('\x0b'),
                Some('0') => {
                    // Octal escape
                    let mut octal = String::new();
                    while let Some(&c) = chars.peek() {
                        if c >= '0' && c <= '7' {
                            octal.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if let Ok(n) = u32::from_str_radix(&octal, 8) {
                        if let Some(c) = char::from_u32(n) {
                            result.push(c);
                        }
                    }
                }
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// `eval` command - evaluate arguments as a command
pub struct EvalBuiltin;

impl Builtin for EvalBuiltin {
    fn name(&self) -> &str {
        "eval"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        // TODO: Execute the arguments as a shell command
        // For now, just print the command
        let cmd = args.join(" ");
        eprintln!("eval: {}", cmd);
        Ok(0)
    }

    fn help(&self) -> &str {
        "eval [arg ...]
Execute arguments as a shell command."
    }
}

/// `exec` command - replace shell with command
pub struct ExecBuiltin;

impl Builtin for ExecBuiltin {
    fn name(&self) -> &str {
        "exec"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            return Ok(0);
        }

        // Replace the shell with the command
        let mut cmd = std::process::Command::new(args[0]);
        if args.len() > 1 {
            cmd.args(&args[1..]);
        }

        // Execute the command (this replaces the current process)
        Err(ShellError::message("exec: not yet implemented"))
    }

    fn help(&self) -> &str {
        "exec [command [arguments]]
Replace the shell with the given command."
    }
}

/// `pushd` command - push directory onto stack
pub struct PushdBuiltin;

impl Builtin for PushdBuiltin {
    fn name(&self) -> &str {
        "pushd"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        // TODO: Implement pushd
        eprintln!("pushd: not yet implemented");
        Ok(1)
    }

    fn help(&self) -> &str {
        "pushd [dir]
Push directory onto the directory stack."
    }
}

/// `popd` command - pop directory from stack
pub struct PopdBuiltin;

impl Builtin for PopdBuiltin {
    fn name(&self) -> &str {
        "popd"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        // TODO: Implement popd
        eprintln!("popd: not yet implemented");
        Ok(1)
    }

    fn help(&self) -> &str {
        "popd
Pop directory from the directory stack."
    }
}

/// `dirs` command - display directory stack
pub struct DirsBuiltin;

impl Builtin for DirsBuiltin {
    fn name(&self) -> &str {
        "dirs"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        // TODO: Implement dirs
        let cwd = std::env::current_dir().unwrap_or_default();
        println!("{}", cwd.display());
        Ok(0)
    }

    fn help(&self) -> &str {
        "dirs
Display the directory stack."
    }
}

/// `alias` command - define or display aliases
pub struct AliasBuiltin;

impl Builtin for AliasBuiltin {
    fn name(&self) -> &str {
        "alias"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            // TODO: Display all aliases
            println!("alias: no aliases defined");
            return Ok(0);
        }

        for arg in args {
            if let Some((name, value)) = arg.split_once('=') {
                // Define alias
                // TODO: Store alias in shell state
                println!("alias {}='{}'", name, value);
            } else {
                // Display alias
                // TODO: Look up alias in shell state
                println!("alias: {}: not found", arg);
            }
        }

        Ok(0)
    }

    fn help(&self) -> &str {
        "alias [name[=value] ...]
Define or display aliases."
    }
}

/// `unalias` command - remove aliases
pub struct UnaliasBuiltin;

impl Builtin for UnaliasBuiltin {
    fn name(&self) -> &str {
        "unalias"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            return Err(ShellError::message("unalias: not enough arguments"));
        }

        for arg in args {
            if *arg == "-a" {
                // Remove all aliases
                // TODO: Clear all aliases
                continue;
            }
            // TODO: Remove specific alias
        }

        Ok(0)
    }

    fn help(&self) -> &str {
        "unalias [-a] [name ...]
Remove aliases."
    }
}

/// `export` command - export variables
pub struct ExportBuiltin;

impl Builtin for ExportBuiltin {
    fn name(&self) -> &str {
        "export"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            // TODO: Display all exported variables
            return Ok(0);
        }

        for arg in args {
            if *arg == "-n" {
                // Unexport (not yet supported)
                continue;
            }

            if let Some((name, value)) = arg.split_once('=') {
                // Export with value
                // TODO: Set and export variable
                std::env::set_var(name, value);
            } else {
                // Export existing variable
                // TODO: Export variable
            }
        }

        Ok(0)
    }

    fn help(&self) -> &str {
        "export [-n] [name[=value] ...]
Export variables to the environment."
    }
}

/// `unset` command - unset variables
pub struct UnsetBuiltin;

impl Builtin for UnsetBuiltin {
    fn name(&self) -> &str {
        "unset"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            return Err(ShellError::message("unset: not enough arguments"));
        }

        for arg in args {
            if *arg == "-v" || *arg == "-f" {
                // Variable or function mode
                continue;
            }
            // TODO: Unset variable
            std::env::remove_var(arg);
        }

        Ok(0)
    }

    fn help(&self) -> &str {
        "unset [-v] [-f] [name ...]
Unset variables or functions."
    }
}

/// `source` or `.` command - execute a script
pub struct SourceBuiltin;

impl Builtin for SourceBuiltin {
    fn name(&self) -> &str {
        "source"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            return Err(ShellError::message("source: filename required"));
        }

        let filename = args[0];
        match std::fs::read_to_string(filename) {
            Ok(content) => {
                // TODO: Execute the script in the current shell context
                eprintln!("source: executing {}", filename);
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        // TODO: Execute each line
                        eprintln!("  > {}", trimmed);
                    }
                }
                Ok(0)
            }
            Err(e) => Err(ShellError::Io(e)),
        }
    }

    fn help(&self) -> &str {
        "source filename
Execute commands from filename in the current shell."
    }
}

/// `type` command - display command type
pub struct TypeBuiltin;

impl Builtin for TypeBuiltin {
    fn name(&self) -> &str {
        "type"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            return Err(ShellError::message("type: not enough arguments"));
        }

        for arg in args {
            // Check if it's a builtin
            let builtins = [
                "echo", "printf", "read", "true", "false", "test",
                "eval", "exec", "pushd", "popd", "dirs",
                "alias", "unalias", "export", "unset",
                "source", ".", "type", "cd", "pwd", "exit",
            ];

            if builtins.contains(arg) {
                println!("{} is a shell builtin", arg);
                continue;
            }

            // Check if it's in PATH
            match which::which(arg) {
                Ok(path) => println!("{} is {}", arg, path.display()),
                Err(_) => println!("{}: not found", arg),
            }
        }

        Ok(0)
    }

    fn help(&self) -> &str {
        "type [name ...]
Display how each name would be interpreted as a command."
    }
}

/// `which` command - locate a command
pub struct WhichBuiltin;

impl Builtin for WhichBuiltin {
    fn name(&self) -> &str {
        "which"
    }

    fn execute(&self, args: &[&str]) -> Result<i32, ShellError> {
        if args.is_empty() {
            return Err(ShellError::message("which: not enough arguments"));
        }

        let mut found = true;
        for arg in args {
            match which::which(arg) {
                Ok(path) => println!("{}", path.display()),
                Err(_) => {
                    eprintln!("which: no {} in PATH", arg);
                    found = false;
                }
            }
        }

        if found { Ok(0) } else { Ok(1) }
    }

    fn help(&self) -> &str {
        "which [name ...]
Locate a command in PATH."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_basic() {
        let echo = EchoBuiltin;
        assert_eq!(echo.execute(&["hello"]).unwrap(), 0);
    }

    #[test]
    fn test_echo_no_newline() {
        let echo = EchoBuiltin;
        assert_eq!(echo.execute(&["-n", "hello"]).unwrap(), 0);
    }

    #[test]
    fn test_true() {
        let true_cmd = TrueBuiltin;
        assert_eq!(true_cmd.execute(&[]).unwrap(), 0);
    }

    #[test]
    fn test_false() {
        let false_cmd = FalseBuiltin;
        assert_eq!(false_cmd.execute(&[]).unwrap(), 1);
    }

    #[test]
    fn test_unset() {
        let unset = UnsetBuiltin;
        assert_eq!(unset.execute(&["TEST_VAR"]).unwrap(), 0);
    }
}
