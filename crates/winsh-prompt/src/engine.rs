//! Prompt engine with escape sequence support.
//!
//! Supports zsh-style prompt escape sequences:
//! - %n - username
//! - %m - hostname (short)
//! - %M - hostname (full)
//! - %~ - current directory (with ~ for home)
//! - %d - current directory (full)
//! - %# - # for root, % for normal user
//! - %? - exit code of last command
//! - %T - time (24h)
//! - %t - time (12h)
//! - %D{fmt} - date/time with format
//! - %j - number of jobs
//! - %B/%b - bold on/off
//! - %U/%u - underline on/off
//! - %F{color}/%f - foreground color on/off
//! - %K{color}/%k - background color on/off
//! - %{...%} - literal escape sequence
//! - %N - shell name
//! - %i - line number
//! - %# - privilege indicator

use std::env;
use std::path::PathBuf;

/// Prompt theme configuration.
#[derive(Debug, Clone)]
pub struct PromptTheme {
    /// Primary prompt (PS1)
    pub primary: String,
    /// Right prompt (RPROMPT)
    pub right: String,
    /// Continuation prompt (PS2)
    pub continuation: String,
    /// Select prompt (PS3)
    pub select: String,
    /// Trace prompt (PS4)
    pub trace: String,
}

impl Default for PromptTheme {
    fn default() -> Self {
        Self {
            primary: "%n@%m %~ %# ".to_string(),
            right: String::new(),
            continuation: "%_> ".to_string(),
            select: "#? ".to_string(),
            trace: "+ ".to_string(),
        }
    }
}

/// Available prompt colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Color256(u8),
}

impl PromptColor {
    /// Parse a color name or number.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "black" | "0" => Some(Self::Black),
            "red" | "1" => Some(Self::Red),
            "green" | "2" => Some(Self::Green),
            "yellow" | "3" => Some(Self::Yellow),
            "blue" | "4" => Some(Self::Blue),
            "magenta" | "5" => Some(Self::Magenta),
            "cyan" | "6" => Some(Self::Cyan),
            "white" | "7" => Some(Self::White),
            "default" => Some(Self::Default),
            "bright-black" | "brightblack" => Some(Self::BrightBlack),
            "bright-red" | "brightred" => Some(Self::BrightRed),
            "bright-green" | "brightgreen" => Some(Self::BrightGreen),
            "bright-yellow" | "brightyellow" => Some(Self::BrightYellow),
            "bright-blue" | "brightblue" => Some(Self::BrightBlue),
            "bright-magenta" | "brightmagenta" => Some(Self::BrightMagenta),
            "bright-cyan" | "brightcyan" => Some(Self::BrightCyan),
            "bright-white" | "brightwhite" => Some(Self::BrightWhite),
            _ => {
                // Try parsing as a number (0-255)
                if let Ok(n) = s.parse::<u8>() {
                    Some(Self::Color256(n))
                } else {
                    None
                }
            }
        }
    }

    /// Get the ANSI escape code for this color (foreground).
    pub fn fg_code(&self) -> String {
        match self {
            Self::Black => "30".to_string(),
            Self::Red => "31".to_string(),
            Self::Green => "32".to_string(),
            Self::Yellow => "33".to_string(),
            Self::Blue => "34".to_string(),
            Self::Magenta => "35".to_string(),
            Self::Cyan => "36".to_string(),
            Self::White => "37".to_string(),
            Self::Default => "39".to_string(),
            Self::BrightBlack => "90".to_string(),
            Self::BrightRed => "91".to_string(),
            Self::BrightGreen => "92".to_string(),
            Self::BrightYellow => "93".to_string(),
            Self::BrightBlue => "94".to_string(),
            Self::BrightMagenta => "95".to_string(),
            Self::BrightCyan => "96".to_string(),
            Self::BrightWhite => "97".to_string(),
            Self::Color256(n) => format!("38;5;{}", n),
        }
    }

    /// Get the ANSI escape code for this color (background).
    pub fn bg_code(&self) -> String {
        match self {
            Self::Black => "40".to_string(),
            Self::Red => "41".to_string(),
            Self::Green => "42".to_string(),
            Self::Yellow => "43".to_string(),
            Self::Blue => "44".to_string(),
            Self::Magenta => "45".to_string(),
            Self::Cyan => "46".to_string(),
            Self::White => "47".to_string(),
            Self::Default => "49".to_string(),
            Self::BrightBlack => "100".to_string(),
            Self::BrightRed => "101".to_string(),
            Self::BrightGreen => "102".to_string(),
            Self::BrightYellow => "103".to_string(),
            Self::BrightBlue => "104".to_string(),
            Self::BrightMagenta => "105".to_string(),
            Self::BrightCyan => "106".to_string(),
            Self::BrightWhite => "107".to_string(),
            Self::Color256(n) => format!("48;5;{}", n),
        }
    }
}

/// Context information for prompt rendering.
#[derive(Debug, Clone)]
pub struct PromptContext {
    /// Current working directory
    pub cwd: PathBuf,
    /// Home directory
    pub home: Option<PathBuf>,
    /// Exit code of last command
    pub exit_code: i32,
    /// Number of background jobs
    pub job_count: usize,
    /// Current line number
    pub line_number: usize,
    /// Shell name
    pub shell_name: String,
    /// Username
    pub username: String,
    /// Hostname
    pub hostname: String,
}

