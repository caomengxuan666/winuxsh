// Tokenizer module for WinSH MVP6
// Ported from MVP5 to provide lexical analysis for shell commands

use crate::error::Result;

/// Token types for lexical analysis
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Pipe,           // |
    And,            // &&
    Or,             // ||
    Background,     // &
    Semicolon,      // ;
    RedirIn,        // <
    RedirOut,       // >
    RedirAppend,    // >>
    RedirErr,       // 2>
    RedirErrAppend, // 2>>
    RedirErrToOut,  // 2>&1
    RedirOutToErr,  // 1>&2 or >&2
    Wildcard(String),   // Wildcard pattern
    CommandSubst(String), // Command substitution
    ArrayStart,     // (
    ArrayEnd,       // )
}

/// Command information structure
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub args: Vec<String>,
    pub stdin_redir: Option<String>,
    pub stdout_redir: Option<String>,
    pub stderr_redir: Option<String>,
    pub stdout_append: bool,
    pub stderr_append: bool,
    pub stderr_to_stdout: bool,
    pub stdout_to_stderr: bool,
    pub background: bool,
}

impl Default for CommandInfo {
    fn default() -> Self {
        CommandInfo {
            args: Vec::new(),
            stdin_redir: None,
            stdout_redir: None,
            stderr_redir: None,
            stdout_append: false,
            stderr_append: false,
            stderr_to_stdout: false,
            stdout_to_stderr: false,
            background: false,
        }
    }
}

/// Parsed command AST
#[derive(Debug, Clone)]
pub enum ParsedCommand {
    Single(CommandInfo),
    Pipeline(Vec<CommandInfo>),
    And(Box<ParsedCommand>, Box<ParsedCommand>),
    Or(Box<ParsedCommand>, Box<ParsedCommand>),
    Sequence(Vec<ParsedCommand>),
}

impl ParsedCommand {
    /// Convert to single command
    pub fn into_single_cmd(self) -> CommandInfo {
        match self {
            ParsedCommand::Single(cmd) => cmd,
            _ => panic!("Expected single command"),
        }
    }
}

/// Tokenizer for shell commands
pub struct Tokenizer;

impl Tokenizer {
    /// Tokenize a command string into tokens
    pub fn tokenize(cmd: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;
        let mut quote_char = ' ';
        let mut chars_iter = cmd.chars().peekable();
        
        while let Some(ch) = chars_iter.next() {
            if in_quote {
                if ch == quote_char {
                    in_quote = false;
                    current.push(ch);
                } else {
                    current.push(ch);
                }
            } else if ch == '\'' || ch == '"' {
                in_quote = true;
                quote_char = ch;
                current.push(ch);
            } else if ch == '|' {
                if !current.trim().is_empty() {
                    tokens.push(Token::Word(current.trim().to_string()));
                    current.clear();
                }
                // Check for ||
                if let Some(&next_ch) = chars_iter.peek() {
                    if next_ch == '|' {
                        chars_iter.next(); // Consume second |
                        tokens.push(Token::Or);
                    } else {
                        tokens.push(Token::Pipe);
                    }
                } else {
                    tokens.push(Token::Pipe);
                }
            } else if ch == '&' {
                if !current.trim().is_empty() {
                    tokens.push(Token::Word(current.trim().to_string()));
                    current.clear();
                }
                // Check for &&
                if let Some(&next_ch) = chars_iter.peek() {
                    if next_ch == '&' {
                        chars_iter.next(); // Consume second &
                        tokens.push(Token::And);
                    } else {
                        tokens.push(Token::Background);
                    }
                } else {
                    tokens.push(Token::Background);
                }
            } else if ch == ';' {
                if !current.trim().is_empty() {
                    tokens.push(Token::Word(current.trim().to_string()));
                    current.clear();
                }
                tokens.push(Token::Semicolon);
            } else if ch == '<' {
                if !current.trim().is_empty() {
                    tokens.push(Token::Word(current.trim().to_string()));
                    current.clear();
                }
                tokens.push(Token::RedirIn);
            } else if ch == '>' {
                if !current.trim().is_empty() {
                    tokens.push(Token::Word(current.trim().to_string()));
                    current.clear();
                }
                // Check for >>, >&2
                if let Some(&next_ch) = chars_iter.peek() {
                    if next_ch == '>' {
                        chars_iter.next(); // Consume second >
                        tokens.push(Token::RedirAppend);
                    } else if next_ch == '&' {
                        chars_iter.next(); // Consume &
                        if let Some(&fd_ch) = chars_iter.peek() {
                            if fd_ch == '2' {
                                chars_iter.next(); // Consume 2
                                tokens.push(Token::RedirOutToErr);
                            } else {
                                tokens.push(Token::RedirOut);
                                tokens.push(Token::Background);
                                current.push(fd_ch);
                                chars_iter.next();
                            }
                        } else {
                            tokens.push(Token::RedirOut);
                            tokens.push(Token::Background);
                        }
                    } else {
                        tokens.push(Token::RedirOut);
                    }
                } else {
                    tokens.push(Token::RedirOut);
                }
            } else if ch == '1' || ch == '2' {
                // File-descriptor-prefixed redirection: 1>, 1>>, 1>&2, 2>, 2>>, 2>&1
                if current.is_empty() {
                    if let Some(&next_ch) = chars_iter.peek() {
                        if next_ch == '>' {
                            chars_iter.next(); // consume >
                            if let Some(&after_ch) = chars_iter.peek() {
                                if after_ch == '>' {
                                    chars_iter.next(); // consume second >
                                    if ch == '2' {
                                        tokens.push(Token::RedirErrAppend);
                                    } else {
                                        tokens.push(Token::RedirAppend);
                                    }
                                } else if after_ch == '&' {
                                    chars_iter.next(); // consume &
                                    if let Some(&fd_ch) = chars_iter.peek() {
                                        if ch == '2' && fd_ch == '1' {
                                            chars_iter.next(); // consume 1
                                            tokens.push(Token::RedirErrToOut);
                                        } else if ch == '1' && fd_ch == '2' {
                                            chars_iter.next(); // consume 2
                                            tokens.push(Token::RedirOutToErr);
                                        } else {
                                            // Fallback: keep best-effort behavior.
                                            if ch == '2' {
                                                tokens.push(Token::RedirErr);
                                            } else {
                                                tokens.push(Token::RedirOut);
                                            }
                                            tokens.push(Token::Background);
                                            current.push(fd_ch);
                                            chars_iter.next();
                                        }
                                    } else if ch == '2' {
                                        tokens.push(Token::RedirErr);
                                        tokens.push(Token::Background);
                                    } else {
                                        tokens.push(Token::RedirOut);
                                        tokens.push(Token::Background);
                                    }
                                } else if ch == '2' {
                                    tokens.push(Token::RedirErr);
                                } else {
                                    tokens.push(Token::RedirOut);
                                }
                            } else if ch == '2' {
                                tokens.push(Token::RedirErr);
                            } else {
                                tokens.push(Token::RedirOut);
                            }
                            continue;
                        }
                    }
                }
                current.push(ch);
            } else if ch == '(' {
                if !current.trim().is_empty() {
                    tokens.push(Token::Word(current.trim().to_string()));
                    current.clear();
                }
                tokens.push(Token::ArrayStart);
            } else if ch == ')' {
                if !current.trim().is_empty() {
                    tokens.push(Token::Word(current.trim().to_string()));
                    current.clear();
                }
                tokens.push(Token::ArrayEnd);
            } else if ch.is_whitespace() {
                if !current.trim().is_empty() {
                    tokens.push(Token::Word(current.trim().to_string()));
                    current.clear();
                }
            } else {
                current.push(ch);
            }
        }
        
        if !current.trim().is_empty() {
            tokens.push(Token::Word(current.trim().to_string()));
        }
        
        // Process multi-character operators
        let processed_tokens = Self::process_operators(tokens)?;
        
        Ok(processed_tokens)
    }
    
