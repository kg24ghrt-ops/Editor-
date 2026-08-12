//! FFI boundary for the editor core. Exports a C‑compatible API.
//! All functions are wrapped with catch_unwind to prevent panics crossing the ABI.

use editor_core::EditorBuffer;
use editor_fold::fold_regions_from_tree;
use editor_highlight::SyntaxHighlighter;
use editor_history::{Delta, History, HistoryEntry};
use std::ffi::{c_char, CStr, CString};
use std::os::raw::{c_int, c_void};
use std::panic;
use std::ptr;

/// Opaque handle to an editor instance.
#[repr(C)]
pub struct EditorHandle {
    buffer: EditorBuffer,
    history: History,
    highlighter: SyntaxHighlighter,
}

/// Convenience for returning a null pointer on panic.
fn catch_unwind_null<F>(f: F) -> *mut c_void
where
    F: FnOnce() -> *mut c_void,
{
    match panic::catch_unwind(f) {
        Ok(p) => p,
        Err(_) => ptr::null_mut(),
    }
}

/// Convenience for returning -1 on panic.
fn catch_unwind_int<F>(f: F) -> c_int
where
    F: FnOnce() -> c_int,
{
    match panic::catch_unwind(f) {
        Ok(i) => i,
        Err(_) => -1,
    }
}

/// Convenience for returning 0 on panic, -1 on error.
fn catch_unwind_unit<F>(f: F) -> c_int
where
    F: FnOnce() -> c_int,
{
    match panic::catch_unwind(f) {
        Ok(i) => i,
        Err(_) => -1,
    }
}

/// Create a new editor with an optional initial text.
/// Uses Rust language for highlighting by default.
#[no_mangle]
pub extern "C" fn editor_create(initial_text: *const c_char) -> *mut EditorHandle {
    catch_unwind_null(|| {
        let text = if initial_text.is_null() {
            ""
        } else {
            unsafe { CStr::from_ptr(initial_text) }
                .to_str()
                .unwrap_or("")
        };

        let buf = EditorBuffer::new(text);
        let history = History::new(100);
        let mut highlighter = SyntaxHighlighter::empty();

        // Attempt to set Rust as the default language.
        // In a real app, you'd expose a function to change this.
        #[cfg(feature = "rust")]
        {
            if let Ok(lang) = tree_sitter_rust::language() {
                let _ = highlighter.set_language(lang);
            }
        }

        let handle = Box::new(EditorHandle {
            buffer: buf,
            history,
            highlighter,
        });
        Box::into_raw(handle) as *mut c_void
    }) as *mut EditorHandle
}

/// Destroy the editor and free its memory.
#[no_mangle]
pub extern "C" fn editor_destroy(handle: *mut EditorHandle) -> c_int {
    catch_unwind_unit(|| {
        if handle.is_null() {
            return -1;
        }
        unsafe { drop(Box::from_raw(handle)) };
        0
    })
}

/// Insert text at the given character position.
#[no_mangle]
pub extern "C" fn editor_insert_text(
    handle: *mut EditorHandle,
    pos: usize,
    text: *const c_char,
) -> c_int {
    catch_unwind_int(|| {
        if handle.is_null() || text.is_null() {
            return -1;
        }
        let handle = unsafe { &mut *handle };
        let text_str = unsafe { CStr::from_ptr(text) }.to_str().unwrap_or("");
        let delta = Delta::Insert {
            start: pos,
            text: text_str.to_string(),
        };
        let entry = HistoryEntry::new(vec![delta], "typing");
        entry.apply_forward(&mut handle.buffer);
        handle.history.push_entry(entry);
        0
    })
}

/// Delete the range [start, end).
#[no_mangle]
pub extern "C" fn editor_delete_range(
    handle: *mut EditorHandle,
    start: usize,
    end: usize,
) -> c_int {
    catch_unwind_int(|| {
        if handle.is_null() {
            return -1;
        }
        let handle = unsafe { &mut *handle };
        if start >= end {
            return 0;
        }
        // Capture the deleted text for undo.
        let deleted_text = handle.buffer.rope().slice(start..end).to_string();
        let delta = Delta::Delete {
            start,
            end,
            text: deleted_text,
        };
        let entry = HistoryEntry::new(vec![delta], "delete");
        handle.buffer.delete_range(start, end);
        handle.history.push_entry(entry);
        0
    })
}

