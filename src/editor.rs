// src/editor.rs
use ropey::Rope;
use add_e::{Command, History};
use synoptic::{SyntaxSet, Theme};

pub struct Buffer {
    rope: Rope,
}

impl Buffer {
    pub fn new() -> Self { Self { rope: Rope::new() } }
    pub fn from_str(s: &str) -> Self { Self { rope: Rope::from(s) } }
    pub fn len(&self) -> usize { self.rope.len_chars() }
    pub fn char_at(&self, pos: usize) -> Option<char> { self.rope.get_char(pos) }
    pub fn text(&self) -> String { self.rope.to_string() }
    
    pub fn insert(&mut self, pos: usize, text: &str) -> (usize, usize) {
        let start = pos;
        self.rope.insert(pos, text);
        let end = start + text.chars().count();
        (start, end)
    }
    
    pub fn delete(&mut self, start: usize, end: usize) -> String {
        let deleted: String = self.rope.slice(start..end).chars().collect();
        self.rope.remove(start..end);
        deleted
    }
    
    pub fn line_count(&self) -> usize { self.rope.len_lines() }
    pub fn line_text(&self, idx: usize) -> String {
        let start = self.rope.line_to_char(idx);
        let end = if idx + 1 < self.rope.len_lines() {
            self.rope.line_to_char(idx + 1)
        } else {
            self.rope.len_chars()
        };
        self.rope.slice(start..end).to_string()
    }
    pub fn char_to_line(&self, pos: usize) -> usize { self.rope.char_to_line(pos) }
    pub fn line_to_char(&self, line: usize) -> usize { self.rope.line_to_char(line) }
}

// ---- Commands for undo/redo ----
pub enum EditCommand {
    Insert { pos: usize, text: String },
    Delete { start: usize, end: usize, deleted: String },
}

impl Command<Buffer> for EditCommand {
    fn execute(&self, buf: &mut Buffer) {
        match self {
            EditCommand::Insert { pos, text } => { buf.insert(*pos, text); }
            EditCommand::Delete { start, end, .. } => { buf.delete(*start, *end); }
        }
    }
    fn undo(&self, buf: &mut Buffer) {
        match self {
            EditCommand::Insert { pos, text } => {
                let end = *pos + text.chars().count();
                buf.delete(*pos, end);
            }
            EditCommand::Delete { start, deleted } => {
                buf.insert(*start, deleted);
            }
        }
    }
}

pub struct EditorHistory {
    history: History<Buffer, EditCommand>,
}

impl EditorHistory {
    pub fn new() -> Self { Self { history: History::new() } }
    pub fn execute(&mut self, buf: &mut Buffer, cmd: EditCommand) {
        self.history.execute(buf, cmd);
    }
    pub fn undo(&mut self, buf: &mut Buffer) { self.history.undo(buf); }
    pub fn redo(&mut self, buf: &mut Buffer) { self.history.redo(buf); }
}

// ---- Syntax Highlighter ----
pub struct Highlighter {
    ss: SyntaxSet,
    theme: Theme,
    lang: String,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            ss: SyntaxSet::load_defaults(),
            theme: Theme::default(),
            lang: "plain".to_string(),
        }
    }
    pub fn set_language(&mut self, lang: &str) { self.lang = lang.to_string(); }
    
    pub fn highlight_line(&self, line: &str) -> Vec<(u32, &str)> {
        let syntax = self.ss.find_syntax_by_name(&self.lang)
            .unwrap_or_else(|| self.ss.find_syntax_plain_text());
        let highlights = synoptic::highlight_line(line, &self.ss, syntax, &self.theme);
        highlights.iter()
            .map(|(style, text)| {
                let color = style.foreground
                    .map(|c| (c.r as u32) << 16 | (c.g as u32) << 8 | (c.b as u32))
                    .unwrap_or(0xFFFFFF);
                (color, *text)
            })
            .collect()
    }
}