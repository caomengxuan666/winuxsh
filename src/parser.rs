// Parser module for WinSH MVP6
// Ported from MVP5 to provide syntax analysis for shell commands

use crate::error::Result;
use crate::tokenizer::{Token, CommandInfo, ParsedCommand};

/// Parser for shell commands
pub struct Parser;

impl Parser {
    /// Parse tokens into a ParsedCommand AST
    pub fn parse(tokens: &[Token]) -> Result<ParsedCommand> {
        // Improved version: handle pipes and semicolons
        let mut commands = Vec::new();
        let mut current_command = CommandInfo::default();
        
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Word(s) => {
                    current_command.args.push(s.clone());
                    i += 1;
                }
                Token::Pipe => {
                    if !current_command.args.is_empty() {
                        commands.push(std::mem::replace(&mut current_command, CommandInfo::default()));
                    }
                    i += 1;
                }
                Token::RedirIn => {
                    // Get input filename
                    if i + 1 < tokens.len() {
                        if let Token::Word(filename) = &tokens[i + 1] {
                            current_command.stdin_redir = Some(filename.clone());
                            i += 2; // Skip redirect operator and filename
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                Token::RedirOut => {
                    current_command.stdout_append = false;
                    // Get output filename
                    if i + 1 < tokens.len() {
                        if let Token::Word(filename) = &tokens[i + 1] {
                            current_command.stdout_redir = Some(filename.clone());
                            i += 2; // Skip redirect operator and filename
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                Token::RedirAppend => {
                    current_command.stdout_append = true;
                    // Get output filename
                    if i + 1 < tokens.len() {
                        if let Token::Word(filename) = &tokens[i + 1] {
                            current_command.stdout_redir = Some(filename.clone());
                            i += 2; // Skip redirect operator and filename
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                Token::RedirErr => {
                    current_command.stderr_append = false;
                    // Get error output filename
                    if i + 1 < tokens.len() {
                        if let Token::Word(filename) = &tokens[i + 1] {
                            current_command.stderr_redir = Some(filename.clone());
                            i += 2; // Skip redirect operator and filename
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                Token::RedirErrAppend => {
                    current_command.stderr_append = true;
                    // Get error output filename
                    if i + 1 < tokens.len() {
                        if let Token::Word(filename) = &tokens[i + 1] {
                            current_command.stderr_redir = Some(filename.clone());
                            i += 2; // Skip redirect operator and filename
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                Token::RedirErrToOut => {
                    current_command.stderr_to_stdout = true;
                    i += 1;
                }
                Token::RedirOutToErr => {
                    current_command.stdout_to_stderr = true;
                    i += 1;
                }
                Token::Background => {
                    current_command.background = true;
                    i += 1;
                }
                Token::Semicolon => {
                    if !current_command.args.is_empty() {
                        commands.push(std::mem::replace(&mut current_command, CommandInfo::default()));
                    }
                    i += 1;
                }
                Token::And => {
                    // Handle && operator
                    if !current_command.args.is_empty() {
                        let left_cmd = std::mem::replace(&mut current_command, CommandInfo::default());
                        let remaining_tokens = &tokens[i + 1..];
                        if !remaining_tokens.is_empty() {
                            let right_parsed = Self::parse(remaining_tokens)?;
                            return Ok(ParsedCommand::And(
                                Box::new(ParsedCommand::Single(left_cmd)),
                                Box::new(right_parsed),
                            ));
                        }
                    }
                    i += 1;
                }
                Token::Or => {
                    // Handle || operator
                    if !current_command.args.is_empty() {
                        let left_cmd = std::mem::replace(&mut current_command, CommandInfo::default());
                        let remaining_tokens = &tokens[i + 1..];
                        if !remaining_tokens.is_empty() {
                            let right_parsed = Self::parse(remaining_tokens)?;
                            return Ok(ParsedCommand::Or(
                                Box::new(ParsedCommand::Single(left_cmd)),
                                Box::new(right_parsed),
                            ));
                        }
                    }
                    i += 1;
                }
                Token::ArrayStart | Token::ArrayEnd | Token::Wildcard(_) | Token::CommandSubst(_) => {
                    // Handle array and special tokens
                    i += 1;
                }
            }
        }
        
        if !current_command.args.is_empty() {
            commands.push(current_command);
        }
        
        if commands.is_empty() {
            return Ok(ParsedCommand::Single(CommandInfo::default()));
        }
        
        if commands.len() == 1 {
            Ok(ParsedCommand::Single(commands[0].clone()))
        } else {
            // Check if original tokens contain pipes
            let has_pipe = tokens.iter().any(|t| matches!(t, Token::Pipe));
            if has_pipe {
                Ok(ParsedCommand::Pipeline(commands))
            } else {
                Ok(ParsedCommand::Sequence(commands.into_iter().map(|cmd| ParsedCommand::Single(cmd)).collect()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let tokens = vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string()),
        ];
        let parsed = Parser::parse(&tokens).unwrap();
        match parsed {
            ParsedCommand::Single(cmd) => {
                assert_eq!(cmd.args, vec!["echo", "hello"]);
            }
            _ => panic!("Expected single command"),
        }
    }

    #[test]
    fn test_parse_pipeline() {
        let tokens = vec![
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::Pipe,
            Token::Word("grep".to_string()),
            Token::Word("test".to_string()),
        ];
        let parsed = Parser::parse(&tokens).unwrap();
        match parsed {
            ParsedCommand::Pipeline(cmds) => {
                assert_eq!(cmds.len(), 2);
            }
            _ => panic!("Expected pipeline"),
        }
    }

    #[test]
    fn test_parse_and() {
        let tokens = vec![
            Token::Word("cmd1".to_string()),
            Token::And,
            Token::Word("cmd2".to_string()),
        ];
        let parsed = Parser::parse(&tokens).unwrap();
        match parsed {
            ParsedCommand::And(_, _) => {
                // Success
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn test_parse_redirect() {
        let tokens = vec![
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::RedirOut,
            Token::Word("output.txt".to_string()),
        ];
        let parsed = Parser::parse(&tokens).unwrap();
        match parsed {
            ParsedCommand::Single(cmd) => {
                assert_eq!(cmd.stdout_redir, Some("output.txt".to_string()));
            }
            _ => panic!("Expected single command"),
        }
    }

    #[test]
    fn test_parse_background() {
        let tokens = vec![
            Token::Word("cmd".to_string()),
            Token::Background,
        ];
        let parsed = Parser::parse(&tokens).unwrap();
        match parsed {
            ParsedCommand::Single(cmd) => {
                assert!(cmd.background);
            }
            _ => panic!("Expected single command"),
        }
    }

    #[test]
    fn test_parse_stderr_to_stdout() {
        let tokens = vec![
            Token::Word("cmd".to_string()),
            Token::RedirErrToOut,
        ];
        let parsed = Parser::parse(&tokens).unwrap();
        match parsed {
            ParsedCommand::Single(cmd) => {
                assert!(cmd.stderr_to_stdout);
            }
            _ => panic!("Expected single command"),
        }
    }

    #[test]
    fn test_parse_stdout_to_stderr() {
        let tokens = vec![
            Token::Word("cmd".to_string()),
            Token::RedirOutToErr,
        ];
        let parsed = Parser::parse(&tokens).unwrap();
        match parsed {
            ParsedCommand::Single(cmd) => {
                assert!(cmd.stdout_to_stderr);
            }
            _ => panic!("Expected single command"),
        }
    }
}
