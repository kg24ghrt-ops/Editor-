//! In‑memory text buffer with cursor, selection, and version counter.

use ropey::Rope;
use std::ops::Range;

/// Represents a single cursor/selection.
/// Multi‑selection is a vector of these.
#[derive(Clone, Debug, Default)]
pub struct Cursor {
    /// Character index of the cursor (or anchor of selection).
    pub head: usize,
    /// Character index of the other end of the selection (if any).
    /// If `tail == head`, there is no selection.
    pub tail: usize,
}

impl Cursor {
    pub fn new(head: usize) -> Self {
        Self { head, tail: head }
    }

    /// Returns the selected range if any, otherwise `None`.
    pub fn selection(&self) -> Option<Range<usize>> {
        if self.head == self.tail {
            None
        } else {
            let (start, end) = if self.head < self.tail {
                (self.head, self.tail)
            } else {
                (self.tail, self.head)
            };
            Some(start..end)
        }
    }

    /// Moves the cursor to a new position, clearing any selection.
    pub fn move_to(&mut self, pos: usize) {
        self.head = pos;
        self.tail = pos;
    }
}

/// The core editor buffer.
pub struct EditorBuffer {
    rope: Rope,
    cursors: Vec<Cursor>,
    /// Monotonically increasing version number; incremented on every mutation.
    pub version: u64,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self {
            rope: Rope::new(),
            cursors: vec![Cursor::default()],
            version: 0,
        }
    }
}

impl EditorBuffer {
    /// Creates a new buffer with the given text.
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from(text),
            cursors: vec![Cursor::new(0)],
            version: 0,
        }
    }

    /// Returns the total number of characters (Unicode scalar values).
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Returns the text as a string (materialises the whole buffer – use only for testing or small buffers).
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Immutable access to the underlying rope.
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    /// Returns a snapshot (cheap clone) of the rope.
    pub fn snapshot(&self) -> Rope {
        self.rope.clone()
    }

    /// Returns the current version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Insert text at the given character index. Updates version.
    pub fn insert_text(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
        self.version += 1;

        let len = text.chars().count();
        for cursor in &mut self.cursors {
            if cursor.head >= char_idx {
                cursor.head += len;
            }
            if cursor.tail >= char_idx {
                cursor.tail += len;
            }
        }
    }

    /// Delete the range [start, end). Updates version.
    pub fn delete_range(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let len = end - start;
        self.rope.remove(start..end);
        self.version += 1;

        for cursor in &mut self.cursors {
            if cursor.head >= start {
                if cursor.head < end {
                    cursor.head = start;
                } else {
                    cursor.head -= len;
                }
            }
            if cursor.tail >= start {
                if cursor.tail < end {
                    cursor.tail = start;
                } else {
                    cursor.tail -= len;
                }
            }
        }
    }

    /// Returns a reference to the current cursors.
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Returns a mutable reference to the current cursors.
    pub fn cursors_mut(&mut self) -> &mut Vec<Cursor> {
        &mut self.cursors
    }

    /// Sets the cursors to a single position.
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursors.clear();
        self.cursors.push(Cursor::new(pos));
    }

    /// Adds a new cursor at the given position.
    pub fn add_cursor(&mut self, pos: usize) {
        self.cursors.push(Cursor::new(pos));
    }

    /// Removes the cursor at the given index.
    pub fn remove_cursor(&mut self, index: usize) {
        if index < self.cursors.len() {
            self.cursors.remove(index);
        }
    }

    /// Clears all cursors and adds a default one at position 0.
    pub fn clear_cursors(&mut self) {
        self.cursors.clear();
        self.cursors.push(Cursor::new(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_delete() {
        let mut buf = EditorBuffer::new("hello");
        buf.insert_text(5, " world");
        assert_eq!(buf.text(), "hello world");
        buf.delete_range(5, 11);
        assert_eq!(buf.text(), "hello");
    }

    #[test]
    fn cursor_adjustment() {
        let mut buf = EditorBuffer::new("hello");
        buf.set_cursor(3);
        buf.insert_text(5, " world");
        // Cursor at 3 should not move.
        assert_eq!(buf.cursors()[0].head, 3);
        buf.set_cursor(5);
        buf.insert_text(5, "XX");
        // Cursor at 5 should move to 7.
        assert_eq!(buf.cursors()[0].head, 7);
    }

    #[test]
    fn multi_cursor() {
        let mut buf = EditorBuffer::new("hello world");
        buf.set_cursor(0);
        buf.add_cursor(6);
        // Insert at beginning affects both cursors.
        buf.insert_text(0, "Hi ");
        let cursors = buf.cursors();
        assert_eq!(cursors[0].head, 3);
        assert_eq!(cursors[1].head, 9);
    }
}