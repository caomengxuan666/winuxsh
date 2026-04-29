//! Keybinding system with Vi and Emacs modes.
//!
//! Provides keybinding configuration, mode switching, and widget support.

use std::collections::HashMap;

/// Editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditingMode {
    /// Emacs mode (default)
    Emacs,
    /// Vi insert mode
    ViInsert,
    /// Vi command/normal mode
    ViCommand,
}

impl Default for EditingMode {
    fn default() -> Self {
        Self::Emacs
    }
}

/// A key binding.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// The key sequence
    pub key: String,
    /// The widget/action name
    pub widget: String,
    /// The mode this binding applies to
    pub mode: EditingMode,
}

/// Manages key bindings.
pub struct KeyBindings {
    /// Bindings organized by mode and key
    bindings: HashMap<EditingMode, HashMap<String, String>>,
    /// Current editing mode
    mode: EditingMode,
    /// Vi insert mode bindings
    vi_insert_bindings: HashMap<String, String>,
    /// Vi command mode bindings
    vi_command_bindings: HashMap<String, String>,
}

impl KeyBindings {
    /// Create a new keybindings manager with default Emacs bindings.
    pub fn new() -> Self {
        let mut kb = Self {
            bindings: HashMap::new(),
            mode: EditingMode::Emacs,
            vi_insert_bindings: HashMap::new(),
            vi_command_bindings: HashMap::new(),
        };

        kb.setup_emacs_defaults();
        kb.setup_vi_defaults();
        kb
    }

    /// Setup default Emacs keybindings.
    fn setup_emacs_defaults(&mut self) {
        let mut emacs = HashMap::new();

        // Movement
        emacs.insert("Ctrl-A".to_string(), "beginning-of-line".to_string());
        emacs.insert("Ctrl-E".to_string(), "end-of-line".to_string());
        emacs.insert("Ctrl-B".to_string(), "backward-char".to_string());
        emacs.insert("Ctrl-F".to_string(), "forward-char".to_string());
        emacs.insert("Ctrl-P".to_string(), "previous-history".to_string());
        emacs.insert("Ctrl-N".to_string(), "next-history".to_string());
        emacs.insert("Left".to_string(), "backward-char".to_string());
        emacs.insert("Right".to_string(), "forward-char".to_string());
        emacs.insert("Up".to_string(), "previous-history".to_string());
        emacs.insert("Down".to_string(), "next-history".to_string());

        // Word movement
        emacs.insert("Alt-B".to_string(), "backward-word".to_string());
        emacs.insert("Alt-F".to_string(), "forward-word".to_string());
        emacs.insert("Ctrl-Left".to_string(), "backward-word".to_string());
        emacs.insert("Ctrl-Right".to_string(), "forward-word".to_string());

        // Editing
        emacs.insert("Ctrl-D".to_string(), "delete-char".to_string());
        emacs.insert("Ctrl-H".to_string(), "backward-delete-char".to_string());
        emacs.insert("Backspace".to_string(), "backward-delete-char".to_string());
        emacs.insert("Ctrl-K".to_string(), "kill-line".to_string());
        emacs.insert("Ctrl-U".to_string(), "backward-kill-line".to_string());
        emacs.insert("Ctrl-W".to_string(), "backward-kill-word".to_string());
        emacs.insert("Alt-D".to_string(), "kill-word".to_string());
        emacs.insert("Ctrl-Y".to_string(), "yank".to_string());
        emacs.insert("Alt-Y".to_string(), "yank-pop".to_string());

        // History
        emacs.insert("Ctrl-R".to_string(), "history-search-backward".to_string());
        emacs.insert("Ctrl-S".to_string(), "history-search-forward".to_string());

        // Completion
        emacs.insert("Tab".to_string(), "complete".to_string());
        emacs.insert("Ctrl-I".to_string(), "complete".to_string());

        // Misc
        emacs.insert("Ctrl-L".to_string(), "clear-screen".to_string());
        emacs.insert("Ctrl-C".to_string(), "interrupt".to_string());
        emacs.insert("Ctrl-D".to_string(), "delete-char-or-eof".to_string());
        emacs.insert("Ctrl-J".to_string(), "accept-line".to_string());
        emacs.insert("Enter".to_string(), "accept-line".to_string());
        emacs.insert("Ctrl-Z".to_string(), "suspend".to_string());
        emacs.insert("Ctrl-T".to_string(), "transpose-chars".to_string());
        emacs.insert("Alt-T".to_string(), "transpose-words".to_string());
        emacs.insert("Alt-U".to_string(), "upcase-word".to_string());
        emacs.insert("Alt-L".to_string(), "downcase-word".to_string());
        emacs.insert("Alt-C".to_string(), "capitalize-word".to_string());

        // Home/End
        emacs.insert("Home".to_string(), "beginning-of-line".to_string());
        emacs.insert("End".to_string(), "end-of-line".to_string());
        emacs.insert("Ctrl-Home".to_string(), "beginning-of-buffer".to_string());
        emacs.insert("Ctrl-End".to_string(), "end-of-buffer".to_string());

        // Delete
        emacs.insert("Delete".to_string(), "delete-char".to_string());
        emacs.insert("Ctrl-Delete".to_string(), "kill-word".to_string());
        emacs.insert("Ctrl-Backspace".to_string(), "backward-kill-word".to_string());

        self.bindings.insert(EditingMode::Emacs, emacs);
    }

