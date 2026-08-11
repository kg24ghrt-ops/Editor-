//! Delta‑based undo/redo with coalescing of rapid edits.

use editor_core::EditorBuffer;

/// A single edit delta: either insertion or deletion.
#[derive(Clone, Debug)]
pub enum Delta {
    Insert { start: usize, text: String },
    /// Delete now stores the removed text, so undo can re-insert it.
    Delete { start: usize, end: usize, text: String },
}

impl Delta {
    /// Applies the delta to the buffer (forward).
    pub fn apply(&self, buf: &mut EditorBuffer) {
        match self {
            Delta::Insert { start, text } => buf.insert_text(*start, text),
            Delta::Delete { start, end, text: _ } => buf.delete_range(*start, *end),
        }
    }

    /// Reverses the delta (for undo).
    pub fn reverse(&self) -> Self {
        match self {
            Delta::Insert { start, text } => {
                let end = *start + text.chars().count();
                Delta::Delete {
                    start: *start,
                    end,
                    text: text.clone(),
                }
            }
            Delta::Delete { start, end, text } => {
                Delta::Insert {
                    start: *start,
                    text: text.clone(),
                }
            }
        }
    }
}

/// A history entry is a group of deltas (e.g., a single keystroke or a paste).
pub struct HistoryEntry {
    deltas: Vec<Delta>,
    /// Optional name for the operation.
    pub name: String,
}

impl HistoryEntry {
    pub fn new(deltas: Vec<Delta>, name: impl Into<String>) -> Self {
        Self {
            deltas,
            name: name.into(),
        }
    }

    pub fn apply_forward(&self, buf: &mut EditorBuffer) {
        for delta in &self.deltas {
            delta.apply(buf);
        }
    }

    pub fn apply_backward(&self, buf: &mut EditorBuffer) {
        for delta in self.deltas.iter().rev() {
            delta.reverse().apply(buf);
        }
    }
}

/// Undo/redo manager.
pub struct History {
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    /// Maximum number of entries (0 = unlimited).
    max_entries: usize,
    /// Coalescing: if true, the next edit will be added to the last entry if possible.
    pub coalescing: bool,
}

impl History {
    pub fn new(max_entries: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_entries,
            coalescing: true,
        }
    }

    /// Pushes a new entry. If coalescing is enabled and the last entry matches the operation,
    /// we merge the new deltas into it.
    pub fn push_entry(&mut self, entry: HistoryEntry) {
        if self.coalescing {
            if let Some(last) = self.undo_stack.last_mut() {
                // Heuristic: if the last entry has the same name and is within a short time?
                // For simplicity, we just merge if the name is "typing".
                if last.name == entry.name && last.name == "typing" {
                    last.deltas.extend(entry.deltas);
                    return;
                }
            }
        }
        self.undo_stack.push(entry);
        self.redo_stack.clear();
        if self.max_entries > 0 && self.undo_stack.len() > self.max_entries {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last entry. Returns true if an undo was performed.
    pub fn undo(&mut self, buf: &mut EditorBuffer) -> bool {
        if let Some(entry) = self.undo_stack.pop() {
            entry.apply_backward(buf);
            self.redo_stack.push(entry);
            true
        } else {
            false
        }
    }

    /// Redo the last undone entry. Returns true if a redo was performed.
    pub fn redo(&mut self, buf: &mut EditorBuffer) -> bool {
        if let Some(entry) = self.redo_stack.pop() {
            entry.apply_forward(buf);
            self.undo_stack.push(entry);
            true
        } else {
            false
        }
    }

    /// Clears both stacks.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_simple() {
        let mut buf = EditorBuffer::new("hello");
        let mut history = History::new(10);
        let insert = Delta::Insert {
            start: 5,
            text: " world".into(),
        };
        let entry = HistoryEntry::new(vec![insert], "typing");
        entry.apply_forward(&mut buf);
        history.push_entry(entry);
        assert_eq!(buf.text(), "hello world");
        assert!(history.undo(&mut buf));
        assert_eq!(buf.text(), "hello");
        assert!(history.redo(&mut buf));
        assert_eq!(buf.text(), "hello world");
    }

    #[test]
    fn undo_delete() {
        let mut buf = EditorBuffer::new("hello world");
        let mut history = History::new(10);
        let delete = Delta::Delete {
            start: 5,
            end: 11,
            text: " world".into(),
        };
        let entry = HistoryEntry::new(vec![delete], "delete");
        entry.apply_forward(&mut buf);
        history.push_entry(entry);
        assert_eq!(buf.text(), "hello");
        assert!(history.undo(&mut buf));
        assert_eq!(buf.text(), "hello world");
        assert!(history.redo(&mut buf));
        assert_eq!(buf.text(), "hello");
    }
}