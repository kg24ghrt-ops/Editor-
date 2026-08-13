//! Fold regions derived from the syntax tree.

use editor_core::EditorBuffer;
use tree_sitter::{Node, Query, QueryCursor, Tree};
use std::ops::Range;

/// A foldable region, typically a block or function body.
#[derive(Debug, Clone)]
pub struct FoldRegion {
    /// Character range of the folded region (should be a contiguous block).
    pub range: Range<usize>,
    /// Optional display name (like function name).
    pub label: String,
}

/// Extracts foldable regions from a syntax tree using a query.
pub fn fold_regions_from_tree(tree: &Tree, buf: &EditorBuffer) -> Vec<FoldRegion> {
    let root = tree.root_node();
    let language = root.language();

    // Define a query for foldable nodes.
    let query_source = r#"
        (block) @fold
        (function_definition) @fold
        (if_statement) @fold
        (for_statement) @fold
        (while_statement) @fold
        (match_expression) @fold
    "#;

    let query = Query::new(language, query_source).unwrap();
    let mut cursor = QueryCursor::new();

    // Get source text from the buffer as bytes.
    let source_text = buf.rope().to_string();
    let source_bytes = source_text.as_bytes();

    let matches = cursor.matches(&query, root, source_bytes);
    let rope = buf.rope();

    let mut regions = Vec::new();
    for mat in matches {
        for capture in mat.captures {
            let node = capture.node;
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let start_char = rope.byte_to_char(start_byte);
            let end_char = rope.byte_to_char(end_byte);

            if end_char > start_char + 1 {
                let label = if node.kind() == "function_definition" {
                    extract_function_name(&node, source_bytes)
                } else {
                    node.kind().to_string()
                };
                regions.push(FoldRegion {
                    range: start_char..end_char,
                    label,
                });
            }
        }
    }

    // Remove overlapping regions (keep the outermost).
    regions.sort_by_key(|r| r.range.start);
    let mut filtered: Vec<FoldRegion> = Vec::new();
    for region in regions {
        if let Some(last) = filtered.last() {
            if region.range.start < last.range.end {
                // Overlap: keep the one that starts earlier (outermost).
                continue;
            }
        }
        filtered.push(region);
    }

    filtered
}

/// Attempt to extract the function name from a function_definition node.
fn extract_function_name(node: &Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return child
                .utf8_text(source)
                .unwrap_or("function")
                .to_string();
        }
        if child.kind() == "function" {
            // Some languages use "function" keyword, then name.
            continue;
        }
    }
    "function".to_string()
}

#[cfg(test)]
mod tests {
    // Tests would require a real language and tree.
}