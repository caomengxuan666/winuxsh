//! Completion system with context-aware matching.
//!
//! Provides command, path, variable, and option completions
//! with fuzzy matching and menu support.

use std::path::Path;
use winsh_core::ShellState;

/// A completion suggestion.
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// The completion text
    pub text: String,
    /// Display text (may differ from completion text)
    pub display: Option<String>,
    /// Description of the suggestion
    pub description: Option<String>,
    /// Category/tag
    pub category: CompletionCategory,
    /// Priority (higher = more relevant)
    pub priority: u32,
}

/// Categories of completions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionCategory {
    Command,
    Builtin,
    Alias,
    Function,
    File,
    Directory,
    Variable,
    Option,
    Keyword,
    History,
    Other,
}

/// Context in which completion is requested.
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// The full input line
    pub input: String,
    /// Cursor position
    pub cursor_pos: usize,
    /// Current working directory
    pub cwd: String,
    /// Shell state
    pub state: CompletionState,
}

/// Simplified state for completions.
#[derive(Debug, Clone)]
pub struct CompletionState {
    /// Built-in command names
    pub builtins: Vec<String>,
    /// Alias names
    pub aliases: Vec<String>,
    /// Function names
    pub functions: Vec<String>,
    /// Shell keywords
    pub keywords: Vec<String>,
    /// PATH directories
    pub path_dirs: Vec<String>,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self {
            builtins: vec![
                "echo", "printf", "read", "true", "false", "test",
                "eval", "exec", "pushd", "popd", "dirs",
                "alias", "unalias", "export", "unset",
                "source", ".", "type", "which", "cd", "pwd", "exit",
                "bg", "fg", "jobs", "kill", "wait", "disown",
                "clear", "help", "history",
            ].into_iter().map(|s| s.to_string()).collect(),
            aliases: Vec::new(),
            functions: Vec::new(),
            keywords: vec![
                "if", "then", "elif", "else", "fi",
                "for", "in", "do", "done", "while", "until",
                "case", "esac", "select",
                "function", "time", "coproc",
            ].into_iter().map(|s| s.to_string()).collect(),
            path_dirs: Vec::new(),
        }
    }
}

/// The main completer.
pub struct Completer {
    /// Whether fuzzy matching is enabled
    fuzzy: bool,
    /// Maximum number of suggestions
    max_suggestions: usize,
}

impl Completer {
    /// Create a new completer.
    pub fn new() -> Self {
        Self {
            fuzzy: true,
            max_suggestions: 100,
        }
    }

    /// Get completions for the given context.
    pub fn complete(&self, ctx: &CompletionContext) -> Vec<Suggestion> {
        let word = self.get_current_word(ctx);

        if word.is_empty() {
            return vec![];
        }

        // Determine what we're completing
        // Check variables and options first, then commands
        if self.is_completing_variable(&word) {
            self.complete_variable(&word, ctx)
        } else if self.is_completing_option(&word) {
            self.complete_option(&word, ctx)
        } else if self.is_completing_command(ctx) {
            self.complete_command(&word, ctx)
        } else {
            self.complete_path(&word, ctx)
        }
    }