    /// Setup default Vi keybindings.
    fn setup_vi_defaults(&mut self) {
        // Vi insert mode
        let mut vi_insert = HashMap::new();
        vi_insert.insert("Escape".to_string(), "vi-cmd-mode".to_string());
        vi_insert.insert("Ctrl-C".to_string(), "vi-cmd-mode".to_string());
        vi_insert.insert("Ctrl-H".to_string(), "backward-delete-char".to_string());
        vi_insert.insert("Backspace".to_string(), "backward-delete-char".to_string());
        vi_insert.insert("Ctrl-W".to_string(), "backward-kill-word".to_string());
        vi_insert.insert("Ctrl-U".to_string(), "backward-kill-line".to_string());
        vi_insert.insert("Tab".to_string(), "complete".to_string());
        vi_insert.insert("Enter".to_string(), "accept-line".to_string());
        vi_insert.insert("Ctrl-J".to_string(), "accept-line".to_string());
        self.vi_insert_bindings = vi_insert;

        // Vi command mode
        let mut vi_command = HashMap::new();
        // Movement
        vi_command.insert("h".to_string(), "backward-char".to_string());
        vi_command.insert("l".to_string(), "forward-char".to_string());
        vi_command.insert("j".to_string(), "previous-history".to_string());
        vi_command.insert("k".to_string(), "next-history".to_string());
        vi_command.insert("H".to_string(), "beginning-of-line".to_string());
        vi_command.insert("L".to_string(), "end-of-line".to_string());
        vi_command.insert("w".to_string(), "forward-word".to_string());
        vi_command.insert("b".to_string(), "backward-word".to_string());
        vi_command.insert("e".to_string(), "forward-word-end".to_string());
        vi_command.insert("0".to_string(), "beginning-of-line".to_string());
        vi_command.insert("$".to_string(), "end-of-line".to_string());
        vi_command.insert("gg".to_string(), "beginning-of-buffer".to_string());
        vi_command.insert("G".to_string(), "end-of-buffer".to_string());

        // Editing
        vi_command.insert("x".to_string(), "delete-char".to_string());
        vi_command.insert("X".to_string(), "backward-delete-char".to_string());
        vi_command.insert("dd".to_string(), "kill-line".to_string());
        vi_command.insert("dw".to_string(), "kill-word".to_string());
        vi_command.insert("db".to_string(), "backward-kill-word".to_string());
        vi_command.insert("D".to_string(), "kill-to-end-of-line".to_string());
        vi_command.insert("C".to_string(), "kill-to-end-of-line-insert".to_string());
        vi_command.insert("cw".to_string(), "change-word".to_string());
        vi_command.insert("cc".to_string(), "change-line".to_string());

        // Mode switching
        vi_command.insert("i".to_string(), "vi-insert-mode".to_string());
        vi_command.insert("I".to_string(), "vi-insert-beginning".to_string());
        vi_command.insert("a".to_string(), "vi-append-mode".to_string());
        vi_command.insert("A".to_string(), "vi-append-eol".to_string());
        vi_command.insert("o".to_string(), "vi-open-line-below".to_string());
        vi_command.insert("O".to_string(), "vi-open-line-above".to_string());

        // Misc
        vi_command.insert("u".to_string(), "undo".to_string());
        vi_command.insert("Ctrl-R".to_string(), "redo".to_string());
        vi_command.insert("p".to_string(), "yank".to_string());
        vi_command.insert("P".to_string(), "yank-before".to_string());
        vi_command.insert("yy".to_string(), "vi-yank-line".to_string());
        vi_command.insert("yw".to_string(), "vi-yank-word".to_string());
        vi_command.insert("/".to_string(), "history-search-backward".to_string());
        vi_command.insert("?".to_string(), "history-search-forward".to_string());
        vi_command.insert("n".to_string(), "history-search-repeat-forward".to_string());
        vi_command.insert("N".to_string(), "history-search-repeat-backward".to_string());

        self.vi_command_bindings = vi_command;
    }

