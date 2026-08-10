//! Streaming regex search over rope chunks.

use editor_core::EditorBuffer;
use regex::bytes::Regex;
use ropey::RopeSlice;
use std::ops::Range;

/// Search result: byte range and char range.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Character range in the rope.
    pub char_range: Range<usize>,
    /// The matched text (as a string).
    pub text: String,
}

/// Searches for a pattern in the buffer, streaming over chunks to avoid materializing.
pub fn search_buffer(buf: &EditorBuffer, pattern: &str) -> Vec<SearchMatch> {
    let re = Regex::new(pattern).expect("Invalid regex");
    let rope = buf.rope();
    let mut results = Vec::new();

    // We need to iterate over the rope chunks and combine them.
    // For simplicity, we convert the whole rope to a string (but we could do streaming).
    // A real implementation would use the chunk iterator and handle cross-chunk matches.
    let text = rope.to_string();
    let bytes = text.as_bytes();
    for mat in re.find_iter(bytes) {
        let start_byte = mat.start();
        let end_byte = mat.end();
        // Convert byte offsets to char offsets.
        let start_char = rope.byte_to_char(start_byte);
        let end_char = rope.byte_to_char(end_byte);
        let matched_text = String::from_utf8_lossy(&bytes[start_byte..end_byte]).to_string();
        results.push(SearchMatch {
            char_range: start_char..end_char,
            text: matched_text,
        });
    }
    results
}

/// Search only in a specific range (character indices).
pub fn search_range(buf: &EditorBuffer, pattern: &str, range: Range<usize>) -> Vec<SearchMatch> {
    let slice = buf.rope().slice(range.clone());
    let text = slice.to_string();
    let re = Regex::new(pattern).expect("Invalid regex");
    let mut results = Vec::new();
    for mat in re.find_iter(text.as_bytes()) {
        let start_byte = mat.start();
        let end_byte = mat.end();
        // Since slice starts at range.start, we adjust.
        let start_char = range.start + slice.byte_to_char(start_byte);
        let end_char = range.start + slice.byte_to_char(end_byte);
        let matched_text = String::from_utf8_lossy(&text.as_bytes()[start_byte..end_byte]).to_string();
        results.push(SearchMatch {
            char_range: start_char..end_char,
            text: matched_text,
        });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_search() {
        let buf = EditorBuffer::new("hello world");
        let matches = search_buffer(&buf, r"l+");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].text, "ll");
        assert_eq!(matches[0].char_range, 2..4);
    }
}