    /// Get the word currently being completed.
    fn get_current_word<'a>(&self, ctx: &'a CompletionContext) -> &'a str {
        let before = &ctx.input[..ctx.cursor_pos];
        let last_space = before.rfind(|c: char| c.is_whitespace()).map(|i| i + 1).unwrap_or(0);
        &before[last_space..]
    }

    /// Check if we're completing a command (first word).
    fn is_completing_command(&self, ctx: &CompletionContext) -> bool {
        let before = &ctx.input[..ctx.cursor_pos];
        // No whitespace before cursor = we're on the first word
        !before.trim().contains(char::is_whitespace)
    }

    /// Check if we're completing a variable.
    fn is_completing_variable(&self, word: &str) -> bool {
        word.starts_with('$')
    }

    /// Check if we're completing an option.
    fn is_completing_option(&self, word: &str) -> bool {
        word.starts_with('-')
    }

    /// Complete command names.
    fn complete_command(&self, word: &str, ctx: &CompletionContext) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Add builtins
        for name in &ctx.state.builtins {
            if self.matches(word, name) {
                suggestions.push(Suggestion {
                    text: name.clone(),
                    display: None,
                    description: Some("builtin".to_string()),
                    category: CompletionCategory::Builtin,
                    priority: 100,
                });
            }
        }

        // Add aliases
        for name in &ctx.state.aliases {
            if self.matches(word, name) {
                suggestions.push(Suggestion {
                    text: name.clone(),
                    display: None,
                    description: Some("alias".to_string()),
                    category: CompletionCategory::Alias,
                    priority: 90,
                });
            }
        }

        // Add functions
        for name in &ctx.state.functions {
            if self.matches(word, name) {
                suggestions.push(Suggestion {
                    text: name.clone(),
                    display: None,
                    description: Some("function".to_string()),
                    category: CompletionCategory::Function,
                    priority: 80,
                });
            }
        }

        // Add keywords
        for name in &ctx.state.keywords {
            if self.matches(word, name) {
                suggestions.push(Suggestion {
                    text: name.clone(),
                    display: None,
                    description: Some("keyword".to_string()),
                    category: CompletionCategory::Keyword,
                    priority: 70,
                });
            }
        }

        // Add executables from PATH
        for dir in &ctx.state.path_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if self.matches(word, &name) {
                        // Check if it's executable (or has .exe/.bat/.cmd on Windows)
                        let is_exec = if cfg!(windows) {
                            name.ends_with(".exe") || name.ends_with(".bat") || name.ends_with(".cmd")
                        } else {
                            true
                        };
                        if is_exec {
                            let display_name = if cfg!(windows) {
                                name.trim_end_matches(".exe").trim_end_matches(".bat").trim_end_matches(".cmd").to_string()
                            } else {
                                name.clone()
                            };
                            suggestions.push(Suggestion {
                                text: display_name,
                                display: Some(name),
                                description: Some("command".to_string()),
                                category: CompletionCategory::Command,
                                priority: 60,
                            });
                        }
                    }
                }
            }
        }

        // Also complete files/directories as potential commands
        suggestions.extend(self.complete_path(word, ctx));

        // Sort by priority (descending)
        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
        suggestions.truncate(self.max_suggestions);

        suggestions
    }

    /// Complete variable names.
    fn complete_variable(&self, word: &str, ctx: &CompletionContext) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Strip leading $
        let var_prefix = if word.starts_with("${") {
            &word[2..]
        } else if word.starts_with('$') {
            &word[1..]
        } else {
            word
        };

        let common_vars = [
            "HOME", "USER", "USERNAME", "PATH", "PWD", "OLDPWD",
            "SHELL", "TERM", "DISPLAY", "EDITOR", "VISUAL", "PAGER",
            "PS1", "PROMPT", "RPROMPT", "PS2",
            "LANG", "LC_ALL", "TZ",
            "HISTFILE", "HISTSIZE", "SAVEHIST",
            "TEMP", "TMP", "TMPDIR",
            "?",
        ];

        for var in &common_vars {
            if self.matches(var_prefix, var) {
                suggestions.push(Suggestion {
                    text: format!("${}", var),
                    display: None,
                    description: Some("variable".to_string()),
                    category: CompletionCategory::Variable,
                    priority: 50,
                });
            }
        }

        suggestions
    }

    /// Complete option names.
    fn complete_option(&self, word: &str, _ctx: &CompletionContext) -> Vec<Suggestion> {
        let common_options = [
            ("--help", "show help"),
            ("--version", "show version"),
            ("-h", "show help"),
            ("-v", "verbose"),
            ("-q", "quiet"),
            ("-f", "force"),
            ("-r", "recursive"),
            ("-n", "no newline"),
            ("-e", "enable escapes"),
            ("-p", "prepend/port"),
            ("-o", "output"),
            ("-i", "interactive"),
            ("-a", "all"),
            ("-l", "list"),
            ("-t", "terse"),
            ("-d", "directory"),
            ("-s", "silent"),
            ("-w", "warn"),
            ("-x", "debug"),
        ];

        let mut suggestions = Vec::new();
        for (option, desc) in &common_options {
            if self.matches(word, option) {
                suggestions.push(Suggestion {
                    text: option.to_string(),
                    display: None,
                    description: Some(desc.to_string()),
                    category: CompletionCategory::Option,
                    priority: 80,
                });
            }
        }
        suggestions
    }

    /// Complete file and directory paths.
    fn complete_path(&self, word: &str, ctx: &CompletionContext) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Determine the base directory and prefix
        let (base_dir, prefix) = if word.contains('/') || word.contains('\\') {
            let sep = if word.contains('/') { '/' } else { '\\' };
            let last_sep = word.rfind(sep).unwrap_or(0);
            let base = if last_sep == 0 {
                if word.starts_with('/') || word.starts_with('\\') {
                    word[..1].to_string()
                } else {
                    ctx.cwd.clone()
                }
            } else {
                let base_path = &word[..last_sep];
                if base_path.starts_with('/') || base_path.starts_with('\\') {
                    base_path.to_string()
                } else {
                    Path::new(&ctx.cwd).join(base_path).to_string_lossy().to_string()
                }
            };
            let prefix = &word[last_sep + 1..];
            (base, prefix.to_string())
        } else {
            (ctx.cwd.clone(), word.to_string())
        };

        // Read the directory
        if let Ok(entries) = std::fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if self.matches(&prefix, &name) {
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

                    let completion_text = if base_dir == ctx.cwd {
                        name.clone()
                    } else {
                        let full = Path::new(&base_dir).join(&name);
                        let rel = full.strip_prefix(&ctx.cwd)
                            .unwrap_or(&full)
                            .to_string_lossy()
                            .to_string();
                        // Ensure we use consistent separators
                        rel.replace('\\', "/")
                    };

                    suggestions.push(Suggestion {
                        text: if is_dir {
                            format!("{}/", completion_text)
                        } else {
                            completion_text
                        },
                        display: None,
                        description: Some(if is_dir { "directory" } else { "file" }.to_string()),
                        category: if is_dir { CompletionCategory::Directory } else { CompletionCategory::File },
                        priority: if is_dir { 40 } else { 30 },
                    });
                }
            }
        }

        // Sort: directories first, then alphabetically
        suggestions.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then(a.text.cmp(&b.text))
        });
        suggestions.truncate(self.max_suggestions);

        suggestions
    }

    /// Check if a word matches a candidate.
    fn matches(&self, pattern: &str, candidate: &str) -> bool {
        if pattern.is_empty() {
            return true;
        }

        let pattern = pattern.to_lowercase();
        let candidate = candidate.to_lowercase();

        // Exact match
        if candidate == pattern {
            return true;
        }

        // Prefix match
        if candidate.starts_with(&pattern) {
            return true;
        }

        // Fuzzy match
        if self.fuzzy && self.fuzzy_match(&pattern, &candidate) {
            return true;
        }

        false
    }

    /// Fuzzy match a pattern against a candidate.
    fn fuzzy_match(&self, pattern: &str, candidate: &str) -> bool {
        let mut pattern_chars = pattern.chars().peekable();
        let candidate_chars: Vec<char> = candidate.chars().collect();

        let mut pos = 0;
        while let Some(&p) = pattern_chars.peek() {
            let mut found = false;
            while pos < candidate_chars.len() {
                if candidate_chars[pos].to_ascii_lowercase() == p.to_ascii_lowercase() {
                    pattern_chars.next();
                    pos += 1;
                    found = true;
                    break;
                }
                pos += 1;
            }
            if !found {
                return false;
            }
        }
        true
    }
}

