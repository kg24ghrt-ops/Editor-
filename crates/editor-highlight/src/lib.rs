//! Incremental syntax highlighting using tree‑sitter.

use editor_core::EditorBuffer;
use tree_sitter::{Parser, Tree};
use tree_sitter_highlight::{Highlighter, HighlightConfiguration, HighlightEvent};
use std::sync::Arc;

/// Manages parsing and highlighting for a single buffer.
pub struct SyntaxHighlighter {
    parser: Parser,
    tree: Option<Tree>,
    /// The last buffer version we parsed.
    last_version: u64,
    /// Language-specific configuration (if any).
    config: Option<Arc<HighlightConfiguration>>,
}

impl SyntaxHighlighter {
    /// Create a new highlighter with a given language.
    pub fn new(language: tree_sitter::Language) -> Self {
        let mut parser = Parser::new();
        parser.set_language(language).expect("Invalid language");
        Self {
            parser,
            tree: None,
            last_version: 0,
            config: None,
        }
    }

    /// Set a highlight configuration (for syntax highlighting).
    pub fn set_highlight_config(&mut self, config: HighlightConfiguration) {
        self.config = Some(Arc::new(config));
    }

    /// Update the parser with the buffer's content, reusing the old tree if possible.
    /// Returns the new syntax tree (root node).
    pub fn parse(&mut self, buf: &EditorBuffer) -> &Tree {
        let rope = buf.rope();
        let source = rope.to_string(); // FIXME: materializes whole buffer; for demo only.
        let old_tree = self.tree.take();

        let new_tree = if let Some(_old_tree) = old_tree {
            // If we have a tree from a previous version, we need to apply edits.
            // For simplicity, we just reparse from scratch.
            // A real implementation would maintain a map of edits and call tree.edit().
            self.parser.parse(source, None).unwrap()
        } else {
            self.parser.parse(source, None).unwrap()
        };

        self.tree = Some(new_tree);
        self.last_version = buf.version;
        self.tree.as_ref().unwrap()
    }

    /// Perform highlighting. Returns a vector of HighlightEvents.
    pub fn highlight(&self, buf: &EditorBuffer) -> Vec<HighlightEvent> {
        if let Some(config) = &self.config {
            let source = buf.rope().to_string();
            let highlighter = Highlighter::new();
            // highlighter.highlight() returns Result<impl Iterator<Item = Result<HighlightEvent, Error>>, Error>
            let events: Vec<HighlightEvent> = highlighter
                .highlight(config, source.as_bytes(), None, |_lang| None)
                .unwrap()                    // Unwrap the outer Result
                .filter_map(Result::ok)      // Keep only successful events
                .collect();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    // For a test we'd need a real language; this is a stub.
}