use synoptic::{Highlighter, SyntaxSet, Theme};

pub struct Highlighter {
    inner: synoptic::Highlighter,
    ss: SyntaxSet,
    theme: Theme,
    lang: String,
}

impl Highlighter {
    pub fn new() -> Self {
        let mut ss = SyntaxSet::new();
        // Load built-in language definitions (available in 2.x)
        ss.load_defaults();

        Self {
            inner: Highlighter::new(4), // 4 = tab width
            ss,
            theme: Theme::default(),
            lang: "plain".to_string(),
        }
    }

    pub fn set_language(&mut self, lang: &str) {
        self.lang = lang.to_string();
    }

    pub fn highlight_line(&self, line: &str) -> Vec<(u32, &str)> {
        // Find syntax definition by name or extension
        let syntax = self
            .ss
            .find_syntax_by_name(&self.lang)
            .or_else(|| self.ss.find_syntax_by_extension(&self.lang))
            .unwrap_or_else(|| self.ss.find_syntax_plain_text());

        // Run the highlighter on this line
        let tokens = self.inner.line(line, &self.ss, syntax);

        tokens
            .iter()
            .map(|token| {
                let color = token
                    .style
                    .foreground
                    .map(|c| (c.r as u32) << 16 | (c.g as u32) << 8 | (c.b as u32))
                    .unwrap_or(0xFFFFFF);
                (color, token.text.as_str())
            })
            .collect()
    }
}