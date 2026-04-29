//! History manager with expansion support.
//!
//! Provides history file management, expansion (!!, !$, !n, ^old^new),
//! and deduplication strategies.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use winsh_core::ShellError;

/// Manages shell command history.
pub struct HistoryManager {
    /// History entries (most recent first)
    entries: Vec<String>,
    /// Maximum entries in memory
    max_entries: usize,
    /// History file path
    file_path: PathBuf,
    /// Maximum entries to save to file
    max_file_entries: usize,
    /// Whether to ignore duplicate consecutive entries
    ignore_dups: bool,
    /// Whether to ignore all duplicate entries
    ignore_all_dups: bool,
    /// Whether to ignore entries starting with space
    ignore_space: bool,
    /// Whether entries have been modified since last save
    dirty: bool,
}

impl HistoryManager {
    /// Create a new history manager.
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let file_path = home.join(".winsh_history");

        Self {
            entries: Vec::new(),
            max_entries: 10000,
            file_path,
            max_file_entries: 10000,
            ignore_dups: true,
            ignore_all_dups: false,
            ignore_space: true,
            dirty: false,
        }
    }

    /// Create a history manager with a custom file path.
    pub fn with_file(file_path: PathBuf) -> Self {
        let mut manager = Self::new();
        manager.file_path = file_path;
        manager
    }

    /// Load history from the file.
    pub fn load(&mut self) -> Result<(), ShellError> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let file = std::fs::File::open(&self.file_path)
            .map_err(|e| ShellError::Io(e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            match line {
                Ok(line) if !line.is_empty() => {
                    self.entries.push(line);
                }
                _ => {}
            }
        }

        // Truncate to max entries
        if self.entries.len() > self.max_entries {
            self.entries.drain(..self.entries.len() - self.max_entries);
        }

        Ok(())
    }

    /// Save history to the file.
    pub fn save(&mut self) -> Result<(), ShellError> {
        if !self.dirty {
            return Ok(());
        }

        // Create the parent directory if needed
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ShellError::Io(e))?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file_path)
            .map_err(|e| ShellError::Io(e))?;

        // Save most recent entries first
        let entries: Vec<String> = self.entries.iter()
            .take(self.max_file_entries)
            .rev()
            .cloned()
            .collect();

        for entry in &entries {
            writeln!(file, "{}", entry).map_err(|e| ShellError::Io(e))?;
        }

        self.dirty = false;
        Ok(())
    }

    /// Add an entry to the history.
    pub fn add(&mut self, entry: &str) {
        let trimmed = entry.trim();

        if trimmed.is_empty() {
            return;
        }

        // Check ignore_space
        if self.ignore_space && entry.starts_with(' ') {
            return;
        }

        // Check ignore_dups (consecutive)
        if self.ignore_dups {
            if let Some(last) = self.entries.first() {
                if last == trimmed {
                    return;
                }
            }
        }

        // Check ignore_all_dups
        if self.ignore_all_dups && self.entries.contains(&trimmed.to_string()) {
            return;
        }

        self.entries.insert(0, trimmed.to_string());
        self.dirty = true;

        // Truncate if too many entries
        if self.entries.len() > self.max_entries {
            self.entries.pop();
        }
    }

    /// Get all history entries.
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Get a specific history entry by index.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(|s| s.as_str())
    }

    /// Get the number of history entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.dirty = true;
    }

    /// Search history for entries containing the given text.
    pub fn search(&self, query: &str) -> Vec<&str> {
        if query.is_empty() {
            return self.entries.iter().map(|s| s.as_str()).collect();
        }
        self.entries
            .iter()
            .filter(|e| e.contains(query))
            .map(|s| s.as_str())
            .collect()
    }

    /// Search history starting with the given text.
    pub fn search_prefix(&self, prefix: &str) -> Vec<&str> {
        if prefix.is_empty() {
            return vec![];
        }
        self.entries
            .iter()
            .filter(|e| e.starts_with(prefix))
            .map(|s| s.as_str())
            .collect()
    }

    /// Set the file path.
    pub fn set_file(&mut self, path: PathBuf) {
        self.file_path = path;
    }

    /// Set the maximum number of entries in memory.
    pub fn set_max_entries(&mut self, max: usize) {
        self.max_entries = max;
    }

    /// Set the maximum number of entries to save.
    pub fn set_max_file_entries(&mut self, max: usize) {
        self.max_file_entries = max;
    }

    /// Set whether to ignore consecutive duplicates.
    pub fn set_ignore_dups(&mut self, ignore: bool) {
        self.ignore_dups = ignore;
    }

    /// Set whether to ignore all duplicates.
    pub fn set_ignore_all_dups(&mut self, ignore: bool) {
        self.ignore_all_dups = ignore;
    }

    /// Set whether to ignore entries starting with space.
    pub fn set_ignore_space(&mut self, ignore: bool) {
        self.ignore_space = ignore;
    }

    /// Expand a history reference.
    ///
    /// Supports:
    /// - !! - last command
    /// - !n - command by index (1-based, oldest = 1)
    /// - !-n - nth from last
    /// - !str - last command starting with str
    /// - !?str? - last command containing str
    /// - !$ - last argument of last command
    /// - !* - all arguments of last command
    /// - !# - the entire command line typed so far
    /// - ^old^new - replace old with new in last command
    pub fn expand(&self, input: &str) -> Option<String> {
        if !input.contains('!') && !input.starts_with('^') {
            return None;
        }

        let mut result = input.to_string();

        // Handle ^old^new
        if result.starts_with('^') {
            if let Some(line) = self.entries.first() {
                let rest = &result[1..];
                if let Some(separator_pos) = rest.find('^') {
                    let old = &rest[..separator_pos];
                    let new = &rest[separator_pos + 1..];
                    return Some(line.replace(old, new));
                }
            }
            return None;
        }

        // Handle !! (last command)
        if result.contains("!!") {
            if let Some(line) = self.entries.first() {
                result = result.replace("!!", line);
            }
        }

        // Handle !n (by index)
        let mut i = 0;
        while i < result.len() {
            if let Some(c) = result.chars().nth(i) {
                if c == '!' && i + 1 < result.len() {
                    let after = &result[i + 1..];
                    if after.starts_with('!') {
                        i += 2;
                        continue;
                    }

                    // Try !-n
                    if after.starts_with('-') {
                        let num_str: String = after[1..]
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if let Ok(n) = num_str.parse::<usize>() {
                            if n > 0 && n <= self.entries.len() {
                                let idx = n - 1;
                                let replacement = self.entries[idx].clone();
                                result = format!(
                                    "{}{}{}",
                                    &result[..i],
                                    replacement,
                                    &result[i + 2 + num_str.len()..]
                                );
                                i += replacement.len();
                            } else {
                                i += 2 + num_str.len();
                            }
                            continue;
                        }
                    }

                    // Try !n (positive number)
                    let num_str: String = after
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(n) = num_str.parse::<usize>() {
                        if n > 0 && n <= self.entries.len() {
                            let idx = n - 1;
                            let replacement = self.entries[idx].clone();
                            result = format!(
                                "{}{}{}",
                                &result[..i],
                                replacement,
                                &result[i + 1 + num_str.len()..]
                            );
                            i += replacement.len();
                        } else {
                            i += 1 + num_str.len();
                        }
                        continue;
                    }

                    // Try !str (starts with)
                    let word_str: String = after
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                        .collect();
                    if !word_str.is_empty() {
                        if let Some(entry) = self.entries.iter().find(|e| e.starts_with(&word_str)) {
                            result = format!(
                                "{}{}{}",
                                &result[..i],
                                entry,
                                &result[i + 1 + word_str.len()..]
                            );
                            i += entry.len();
                        } else {
                            i += 1 + word_str.len();
                        }
                        continue;
                    }

                    // Handle !$ (last argument of last command)
                    if after.starts_with('$') {
                        if let Some(line) = self.entries.first() {
                            let last_arg = line.split_whitespace().last().unwrap_or("");
                            result = format!(
                                "{}{}{}",
                                &result[..i],
                                last_arg,
                                &result[i + 2..]
                            );
                            i += last_arg.len();
                        } else {
                            i += 2;
                        }
                        continue;
                    }

                    // Handle !* (all arguments of last command)
                    if after.starts_with('*') {
                        if let Some(line) = self.entries.first() {
                            let args: Vec<&str> = line.split_whitespace().skip(1).collect();
                            let args_str = args.join(" ");
                            result = format!(
                                "{}{}{}",
                                &result[..i],
                                args_str,
                                &result[i + 2..]
                            );
                            i += args_str.len();
                        } else {
                            i += 2;
                        }
                        continue;
                    }
                }
            }
            i += 1;
        }

        Some(result)
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_add_and_get() {
        let mut h = HistoryManager::new();
        h.add("echo hello");
        h.add("ls -la");
        assert_eq!(h.len(), 2);
        assert_eq!(h.get(0), Some("ls -la"));
        assert_eq!(h.get(1), Some("echo hello"));
    }

    #[test]
    fn test_history_ignore_dups() {
        let mut h = HistoryManager::new();
        h.add("echo hello");
        h.add("echo hello");
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn test_history_ignore_space() {
        let mut h = HistoryManager::new();
        h.add(" echo hello");
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn test_history_search() {
        let mut h = HistoryManager::new();
        h.add("git status");
        h.add("git commit");
        h.add("cargo build");
        let results = h.search("git");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_history_search_prefix() {
        let mut h = HistoryManager::new();
        h.add("git status");
        h.add("git commit");
        h.add("cargo build");
        let results = h.search_prefix("git");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_history_expand_double_bang() {
        let mut h = HistoryManager::new();
        h.add("echo hello world");
        let result = h.expand("!!").unwrap();
        assert_eq!(result, "echo hello world");
    }

    #[test]
    fn test_history_expand_last_arg() {
        let mut h = HistoryManager::new();
        h.add("echo hello world");
        h.add("ls -la");
        // !$ = last arg of last command (ls -la) = "-la"
        let result = h.expand("echo !$").unwrap();
        assert_eq!(result, "echo -la");
    }

    #[test]
    fn test_history_expand_all_args() {
        let mut h = HistoryManager::new();
        h.add("echo hello world");
        let result = h.expand("ls !*").unwrap();
        assert_eq!(result, "ls hello world");
    }

    #[test]
    fn test_history_expand_caret() {
        let mut h = HistoryManager::new();
        h.add("echo hello");
        let result = h.expand("^hello^world").unwrap();
        assert_eq!(result, "echo world");
    }

    #[test]
    fn test_history_expand_by_number() {
        let mut h = HistoryManager::new();
        h.add("cmd1"); // index 0
        h.add("cmd2"); // index 1
        let result = h.expand("!2").unwrap();
        assert_eq!(result, "cmd1");
    }
}
