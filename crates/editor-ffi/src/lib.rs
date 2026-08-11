//! FFI boundary for the editor core. Exports a C‑compatible API.
//! All functions are wrapped with catch_unwind to prevent panics crossing the ABI.

use editor_core::EditorBuffer;
use editor_history::{History, HistoryEntry, Delta};
use editor_highlight::SyntaxHighlighter;
use std::ffi::{c_char, CStr, CString};
use std::os::raw::{c_int, c_void};
use std::panic;
use std::ptr;

/// Opaque handle to an editor instance.
#[repr(C)]
pub struct EditorHandle {
    buffer: EditorBuffer,
    history: History,
    highlighter: Option<SyntaxHighlighter>,
}

/// Convenience for returning a null pointer on panic.
fn catch_unwind_null<F: panic::UnwindSafe>(f: F) -> *mut c_void
where
    F: FnOnce() -> *mut c_void,
{
    match panic::catch_unwind(f) {
        Ok(p) => p,
        Err(_) => ptr::null_mut(),
    }
}

/// Convenience for returning -1 on panic.
fn catch_unwind_int<F: panic::UnwindSafe>(f: F) -> c_int
where
    F: FnOnce() -> c_int,
{
    match panic::catch_unwind(f) {
        Ok(i) => i,
        Err(_) => -1,
    }
}

/// Convenience for returning 0 on panic.
fn catch_unwind_unit<F: panic::UnwindSafe>(f: F) -> c_int
where
    F: FnOnce(),
{
    match panic::catch_unwind(f) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Create a new editor with an optional initial text.
#[no_mangle]
pub extern "C" fn editor_create(initial_text: *const c_char) -> *mut EditorHandle {
    catch_unwind_null(|| {
        let text = if initial_text.is_null() {
            ""
        } else {
            unsafe { CStr::from_ptr(initial_text) }.to_str().unwrap_or("")
        };
        let buf = EditorBuffer::new(text);
        let history = History::new(100);
        let handle = Box::new(EditorHandle {
            buffer: buf,
            history,
            highlighter: None,
        });
        Box::into_raw(handle) as *mut c_void
    }) as *mut EditorHandle
}

/// Destroy the editor and free its memory.
#[no_mangle]
pub extern "C" fn editor_destroy(handle: *mut EditorHandle) -> c_int {
    catch_unwind_unit(|| {
        if handle.is_null() {
            return;
        }
        unsafe { drop(Box::from_raw(handle)) };
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
        let handle = unsafe { &mut *handle };
        let text_str = unsafe { CStr::from_ptr(text) }.to_str().unwrap_or("");
        let delta = Delta::Insert {
            start: pos,
            text: text_str.to_string(),
        };
        let entry = HistoryEntry::new(vec![delta], "typing".to_string());
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
        let handle = unsafe { &mut *handle };
        // Capture the deleted text before removal — this is what enables undo.
        let deleted_text = if start < end {
            handle.buffer.rope().slice(start..end).to_string()
        } else {
            String::new()
        };
        let delta = Delta::Delete {
            start,
            end,
            text: deleted_text,
        };
        let entry = HistoryEntry::new(vec![delta], "delete".to_string());
        handle.buffer.delete_range(start, end);
        handle.history.push_entry(entry);
        0
    })
}

/// Undo the last edit.
#[no_mangle]
pub extern "C" fn editor_undo(handle: *mut EditorHandle) -> c_int {
    catch_unwind_int(|| {
        let handle = unsafe { &mut *handle };
        if handle.history.undo(&mut handle.buffer) {
            1
        } else {
            0
        }
    })
}

/// Redo the last undone edit.
#[no_mangle]
pub extern "C" fn editor_redo(handle: *mut EditorHandle) -> c_int {
    catch_unwind_int(|| {
        let handle = unsafe { &mut *handle };
        if handle.history.redo(&mut handle.buffer) {
            1
        } else {
            0
        }
    })
}

/// Get the entire text as a UTF‑8 string. The caller must free the returned string.
#[no_mangle]
pub extern "C" fn editor_get_text(handle: *mut EditorHandle) -> *mut c_char {
    catch_unwind_null(|| {
        let handle = unsafe { &mut *handle };
        let text = handle.buffer.text();
        let cstring = CString::new(text).unwrap();
        cstring.into_raw() as *mut c_void
    }) as *mut c_char
}

/// Free a string allocated by the library.
#[no_mangle]
pub extern "C" fn editor_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) };
    }
}

/// Set the cursor position.
#[no_mangle]
pub extern "C" fn editor_set_cursor(handle: *mut EditorHandle, pos: usize) -> c_int {
    catch_unwind_unit(|| {
        let handle = unsafe { &mut *handle };
        handle.buffer.set_cursor(pos);
    })
}