impl Default for PromptContext {
    fn default() -> Self {
        Self {
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            home: env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok().map(PathBuf::from),
            exit_code: 0,
            job_count: 0,
            line_number: 1,
            shell_name: "winsh".to_string(),
            username: env::var("USER").or_else(|_| env::var("USERNAME")).unwrap_or_else(|_| "user".to_string()),
            hostname: get_hostname(),
        }
    }
}

/// Get the system hostname.
fn get_hostname() -> String {
    env::var("COMPUTERNAME")
        .or_else(|_| env::var("HOSTNAME"))
        .unwrap_or_else(|_| "localhost".to_string())
}

/// Render a prompt string with escape sequences.
pub fn render_prompt(template: &str, ctx: &PromptContext) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('n') => result.push_str(&ctx.username),
                Some('m') => {
                    let hostname = &ctx.hostname;
                    let short = hostname.split('.').next().unwrap_or(hostname);
                    result.push_str(short);
                }
                Some('M') => result.push_str(&ctx.hostname),
                Some('~') => {
                    if let Some(ref home) = ctx.home {
                        if let Ok(rel) = ctx.cwd.strip_prefix(home) {
                            result.push('~');
                            if !rel.as_os_str().is_empty() {
                                result.push('/');
                                result.push_str(&rel.to_string_lossy().replace('\\', "/"));
                            }
                        } else {
                            result.push_str(&ctx.cwd.to_string_lossy().replace('\\', "/"));
                        }
                    } else {
                        result.push_str(&ctx.cwd.to_string_lossy().replace('\\', "/"));
                    }
                }
                Some('d') => result.push_str(&ctx.cwd.to_string_lossy().replace('\\', "/")),
                Some('#') => {
                    if is_root() {
                        result.push('#');
                    } else {
                        result.push('%');
                    }
                }
                Some('?') => result.push_str(&ctx.exit_code.to_string()),
                Some('T') => result.push_str(&format_time("%H:%M")),
                Some('t') => result.push_str(&format_time("%I:%M %p")),
                Some('D') => {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        let mut fmt = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '}' { chars.next(); break; }
                            fmt.push(chars.next().unwrap());
                        }
                        result.push_str(&format_time(&fmt));
                    }
                }
                Some('j') => result.push_str(&ctx.job_count.to_string()),
                Some('B') => result.push_str("\x1b[1m"),
                Some('b') => result.push_str("\x1b[22m"),
                Some('U') => result.push_str("\x1b[4m"),
                Some('u') => result.push_str("\x1b[24m"),
                Some('F') => {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        let mut color_name = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '}' { chars.next(); break; }
                            color_name.push(chars.next().unwrap());
                        }
                        if let Some(color) = PromptColor::parse(&color_name) {
                            result.push_str(&format!("\x1b[{}m", color.fg_code()));
                        }
                    }
                }
                Some('f') => result.push_str("\x1b[39m"),
                Some('K') => {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        let mut color_name = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '}' { chars.next(); break; }
                            color_name.push(chars.next().unwrap());
                        }
                        if let Some(color) = PromptColor::parse(&color_name) {
                            result.push_str(&format!("\x1b[{}m", color.bg_code()));
                        }
                    }
                }
                Some('k') => result.push_str("\x1b[49m"),
                Some('{') => {
                    // %{...%} - literal escape sequence
                    while let Some(c) = chars.next() {
                        if c == '%' && chars.peek() == Some(&'}') {
                            chars.next();
                            break;
                        }
                        result.push(c);
                    }
                }
                Some('N') => result.push_str(&ctx.shell_name),
                Some('i') => result.push_str(&ctx.line_number.to_string()),
                Some('%') => result.push('%'),
                Some(c) => { result.push('%'); result.push(c); }
                None => result.push('%'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Format time using a strftime-like format string.
fn format_time(fmt: &str) -> String {
    // Simple implementation - just return current time as HH:MM:SS
    // In a real implementation, we'd use chrono or similar
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() % 86400;
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Check if the current user is root (on Windows, always false).
fn is_root() -> bool {
    // On Windows, there's no direct equivalent of root
    // We could check for admin privileges, but for now return false
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_prompt() {
        let ctx = PromptContext::default();
        let result = render_prompt("%n@%m %~ %# ", &ctx);
        assert!(result.contains("@"));
        assert!(result.contains("%"));
    }

    #[test]
    fn test_render_exit_code() {
        let mut ctx = PromptContext::default();
        ctx.exit_code = 42;
        let result = render_prompt("exit: %?", &ctx);
        assert_eq!(result, "exit: 42");
    }

    #[test]
    fn test_render_literal_percent() {
        let ctx = PromptContext::default();
        let result = render_prompt("100%%", &ctx);
        assert_eq!(result, "100%");
    }

    #[test]
    fn test_render_bold() {
        let ctx = PromptContext::default();
        let result = render_prompt("%Bbold%b", &ctx);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("\x1b[22m"));
    }

    #[test]
    fn test_render_color() {
        let ctx = PromptContext::default();
        let result = render_prompt("%F{red}error%f", &ctx);
        assert!(result.contains("\x1b[31m"));
        assert!(result.contains("\x1b[39m"));
    }

    #[test]
    fn test_color_parse() {
        assert_eq!(PromptColor::parse("red"), Some(PromptColor::Red));
        assert_eq!(PromptColor::parse("0"), Some(PromptColor::Black));
        assert_eq!(PromptColor::parse("255"), Some(PromptColor::Color256(255)));
        assert_eq!(PromptColor::parse("invalid"), None);
    }

    #[test]
    fn test_prompt_theme_default() {
        let theme = PromptTheme::default();
        assert!(theme.primary.contains("%n"));
        assert!(theme.primary.contains("%~"));
    }
}