impl Default for Completer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_command_builtin() {
        let completer = Completer::new();
        let ctx = CompletionContext {
            input: "ec".to_string(),
            cursor_pos: 2,
            cwd: ".".to_string(),
            state: CompletionState::default(),
        };
        let suggestions = completer.complete(&ctx);
        assert!(suggestions.iter().any(|s| s.text == "echo"));
    }

    #[test]
    fn test_complete_variable() {
        let completer = Completer::new();
        let ctx = CompletionContext {
            input: "$HO".to_string(),
            cursor_pos: 3,
            cwd: ".".to_string(),
            state: CompletionState::default(),
        };
        let suggestions = completer.complete(&ctx);
        assert!(suggestions.iter().any(|s| s.text == "$HOME"));
    }

    #[test]
    fn test_fuzzy_match() {
        let completer = Completer::new();
        assert!(completer.fuzzy_match("hlo", "hello"));
        assert!(completer.fuzzy_match("wrd", "world"));
        assert!(!completer.fuzzy_match("xyz", "hello"));
    }

    #[test]
    fn test_matches_exact() {
        let completer = Completer::new();
        assert!(completer.matches("echo", "echo"));
    }

    #[test]
    fn test_matches_prefix() {
        let completer = Completer::new();
        assert!(completer.matches("ec", "echo"));
    }

    #[test]
    fn test_get_current_word() {
        let completer = Completer::new();
        let ctx = CompletionContext {
            input: "echo hel".to_string(),
            cursor_pos: 8,
            cwd: ".".to_string(),
            state: CompletionState::default(),
        };
        assert_eq!(completer.get_current_word(&ctx), "hel");
    }

    #[test]
    fn test_is_completing_command() {
        let completer = Completer::new();
        let ctx = CompletionContext {
            input: "echo".to_string(),
            cursor_pos: 4,
            cwd: ".".to_string(),
            state: CompletionState::default(),
        };
        assert!(completer.is_completing_command(&ctx));

        let ctx = CompletionContext {
            input: "echo hello".to_string(),
            cursor_pos: 10,
            cwd: ".".to_string(),
            state: CompletionState::default(),
        };
        assert!(!completer.is_completing_command(&ctx));
    }

    #[test]
    fn test_is_completing_variable() {
        let completer = Completer::new();
        assert!(completer.is_completing_variable("$HO"));
        assert!(completer.is_completing_variable("${HO"));
        assert!(!completer.is_completing_variable("hello"));
    }

    #[test]
    fn test_is_completing_option() {
        let completer = Completer::new();
        assert!(completer.is_completing_option("--he"));
        assert!(completer.is_completing_option("-v"));
        assert!(!completer.is_completing_option("hello"));
    }
}
