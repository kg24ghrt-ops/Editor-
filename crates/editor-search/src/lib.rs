//! Streaming regex search over rope chunks.

use editor_core::EditorBuffer;
use regex::bytes::Regex;
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
    let mut byte_offset = 0;

    // Iterate over chunks.
    for chunk in rope.chunks() {
        let chunk_bytes = chunk.as_bytes();
        // Search within this chunk.
        for mat in re.find_iter(chunk_bytes) {
            let start_byte = byte_offset + mat.start();
            let end_byte = byte_offset + mat.end();
            let start_char = rope.byte_to_char(start_byte);
            let end_char = rope.byte_to_char(end_byte);
            let matched_text =
                String::from_utf8_lossy(&chunk_bytes[mat.start()..mat.end()]).to_string();
            results.push(SearchMatch {
                char_range: start_char..end_char,
                text: matched_text,
            });
        }
        byte_offset += chunk.len();
    }
    results
}

/// Search only in a specific range (character indices).
pub fn search_range(buf: &EditorBuffer, pattern: &str, range: Range<usize>) -> Vec<SearchMatch> {
    let re = Regex::new(pattern).expect("Invalid regex");
    let rope = buf.rope();

    // We need to search only within the byte range corresponding to the char range.
    let start_byte = rope.char_to_byte(range.start);
    let end_byte = rope.char_to_byte(range.end);

    let mut results = Vec::new();

    // Get the chunks iterator starting at the desired byte position.
    // chunks_at_byte returns (Chunks, chunk_start_byte, chunk_end_byte, line_index)
    let (mut chunks, mut chunk_start_byte, mut _chunk_end_byte, _line_idx) =
        rope.chunks_at_byte(start_byte);

    // Iterate over chunks from the starting position.
    while let Some(chunk) = chunks.next() {
        if chunk_start_byte >= end_byte {
            break;
        }

        let chunk_bytes = chunk.as_bytes();
        // Limit search to the portion of the chunk that's within the range.
        let local_start = 0;
        let local_end = (end_byte - chunk_start_byte).min(chunk.len());

        // Safety: chunk is valid UTF-8, so slicing by bytes is safe as long as
        // the indices are on UTF-8 character boundaries.
        // We're only using this slice for regex search on bytes, not for string conversion directly.
        let chunk_slice = &chunk_bytes[local_start..local_end];

        for mat in re.find_iter(chunk_slice) {
            let abs_start_byte = chunk_start_byte + mat.start();
            let abs_end_byte = chunk_start_byte + mat.end();
            if abs_end_byte > end_byte {
                break;
            }
            let start_char = rope.byte_to_char(abs_start_byte);
            let end_char = rope.byte_to_char(abs_end_byte);
            let matched_text =
                String::from_utf8_lossy(&chunk_slice[mat.start()..mat.end()]).to_string();
            results.push(SearchMatch {
                char_range: start_char..end_char,
                text: matched_text,
            });
        }

        chunk_start_byte += chunk.len();
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

    #[test]
    fn search_range() {
        let buf = EditorBuffer::new("hello world");
        let matches = search_range(&buf, r"l+", 0..5);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "ll");
        assert_eq!(matches[0].char_range, 2..4);
    }
}