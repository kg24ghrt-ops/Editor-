//! Fold regions derived from the syntax tree.

use editor_core::EditorBuffer;
use tree_sitter::{Node, Tree};
use std::ops::Range;

/// A foldable region, typically a block or function body.
#[derive(Debug, Clone)]
pub struct FoldRegion {
    /// Character range of the folded region (should be a contiguous block).
    pub range: Range<usize>,
    /// Optional display name (like function name).
    pub label: String,
}

/// Extracts foldable regions from a syntax tree.
pub fn fold_regions_from_tree(tree: &Tree, buf: &EditorBuffer) -> Vec<FoldRegion> {
    let root = tree.root_node();
    let mut regions = Vec::new();
    collect_fold_regions(&root, buf, &mut regions);
    regions
}

fn collect_fold_regions(node: &Node, buf: &EditorBuffer, out: &mut Vec<FoldRegion>) {
    // Heuristic: fold any block-like nodes (brace-delimited) or functions.
    // For a real implementation, use queries.
    let kind = node.kind();
    if kind == "block" || kind == "function_definition" || kind == "if_statement" {
        // Compute char range from byte range.
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let rope = buf.rope();
        let start_char = rope.byte_to_char(start_byte);
        let end_char = rope.byte_to_char(end_byte);
        if end_char > start_char + 1 {
            let label = if kind == "function_definition" {
                // Try to find the function name.
                let mut name = "function".to_string();
                // In a real impl, walk children.
                name
            } else {
                kind.to_string()
            };
            out.push(FoldRegion {
                range: start_char..end_char,
                label,
            });
        }
    }

    // Recurse.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_fold_regions(&child, buf, out);
    }
}

#[cfg(test)]
mod tests {
    // Tests would require a real language and tree.
}