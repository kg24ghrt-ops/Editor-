mod editor;
mod renderer;

use jni::objects::{JClass, JObject};
use jni::sys::{jlong, jint};
use jni::Env;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

type EditorHandle = u64;

static EDITORS: OnceLock<Mutex<HashMap<EditorHandle, EditorState>>> = OnceLock::new();
static NEXT_HANDLE: OnceLock<Mutex<EditorHandle>> = OnceLock::new();

fn get_editors() -> &'static Mutex<HashMap<EditorHandle, EditorState>> {
    EDITORS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_next_handle() -> &'static Mutex<EditorHandle> {
    NEXT_HANDLE.get_or_init(|| Mutex::new(1))
}

pub struct EditorState {
    pub buffer: editor::Buffer,
    pub history: editor::EditorHistory,
    pub highlighter: editor::SyntaxHighlighter,
    pub renderer: renderer::Renderer,
    pub cursor_pos: usize,
    pub scroll_line: usize,
    pub viewport_lines: usize,
    pub line_height: f32,
    pub font_size: f32,
}

#[no_mangle]
pub extern "system" fn Java_com_yourapp_editor_EditorBridge_createEditor(
    env: Env,
    _class: JClass,
    surface: JObject,
    width: jint,
    height: jint,
) -> jlong {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let result: Result<jlong, renderer::RendererError> = (|| {
        let mut renderer = pollster::block_on(renderer::Renderer::new(surface, &env, width as u32, height as u32))?;
        renderer.resize(width as u32, height as u32);

        let state = EditorState {
            buffer: editor::Buffer::from_str("Welcome to Mega Editor!\nType something..."),
            history: editor::EditorHistory::new(),
            highlighter: editor::SyntaxHighlighter::new(),
            renderer,
            cursor_pos: 0,
            scroll_line: 0,
            viewport_lines: (height as f32 / 20.0) as usize,
            line_height: 20.0,
            font_size: 16.0,
        };

        let mut map = get_editors().lock().unwrap();
        let mut next = get_next_handle().lock().unwrap();
        let handle = *next;
        *next += 1;
        map.insert(handle, state);
        Ok(handle as jlong)
    })();

    result.unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_yourapp_editor_EditorBridge_renderFrame(
    _env: Env,
    _class: JClass,
    handle: jlong,
) {
    if let Ok(mut map) = get_editors().lock() {
        if let Some(state) = map.get_mut(&(handle as EditorHandle)) {
            let _ = state.renderer.render();
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_yourapp_editor_EditorBridge_onKeyEvent(
    _env: Env,
    _class: JClass,
    handle: jlong,
    key_code: jint,
    is_pressed: jint,
    is_ctrl: jint,
) {
    if is_pressed == 0 {
        return;
    }
    let ctrl = is_ctrl != 0;

    if let Ok(mut map) = get_editors().lock() {
        if let Some(state) = map.get_mut(&(handle as EditorHandle)) {
            use editor::EditCommand;

            match key_code {
                67 => {
                    if state.cursor_pos > 0 {
                        let start = state.cursor_pos - 1;
                        if let Some(ch) = state.buffer.char_at(start) {
                            let deleted = ch.to_string();
                            let cmd = EditCommand::Delete {
                                start,
                                end: state.cursor_pos,
                                deleted,
                            };
                            state.history.edit(&mut state.buffer, cmd);
                            state.cursor_pos -= 1;
                        }
                    }
                }
                66 => {
                    let cmd = EditCommand::Insert {
                        pos: state.cursor_pos,
                        text: "\n".to_string(),
                    };
                    state.history.edit(&mut state.buffer, cmd);
                    state.cursor_pos += 1;
                }
                21 => {
                    if state.cursor_pos > 0 {
                        state.cursor_pos -= 1;
                    }
                }
                22 => {
                    if state.cursor_pos < state.buffer.len() {
                        state.cursor_pos += 1;
                    }
                }
                19 => {
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
                20 => {
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
                26 if ctrl => {
                    state.history.undo(&mut state.buffer);
                }
                25 if ctrl => {
                    state.history.redo(&mut state.buffer);
                }
                _ => {
                    if let Some(ch) = char::from_u32(key_code as u32) {
                        if ch.is_ascii_graphic() || ch == ' ' {
                            let text = ch.to_string();
                            let cmd = EditCommand::Insert {
                                pos: state.cursor_pos,
                                text,
                            };
                            state.history.edit(&mut state.buffer, cmd);
                            state.cursor_pos += 1;
                        }
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_yourapp_editor_EditorBridge_destroyEditor(
    _env: Env,
    _class: JClass,
    handle: jlong,
) {
    if let Ok(mut map) = get_editors().lock() {
        map.remove(&(handle as EditorHandle));
    }
}
