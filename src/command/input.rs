use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CommandFocus {
    #[default]
    Input,
    Browser,
}

#[derive(Clone, Debug, Default)]
pub struct CommandLine {
    buffer: String,
    cursor: usize,
    pub focus: CommandFocus,
}

impl CommandLine {
    pub fn text(&self) -> &str {
        &self.buffer
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn insert(&mut self, text: &str) {
        self.buffer.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.normalize_cursor();
    }
    fn normalize_cursor(&mut self) {
        // Editing can merge adjacent graphemes (combining marks, ZWJ, flags).
        self.cursor = self
            .buffer
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .find(|&i| i >= self.cursor)
            .unwrap_or(self.buffer.len());
    }
    pub fn home(&mut self) {
        self.cursor = 0;
    }
    pub fn end(&mut self) {
        self.cursor = self.buffer.len();
    }
    pub fn left(&mut self) {
        self.cursor = self.buffer[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map_or(0, |(i, _)| i);
    }
    pub fn right(&mut self) {
        self.cursor += self.buffer[self.cursor..]
            .graphemes(true)
            .next()
            .map_or(0, str::len);
    }
    pub fn backspace(&mut self) {
        let end = self.cursor;
        self.left();
        self.buffer.replace_range(self.cursor..end, "");
        self.normalize_cursor();
    }
    pub fn delete(&mut self) {
        let end = self.cursor
            + self.buffer[self.cursor..]
                .graphemes(true)
                .next()
                .map_or(0, str::len);
        self.buffer.replace_range(self.cursor..end, "");
        self.normalize_cursor();
    }
    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            CommandFocus::Input => CommandFocus::Browser,
            CommandFocus::Browser => CommandFocus::Input,
        };
    }
}