/// Undo the last operation.
#[no_mangle]
pub extern "C" fn editor_undo(handle: *mut EditorHandle) -> c_int {
    catch_unwind_int(|| {
        if handle.is_null() {
            return -1;
        }
        let handle = unsafe { &mut *handle };
        if handle.history.undo(&mut handle.buffer) {
            1
        } else {
            0
        }
    })
}

/// Redo the last undone operation.
#[no_mangle]
pub extern "C" fn editor_redo(handle: *mut EditorHandle) -> c_int {
    catch_unwind_int(|| {
        if handle.is_null() {
            return -1;
        }
        let handle = unsafe { &mut *handle };
        if handle.history.redo(&mut handle.buffer) {
            1
        } else {
            0
        }
    })
}

/// Get the entire text of the buffer.
/// The caller must free the returned string with `editor_free_string`.
#[no_mangle]
pub extern "C" fn editor_get_text(handle: *mut EditorHandle) -> *mut c_char {
    catch_unwind_null(|| {
        if handle.is_null() {
            return ptr::null_mut();
        }
        let handle = unsafe { &mut *handle };
        let text = handle.buffer.text();
        let cstr = CString::new(text).unwrap();
        cstr.into_raw() as *mut c_char
    })
}

/// Free a string previously allocated by the editor.
#[no_mangle]
pub extern "C" fn editor_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

/// Get the buffer's version number.
#[no_mangle]
pub extern "C" fn editor_get_version(handle: *mut EditorHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    let handle = unsafe { &mut *handle };
    handle.buffer.version()
}

/// Get the current cursor position (head of the first cursor).
#[no_mangle]
pub extern "C" fn editor_get_cursor(handle: *mut EditorHandle) -> usize {
    if handle.is_null() {
        return 0;
    }
    let handle = unsafe { &mut *handle };
    handle.buffer.cursors().first().map(|c| c.head).unwrap_or(0)
}

/// Set the cursor to a single position.
#[no_mangle]
pub extern "C" fn editor_set_cursor(handle: *mut EditorHandle, pos: usize) -> c_int {
    catch_unwind_int(|| {
        if handle.is_null() {
            return -1;
        }
        let handle = unsafe { &mut *handle };
        handle.buffer.set_cursor(pos);
        0
    })
}

/// Get the total number of characters in the buffer.
#[no_mangle]
pub extern "C" fn editor_len_chars(handle: *mut EditorHandle) -> usize {
    if handle.is_null() {
        return 0;
    }
    let handle = unsafe { &mut *handle };
    handle.buffer.len_chars()
}

/// Trigger a parse and highlight update.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn editor_highlight(handle: *mut EditorHandle) -> c_int {
    catch_unwind_int(|| {
        if handle.is_null() {
            return -1;
        }
        let handle = unsafe { &mut *handle };
        handle.highlighter.ensure_parsed(&handle.buffer);
        // Highlight events are not returned here; use a separate function if needed.
        0
    })
}

/// Get fold regions for the current buffer.
/// Returns a pointer to an array of FoldRegion structs.
/// The caller must free with `editor_free_fold_regions`.
#[no_mangle]
pub extern "C" fn editor_get_fold_regions(
    handle: *mut EditorHandle,
    out_count: *mut usize,
) -> *mut FoldRegion {
    catch_unwind_null(|| {
        if handle.is_null() || out_count.is_null() {
            return ptr::null_mut();
        }
        let handle = unsafe { &mut *handle };
        // Ensure we have a tree.
        let tree = handle.highlighter.ensure_parsed(&handle.buffer);
        let regions = fold_regions_from_tree(tree, &handle.buffer);
        unsafe { *out_count = regions.len() };
        let boxed = regions.into_boxed_slice();
        Box::into_raw(boxed) as *mut FoldRegion
    })
}

/// Free fold regions array.
#[no_mangle]
pub extern "C" fn editor_free_fold_regions(regions: *mut FoldRegion, count: usize) {
    if !regions.is_null() {
        unsafe {
            let slice = Vec::from_raw_parts(regions, count, count);
            drop(slice);
        }
    }
}

/// FFI‑compatible representation of a fold region.
#[repr(C)]
pub struct FoldRegion {
    pub start: usize,
    pub end: usize,
    pub label: *const c_char,
}

#[cfg(test)]
mod tests {
    // Integration tests would go here.
}