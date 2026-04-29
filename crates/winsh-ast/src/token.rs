//! Token types for the lexer.

use std::fmt;

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: crate::span::Span,
}

/// The type of a token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals and words
    /// A plain word (unquoted text)
    Word(String),
    /// A single-quoted string (no expansion)
    SingleQuoted(String),
    /// A double-quoted string (expansion allowed)
    DoubleQuoted(String),
    /// A dollar-quoted string with ANSI C escapes
    DollarQuoted(String),
    /// A variable reference: $VAR or ${VAR}
    Variable(String),
    /// A braced variable: ${VAR}
    BracedVariable(String),
    /// A command substitution: $(...)
    CommandSubst(String),
    /// A backtick command substitution: `...`
    BacktickSubst(String),
    /// An arithmetic expansion: $((...))
    Arithmetic(String),
    /// A glob pattern: *, ?, [...]
    Glob(String),

    // Operators
    /// Pipe: |
    Pipe,
    /// Logical AND: &&
    And,
    /// Logical OR: ||
    Or,
    /// Background: &
    Background,
    /// Semicolon: ;
    Semicolon,
    /// Newline
    Newline,

    // Redirections
    /// Input redirect: <
    RedirIn,
    /// Output redirect: >
    RedirOut,
    /// Append redirect: >>
    RedirAppend,
    /// Stderr redirect: 2>
    RedirErr,
    /// Stderr append: 2>>
    RedirErrAppend,
    /// Stderr to stdout: 2>&1
    RedirErrToOut,
    /// Stdout to stderr: 1>&2
    RedirOutToErr,
    /// Combined redirect: &>
    RedirCombined,
    /// Combined append redirect: &>>
    RedirCombinedAppend,
    /// Here document: <<
    HereDoc,
    /// Here string: <<<
    HereString,

    // Grouping
    /// Left parenthesis: (
    LeftParen,
    /// Right parenthesis: )
    RightParen,
    /// Left brace: {
    LeftBrace,
    /// Right brace: }
    RightBrace,
    /// Left bracket: [[
    DoubleLeftBracket,
    /// Right bracket: ]]
    DoubleRightBracket,

    // Keywords
    /// if
    If,
    /// then
    Then,
    /// elif
    Elif,
    /// else
    Else,
    /// fi
    Fi,
    /// for
    For,
    /// in
    In,
    /// do
    Do,
    /// done
    Done,
    /// while
    While,
    /// until
    Until,
    /// case
    Case,
    /// esac
    Esac,
    /// select
    Select,
    /// function
    Function,
    /// time
    Time,
    /// coproc
    Coproc,

    // Special
    /// Bang: !
    Bang,
    /// Dollar: $
    Dollar,
    /// Backslash: \
    Backslash,
    /// Comment: #...
    Comment(String),
    /// End of file
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Word(s) => write!(f, "{}", s),
            TokenKind::SingleQuoted(s) => write!(f, "'{}'", s),
            TokenKind::DoubleQuoted(s) => write!(f, "\"{}\"", s),
            TokenKind::DollarQuoted(s) => write!(f, "$'{}'", s),
            TokenKind::Variable(s) => write!(f, "${}", s),
            TokenKind::BracedVariable(s) => write!(f, "${{{}}}", s),
            TokenKind::CommandSubst(s) => write!(f, "$({})", s),
            TokenKind::BacktickSubst(s) => write!(f, "`{}`", s),
            TokenKind::Arithmetic(s) => write!(f, "$(({}))", s),
            TokenKind::Glob(s) => write!(f, "{}", s),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Background => write!(f, "&"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Newline => writeln!(f),
            TokenKind::RedirIn => write!(f, "<"),
            TokenKind::RedirOut => write!(f, ">"),
            TokenKind::RedirAppend => write!(f, ">>"),
            TokenKind::RedirErr => write!(f, "2>"),
            TokenKind::RedirErrAppend => write!(f, "2>>"),
            TokenKind::RedirErrToOut => write!(f, "2>&1"),
            TokenKind::RedirOutToErr => write!(f, "1>&2"),
            TokenKind::RedirCombined => write!(f, "&>"),
            TokenKind::RedirCombinedAppend => write!(f, "&>>"),
            TokenKind::HereDoc => write!(f, "<<"),
            TokenKind::HereString => write!(f, "<<<"),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::DoubleLeftBracket => write!(f, "[["),
            TokenKind::DoubleRightBracket => write!(f, "]]"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Then => write!(f, "then"),
            TokenKind::Elif => write!(f, "elif"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::Fi => write!(f, "fi"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::Done => write!(f, "done"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Until => write!(f, "until"),
            TokenKind::Case => write!(f, "case"),
            TokenKind::Esac => write!(f, "esac"),
            TokenKind::Select => write!(f, "select"),
            TokenKind::Function => write!(f, "function"),
            TokenKind::Time => write!(f, "time"),
            TokenKind::Coproc => write!(f, "coproc"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Dollar => write!(f, "$"),
            TokenKind::Backslash => write!(f, "\\"),
            TokenKind::Comment(s) => write!(f, "#{}", s),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

/// Check if a word is a shell keyword.
pub fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "then"
            | "elif"
            | "else"
            | "fi"
            | "for"
            | "in"
            | "do"
            | "done"
            | "while"
            | "until"
            | "case"
            | "esac"
            | "select"
            | "function"
            | "time"
            | "coproc"
    )
}

/// Convert a word to its keyword token kind, if applicable.
pub fn word_to_keyword(word: &str) -> Option<TokenKind> {
    match word {
        "if" => Some(TokenKind::If),
        "then" => Some(TokenKind::Then),
        "elif" => Some(TokenKind::Elif),
        "else" => Some(TokenKind::Else),
        "fi" => Some(TokenKind::Fi),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "do" => Some(TokenKind::Do),
        "done" => Some(TokenKind::Done),
        "while" => Some(TokenKind::While),
        "until" => Some(TokenKind::Until),
        "case" => Some(TokenKind::Case),
        "esac" => Some(TokenKind::Esac),
        "select" => Some(TokenKind::Select),
        "function" => Some(TokenKind::Function),
        "time" => Some(TokenKind::Time),
        "coproc" => Some(TokenKind::Coproc),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_keyword() {
        assert!(is_keyword("if"));
        assert!(is_keyword("then"));
        assert!(is_keyword("for"));
        assert!(is_keyword("while"));
        assert!(!is_keyword("echo"));
        assert!(!is_keyword("ls"));
    }

    #[test]
    fn test_word_to_keyword() {
        assert_eq!(word_to_keyword("if"), Some(TokenKind::If));
        assert_eq!(word_to_keyword("echo"), None);
    }

    #[test]
    fn test_token_display() {
        assert_eq!(TokenKind::Word("hello".to_string()).to_string(), "hello");
        assert_eq!(TokenKind::Pipe.to_string(), "|");
        assert_eq!(TokenKind::And.to_string(), "&&");
        assert_eq!(TokenKind::If.to_string(), "if");
    }
}
