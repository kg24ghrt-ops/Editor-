// src/lib.rs
mod editor;
mod renderer;

use jni::objects::{JClass, JObject};
use jni::sys::{jlong, jint};
use jni::JNIEnv;
use std::sync::Mutex;
use std::collections::HashMap;

// Global registry of editor instances (handle -> EditorState)
type EditorHandle = u64;
static EDITORS: Mutex<HashMap<EditorHandle, EditorState>> = Mutex::new(HashMap::new());
static NEXT_HANDLE: Mutex<EditorHandle> = Mutex::new(1);

pub struct EditorState {
    buffer: editor::Buffer,
    history: editor::EditorHistory,
    highlighter: editor::Highlighter,
    renderer: renderer::Renderer,
    cursor_pos: usize,
    scroll_line: usize,   // first visible line
    viewport_lines: usize,
    line_height: f32,
    font_size: f32,
}

/// JNI entry point: create an editor instance.
/// Called from Kotlin when SurfaceView is created.
#[no_mangle]
pub extern "system" fn Java_com_yourapp_EditorBridge_createEditor(
    env: JNIEnv,
    _class: JClass,
    surface: JObject,
    width: jint,
    height: jint,
) -> jlong {
    // Initialize Android logger
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info)
    );
    
    let result: Result<jlong, Box<dyn std::error::Error>> = try {
        // Build the renderer (async, but we block since we're in JNI)
        let renderer = pollster::block_on(renderer::Renderer::new(surface, &env))?;
        renderer.resize(width as u32, height as u32);
        
        let state = EditorState {
            buffer: editor::Buffer::from_str("Welcome to Mega Editor!\nType something..."),
            history: editor::EditorHistory::new(),
            highlighter: editor::Highlighter::new(),
            renderer,
            cursor_pos: 0,
            scroll_line: 0,
            viewport_lines: (height as f32 / 20.0) as usize,
            line_height: 20.0,
            font_size: 16.0,
        };
        
        let mut map = EDITORS.lock().unwrap();
        let handle = *NEXT_HANDLE.lock().unwrap();
        *NEXT_HANDLE.lock().unwrap() += 1;
        map.insert(handle, state);
        handle as jlong
    };
    
    result.unwrap_or(0)
}

/// JNI: render a frame
#[no_mangle]
pub extern "system" fn Java_com_yourapp_EditorBridge_renderFrame(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if let Ok(mut map) = EDITORS.lock() {
        if let Some(state) = map.get_mut(&(handle as EditorHandle)) {
            let _ = state.renderer.render();
        }
    }
}

/// JNI: handle a key event (called from Kotlin's onKeyDown/onKeyUp)
#[no_mangle]
pub extern "system" fn Java_com_yourapp_EditorBridge_onKeyEvent(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    key_code: jint,
    is_pressed: bool,
    is_ctrl: bool,
) {
    if !is_pressed { return; }
    
    if let Ok(mut map) = EDITORS.lock() {
        if let Some(state) = map.get_mut(&(handle as EditorHandle)) {
            use editor::EditCommand;
            
            match key_code {
                67 => { // Backspace
                    if state.cursor_pos > 0 {
                        let start = state.cursor_pos - 1;
                        if let Some(ch) = state.buffer.char_at(start) {
                            let deleted = ch.to_string();
                            let cmd = EditCommand::Delete { start, end: state.cursor_pos, deleted };
                            state.history.execute(&mut state.buffer, cmd);
                            state.cursor_pos -= 1;
                        }
                    }
                }
                66 => { // Enter
                    let cmd = EditCommand::Insert { pos: state.cursor_pos, text: "\n".to_string() };
                    state.history.execute(&mut state.buffer, cmd);
                    state.cursor_pos += 1;
                }
                21 => { // Left arrow
                    if state.cursor_pos > 0 { state.cursor_pos -= 1; }
                }
                22 => { // Right arrow
                    if state.cursor_pos < state.buffer.len() { state.cursor_pos += 1; }
                }
                19 => { // Up arrow
                    let line = state.buffer.char_to_line(state.cursor_pos);
                    if line > 0 {
                        let start = state.buffer.line_to_char(line - 1);
                        let end = state.buffer.line_to_char(line);
                        let col = state.cursor_pos - start;
                        let new_start = state.buffer.line_to_char(line - 1);
                        let new_end = state.buffer.line_to_char(line);
                        state.cursor_pos = new_start + (col.min(new_end - new_start));
                    }
                }
                20 => { // Down arrow
                    let line = state.buffer.char_to_line(state.cursor_pos);
                    if line + 1 < state.buffer.line_count() {
                        let start = state.buffer.line_to_char(line);
                        let end = state.buffer.line_to_char(line + 1);
                        let col = state.cursor_pos - start;
                        let new_start = state.buffer.line_to_char(line + 1);
                        let new_end = if line + 2 < state.buffer.line_count() {
                            state.buffer.line_to_char(line + 2)
                        } else {
                            state.buffer.len()
                        };
                        state.cursor_pos = new_start + (col.min(new_end - new_start));
                    }
                }
                26 => { // 'Z' with Ctrl = Undo
                    if is_ctrl { state.history.undo(&mut state.buffer); }
                }
                25 => { // 'Y' with Ctrl = Redo
                    if is_ctrl { state.history.redo(&mut state.buffer); }
                }
                _ => {
                    // Insert printable character
                    if let Some(ch) = char::from_u32(key_code as u32) {
                        if ch.is_ascii_graphic() || ch == ' ' {
                            let text = ch.to_string();
                            let cmd = EditCommand::Insert { pos: state.cursor_pos, text };
                            state.history.execute(&mut state.buffer, cmd);
                            state.cursor_pos += 1;
                        }
                    }
                }
            }
        }
    }
}

/// JNI: destroy editor and free GPU resources
#[no_mangle]
pub extern "system" fn Java_com_yourapp_EditorBridge_destroyEditor(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if let Ok(mut map) = EDITORS.lock() {
        map.remove(&(handle as EditorHandle));
    }
}