    /// Process multi-character operators
    fn process_operators(tokens: Vec<Token>) -> Result<Vec<Token>> {
        let mut processed_tokens = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            match &tokens[i] {
                Token::Word(s) => {
                    if s == "&" && i + 1 < tokens.len() && tokens[i + 1] == Token::And {
                        processed_tokens.push(Token::Background);
                        i += 2;
                    } else if s == ">" && i + 1 < tokens.len() && tokens[i + 1] == Token::RedirOut {
                        processed_tokens.push(Token::RedirAppend);
                        i += 2;
                    } else if s == "|" && i + 1 < tokens.len() && tokens[i + 1] == Token::Pipe {
                        processed_tokens.push(Token::Or);
                        i += 2;
                    } else {
                        processed_tokens.push(tokens[i].clone());
                        i += 1;
                    }
                }
                _ => {
                    processed_tokens.push(tokens[i].clone());
                    i += 1;
                }
            }
        }
        
        Ok(processed_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_command() {
        let tokens = Tokenizer::tokenize("echo hello").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::Word("echo".to_string()));
        assert_eq!(tokens[1], Token::Word("hello".to_string()));
    }

    #[test]
    fn test_tokenize_pipe() {
        let tokens = Tokenizer::tokenize("echo test | grep test").unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[2], Token::Pipe);
    }

    #[test]
    fn test_tokenize_and() {
        let tokens = Tokenizer::tokenize("cmd1 && cmd2").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1], Token::And);
    }

    #[test]
    fn test_tokenize_or() {
        let tokens = Tokenizer::tokenize("cmd1 || cmd2").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1], Token::Or);
    }

    #[test]
    fn test_tokenize_redirect() {
        let tokens = Tokenizer::tokenize("echo test > output.txt").unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[2], Token::RedirOut);
    }

    #[test]
    fn test_tokenize_append() {
        let tokens = Tokenizer::tokenize("echo test >> output.txt").unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[2], Token::RedirAppend);
    }

    #[test]
    fn test_tokenize_error_redirect() {
        let tokens = Tokenizer::tokenize("cmd 2> error.txt").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1], Token::RedirErr);
    }

    #[test]
    fn test_tokenize_error_redirect_append() {
        let tokens = Tokenizer::tokenize("cmd 2>> error.txt").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1], Token::RedirErrAppend);
    }

    #[test]
    fn test_tokenize_stderr_to_stdout() {
        let tokens = Tokenizer::tokenize("cmd 2>&1").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1], Token::RedirErrToOut);
    }

    #[test]
    fn test_tokenize_stdout_to_stderr() {
        let tokens = Tokenizer::tokenize("cmd 1>&2").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1], Token::RedirOutToErr);
    }

    #[test]
    fn test_tokenize_background() {
        let tokens = Tokenizer::tokenize("cmd &").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1], Token::Background);
    }

    #[test]
    fn test_tokenize_quotes() {
        let tokens = Tokenizer::tokenize("echo \"hello world\"").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1], Token::Word("\"hello world\"".to_string()));
    }

    #[test]
    fn test_tokenize_array() {
        let tokens = Tokenizer::tokenize("array define fruits (apple banana)").unwrap();
        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[3], Token::ArrayStart);
        assert_eq!(tokens[6], Token::ArrayEnd);
    }
}