    /// Get the current editing mode.
    pub fn mode(&self) -> EditingMode {
        self.mode
    }

    /// Set the editing mode.
    pub fn set_mode(&mut self, mode: EditingMode) {
        self.mode = mode;
    }

    /// Look up a widget for a key in the current mode.
    pub fn lookup(&self, key: &str) -> Option<&str> {
        match self.mode {
            EditingMode::Emacs => {
                if let Some(emacs) = self.bindings.get(&EditingMode::Emacs) {
                    emacs.get(key).map(|s: &String| s.as_str())
                } else {
                    None
                }
            }
            EditingMode::ViInsert => {
                self.vi_insert_bindings.get(key).map(|s: &String| s.as_str())
            }
            EditingMode::ViCommand => {
                self.vi_command_bindings.get(key).map(|s: &String| s.as_str())
            }
        }
    }

    /// Bind a key to a widget in the current mode.
    pub fn bind(&mut self, key: &str, widget: &str, mode: Option<EditingMode>) {
        let mode = mode.unwrap_or(self.mode);
        match mode {
            EditingMode::Emacs => {
                if let Some(emacs) = self.bindings.get_mut(&EditingMode::Emacs) {
                    emacs.insert(key.to_string(), widget.to_string());
                }
            }
            EditingMode::ViInsert => {
                self.vi_insert_bindings.insert(key.to_string(), widget.to_string());
            }
            EditingMode::ViCommand => {
                self.vi_command_bindings.insert(key.to_string(), widget.to_string());
            }
        }
    }

    /// List all built-in widgets.
    pub fn builtin_widgets() -> Vec<&'static str> {
        vec![
            "accept-line",
            "backward-char",
            "backward-delete-char",
            "backward-kill-line",
            "backward-kill-word",
            "backward-word",
            "beginning-of-buffer",
            "beginning-of-line",
            "capitalize-word",
            "clear-screen",
            "complete",
            "delete-char",
            "delete-char-or-eof",
            "downcase-word",
            "end-of-buffer",
            "end-of-line",
            "forward-char",
            "forward-word",
            "history-search-backward",
            "history-search-forward",
            "interrupt",
            "kill-line",
            "kill-word",
            "next-history",
            "previous-history",
            "redo",
            "suspend",
            "transpose-chars",
            "transpose-words",
            "undo",
            "upcase-word",
            "vi-append-mode",
            "vi-cmd-mode",
            "vi-insert-mode",
            "vi-open-line-below",
            "yank",
            "yank-pop",
        ]
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emacs_lookup() {
        let kb = KeyBindings::new();
        assert_eq!(kb.lookup("Ctrl-A"), Some("beginning-of-line"));
        assert_eq!(kb.lookup("Ctrl-E"), Some("end-of-line"));
        assert_eq!(kb.lookup("Ctrl-K"), Some("kill-line"));
    }

    #[test]
    fn test_vi_insert_lookup() {
        let mut kb = KeyBindings::new();
        kb.set_mode(EditingMode::ViInsert);
        assert_eq!(kb.lookup("Escape"), Some("vi-cmd-mode"));
    }

    #[test]
    fn test_vi_command_lookup() {
        let mut kb = KeyBindings::new();
        kb.set_mode(EditingMode::ViCommand);
        assert_eq!(kb.lookup("h"), Some("backward-char"));
        assert_eq!(kb.lookup("i"), Some("vi-insert-mode"));
    }

    #[test]
    fn test_bind_custom_key() {
        let mut kb = KeyBindings::new();
        kb.bind("Ctrl-T", "transpose-chars", None);
        assert_eq!(kb.lookup("Ctrl-T"), Some("transpose-chars"));
    }

    #[test]
    fn test_switch_mode() {
        let mut kb = KeyBindings::new();
        assert_eq!(kb.mode(), EditingMode::Emacs);

        kb.set_mode(EditingMode::ViInsert);
        assert_eq!(kb.mode(), EditingMode::ViInsert);

        kb.set_mode(EditingMode::ViCommand);
        assert_eq!(kb.mode(), EditingMode::ViCommand);
    }

    #[test]
    fn test_builtin_widgets() {
        let widgets = KeyBindings::builtin_widgets();
        assert!(widgets.contains(&"accept-line"));
        assert!(widgets.contains(&"complete"));
        assert!(widgets.contains(&"yank"));
    }
}
