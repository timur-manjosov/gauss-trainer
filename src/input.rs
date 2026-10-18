//! A small single-line editor and the command history behind the command bar.
//!
//! Positions are measured in `char`s, so editing is correct for umlauts and
//! other multi-byte characters.

/// Longest line the editor accepts, in characters.
pub const MAX_LINE_LEN: usize = 400;

/// A single-line text buffer with a cursor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineEditor {
    text: String,
    cursor: usize,
}

impl LineEditor {
    /// An empty editor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The cursor position in characters, `0..=len`.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn byte_index(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map_or(self.text.len(), |(i, _)| i)
    }

    /// Replaces the content and moves the cursor to the end.
    pub fn set(&mut self, text: &str) {
        self.text = text.chars().take(MAX_LINE_LEN).collect();
        self.cursor = self.len();
    }

    /// Empties the buffer.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Empties the buffer and returns its previous content.
    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.text)
    }

    /// Inserts `c` at the cursor. Control characters and overflow are ignored.
    pub fn insert(&mut self, c: char) {
        if c.is_control() || self.len() >= MAX_LINE_LEN {
            return;
        }
        let at = self.byte_index(self.cursor);
        self.text.insert(at, c);
        self.cursor += 1;
    }

    /// Deletes the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let at = self.byte_index(self.cursor);
        self.text.remove(at);
    }

    /// Deletes the character under the cursor.
    pub fn delete(&mut self) {
        if self.cursor < self.len() {
            let at = self.byte_index(self.cursor);
            self.text.remove(at);
        }
    }

    /// Deletes the word before the cursor (and the spaces between).
    pub fn delete_word_back(&mut self) {
        while self.cursor > 0 && self.char_before_cursor() == Some(' ') {
            self.backspace();
        }
        while self.cursor > 0 && self.char_before_cursor() != Some(' ') {
            self.backspace();
        }
    }

    fn char_before_cursor(&self) -> Option<char> {
        self.text.chars().nth(self.cursor.checked_sub(1)?)
    }

    /// Moves the cursor one character left.
    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the cursor one character right.
    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len());
    }

    /// Moves the cursor to the start.
    pub fn home(&mut self) {
        self.cursor = 0;
    }

    /// Moves the cursor to the end.
    pub fn end(&mut self) {
        self.cursor = self.len();
    }
}

/// Previously submitted command lines, navigable with the arrow keys.
#[derive(Debug, Clone, Default)]
pub struct CommandHistory {
    entries: Vec<String>,
    /// Index of the entry currently shown, or `None` while editing a fresh line.
    position: Option<usize>,
    /// What the user had typed before starting to browse.
    draft: String,
}

impl CommandHistory {
    /// An empty history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no command has been stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Stores a submitted line (blank lines and immediate repeats are skipped).
    pub fn push(&mut self, line: &str) {
        let line = line.trim();
        if !line.is_empty() && self.entries.last().map(String::as_str) != Some(line) {
            self.entries.push(line.to_owned());
        }
        self.position = None;
        self.draft.clear();
    }

    /// Steps back in time; `current` is remembered as the draft on the first step.
    pub fn older(&mut self, current: &str) -> Option<&str> {
        let next = match self.position {
            None if self.entries.is_empty() => return None,
            None => {
                self.draft = current.to_owned();
                self.entries.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.position = Some(next);
        self.entries.get(next).map(String::as_str)
    }

    /// Steps forward in time; after the newest entry the draft is returned.
    pub fn newer(&mut self) -> Option<&str> {
        let i = self.position?;
        if i + 1 < self.entries.len() {
            self.position = Some(i + 1);
            self.entries.get(i + 1).map(String::as_str)
        } else {
            self.position = None;
            Some(self.draft.as_str())
        }
    }

    /// Forgets the browsing position (called when the user edits the line).
    pub fn stop_browsing(&mut self) {
        self.position = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> LineEditor {
        let mut e = LineEditor::new();
        for c in s.chars() {
            e.insert(c);
        }
        e
    }

    #[test]
    fn inserts_and_moves() {
        let mut e = typed("m 1 2");
        assert_eq!(e.text(), "m 1 2");
        assert_eq!(e.cursor(), 5);
        e.home();
        e.insert('x');
        assert_eq!(e.text(), "xm 1 2");
        e.end();
        e.left();
        e.left();
        e.insert('!');
        assert_eq!(e.text(), "xm 1! 2");
    }

    #[test]
    fn edits_multibyte_characters() {
        let mut e = typed("lösung");
        for _ in 0..4 {
            e.left();
        }
        e.backspace();
        assert_eq!(e.text(), "lsung");
        e.delete();
        assert_eq!(e.text(), "lung");
        assert_eq!(e.cursor(), 1);
    }

    #[test]
    fn backspace_and_delete_at_the_borders_are_noops() {
        let mut e = typed("ab");
        e.end();
        e.delete();
        e.home();
        e.backspace();
        e.left();
        assert_eq!((e.text(), e.cursor()), ("ab", 0));
        e.end();
        e.right();
        assert_eq!(e.cursor(), 2);
    }

    #[test]
    fn deletes_words_backwards() {
        let mut e = typed("a -2  1 2");
        e.delete_word_back();
        assert_eq!(e.text(), "a -2  1 ");
        e.delete_word_back();
        assert_eq!(e.text(), "a -2  ");
        e.delete_word_back();
        assert_eq!(e.text(), "a ");
    }

    #[test]
    fn ignores_control_characters_and_limits_length() {
        let mut e = LineEditor::new();
        e.insert('\n');
        e.insert('\t');
        assert!(e.is_empty());
        for _ in 0..MAX_LINE_LEN + 10 {
            e.insert('x');
        }
        assert_eq!(e.text().chars().count(), MAX_LINE_LEN);
    }

    #[test]
    fn take_and_set() {
        let mut e = typed("abc");
        assert_eq!(e.take(), "abc");
        assert!(e.is_empty());
        e.set("neu");
        assert_eq!((e.text(), e.cursor()), ("neu", 3));
        e.clear();
        assert!(e.is_empty());
    }

    #[test]
    fn history_navigates_and_restores_the_draft() {
        let mut h = CommandHistory::new();
        assert_eq!(h.older("x"), None);
        h.push("m 1 2");
        h.push("s 1 2");
        h.push("s 1 2");
        h.push("   ");
        assert_eq!(h.len(), 2);
        assert_eq!(h.older("draft"), Some("s 1 2"));
        assert_eq!(h.older("ignored"), Some("m 1 2"));
        assert_eq!(h.older("ignored"), Some("m 1 2"));
        assert_eq!(h.newer(), Some("s 1 2"));
        assert_eq!(h.newer(), Some("draft"));
        assert_eq!(h.newer(), None);
    }

    #[test]
    fn pushing_resets_browsing() {
        let mut h = CommandHistory::new();
        h.push("a");
        h.older("");
        h.push("b");
        assert_eq!(h.older(""), Some("b"));
        h.stop_browsing();
        assert_eq!(h.newer(), None);
    }
}
