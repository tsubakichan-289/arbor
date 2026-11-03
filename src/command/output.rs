use std::collections::VecDeque;

/// A bounded tail of raw output. UTF-8 split across reads is decoded together.
#[derive(Default)]
pub struct CommandOutput {
    bytes: VecDeque<u8>,
    pub truncated: bool,
}
impl CommandOutput {
    pub const LIMIT: usize = 64 * 1024;

    pub fn clear(&mut self) {
        self.bytes.clear();
        self.truncated = false;
    }
    pub fn append(&mut self, bytes: &[u8]) {
        let overflow = (self.bytes.len() + bytes.len()).saturating_sub(Self::LIMIT);
        self.truncated |= overflow > 0;
        self.bytes.drain(..overflow.min(self.bytes.len()));
        self.bytes
            .extend(&bytes[bytes.len().saturating_sub(Self::LIMIT)..]);
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    /// Strip terminal controls; subprocess output must never control Arbor's tty.
    /// This is a plain-text log, not a terminal emulator.
    pub fn text(&self) -> String {
        let bytes: Vec<_> = self.bytes.iter().copied().collect();
        let text = String::from_utf8_lossy(&bytes);
        let mut result = String::new();
        let mut state = 0;
        for c in text.chars() {
            match state {
                1 => {
                    state = match c {
                        '[' => 2,
                        ']' | 'P' | '^' | '_' => 3,
                        ' '..='/' => 5,
                        _ => 0,
                    }
                }
                2 => {
                    if ('@'..='~').contains(&c) {
                        state = 0;
                    }
                }
                3 => {
                    if c == '\u{7}' {
                        state = 0;
                    } else if c == '\u{1b}' {
                        state = 4;
                    }
                }
                4 => state = if c == '\\' { 0 } else { 3 },
                5 => {
                    if ('0'..='~').contains(&c) {
                        state = 0;
                    }
                }
                _ => match c {
                    '\u{1b}' => state = 1,
                    '\n' => result.push('\n'),
                    '\r' => {}
                    '\t' => result.push_str("    "),
                    c if !c.is_control() => result.push(c),
                    _ => {}
                },
            }
        }
        result
    }
}
