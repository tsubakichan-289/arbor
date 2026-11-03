use std::collections::VecDeque;

/// Session-only, bounded history; persistence and recall keys can be added later.
#[derive(Default)]
pub struct History(VecDeque<String>);
impl History {
    pub fn record(&mut self, command: &str) {
        if command.trim().is_empty() || self.0.back().is_some_and(|last| last == command) {
            return;
        }
        if self.0.len() == 100 {
            self.0.pop_front();
        }
        self.0.push_back(command.to_owned());
    }
}
