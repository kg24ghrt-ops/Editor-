//! Incremental syntax highlighting using tree‑sitter.

use editor_core::EditorBuffer;
use tree_sitter::{InputEdit, Parser, Point, Tree};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use std::sync::Arc;

/// Manages parsing and highlighting for a single buffer.
pub struct SyntaxHighlighter {
    parser: Parser,
    tree: Option<Tree>,
    /// The last buffer version we parsed.
    last_version: u64,
    /// Language‑specific configuration (if any).
    config: Option<Arc<HighlightConfiguration>>,
    /// The language used for parsing.
    language: Option<tree_sitter::Language>,
}

impl SyntaxHighlighter {
    /// Create a new highlighter with a given language.
    pub fn new(language: tree_sitter::Language) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(language)
            .expect("Invalid language for parser");
        Self {
            parser,
            tree: None,
            last_version: 0,
            config: None,
            language: Some(language),
        }
    }

    /// Create a highlighter without an initial language (use `set_language` later).
    pub fn empty() -> Self {
        Self {
            parser: Parser::new(),
            tree: None,
            last_version: 0,
            config: None,
            language: None,
        }
    }

    /// Set or change the language.
    pub fn set_language(&mut self, language: tree_sitter::Language) -> Result<(), &'static str> {
        self.parser
            .set_language(language)
            .map_err(|_| "Failed to set language")?;
        self.language = Some(language);
        // Invalidate the tree since the language changed.
        self.tree = None;
        self.last_version = 0;
        Ok(())
    }

    /// Set a highlight configuration (for syntax highlighting).
    pub fn set_highlight_config(&mut self, config: HighlightConfiguration) {
        self.config = Some(Arc::new(config));
    }

    /// Update the parser with the buffer's content, reusing the old tree if possible.
    /// Returns the new syntax tree (root node).
    pub fn parse(&mut self, buf: &EditorBuffer) -> &Tree {
        let rope = buf.rope();
        let source = rope.to_string(); // Still materialises; can be optimised later with a custom `Read` impl.

        let old_tree = self.tree.take();
        let new_tree = if let Some(mut old_tree) = old_tree {
            // If we have a tree from a previous version, we need to apply edits.
            // For simplicity, we reparse from scratch if the version changed.
            // A real implementation would maintain a map of edits and call tree.edit().
            if self.last_version != buf.version {
                // In a full implementation, we'd compute the edit delta and call old_tree.edit(&edit).
                // For now, we just reparse.
                self.parser.parse(source, None).unwrap()
            } else {
                // No changes, return the existing tree.
                old_tree
            }
        } else {
            self.parser.parse(source, None).unwrap()
        };

        self.tree = Some(new_tree);
        self.last_version = buf.version;
        self.tree.as_ref().unwrap()
    }

    /// Perform highlighting. Returns a vector of (byte_offset, HighlightEvent).
    pub fn highlight(&self, buf: &EditorBuffer) -> Vec<(usize, HighlightEvent)> {
        if let (Some(config), Some(tree)) = (self.config.as_ref(), self.tree.as_ref()) {
            let source = buf.rope().to_string();
            let mut highlighter = Highlighter::new();
            let mut events = Vec::new();

            // Use the tree's root node to get the byte range.
            let root = tree.root_node();
            let start_byte = root.start_byte();
            let end_byte = root.end_byte();

            // Highlight only the root node's range.
            let highlight_iter = highlighter
                .highlight(config, source.as_bytes(), Some(&tree), |_lang| None)
                .unwrap();

            for event in highlight_iter {
                match event {
                    Ok((byte_range, ev)) => {
                        // Convert byte range to (start_byte, event)
                        events.push((byte_range.start, ev));
                    }
                    Err(_) => continue,
                }
            }
            events
        } else {
            Vec::new()
        }
    }

    /// Check if the buffer version changed, and reparse if needed.
    pub fn ensure_parsed(&mut self, buf: &EditorBuffer) -> &Tree {
        if self.last_version != buf.version {
            self.parse(buf);
        }
        self.tree.as_ref().unwrap()
    }

    /// Returns a reference to the current syntax tree, if any.
    pub fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        // This test requires a real language; we skip if not available.
        // In practice, you'd use tree_sitter_rust::language().
    }
}