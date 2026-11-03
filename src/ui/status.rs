use super::{canvas::Canvas, style};
use crate::{app::BrowserState, fs::entry::safe_text, layout::LayoutResult};

pub struct Status {
    pub header: String,
    pub footer: String,
    pub error: bool,
}

impl Status {
    pub fn from_state(state: &BrowserState) -> Self {
        let active = state.active();
        let header = format!(" ARBOR  {}", safe_text(&active.directory.to_string_lossy()));
        let issue = active
            .error
            .as_ref()
            .map(|error| format!("error: {error}"))
            .or_else(|| {
                state
                    .preview
                    .as_ref()
                    .and_then(|preview| preview.error.as_ref())
                    .map(|error| format!("preview error: {error}"))
            });
        let error = issue.is_some();
        let footer = match issue {
            Some(issue) => format!(" {issue}  |  ← parent  q/Esc quit"),
            None => format!(
                " ↑↓ select  ← parent  → enter  : command  ! shell  q quit  |  {}/{}  depth {}",
                active.selected.map_or(0, |i| i + 1),
                active.entries().len(),
                state.columns.len()
            ),
        };
        Self {
            header,
            footer,
            error,
        }
    }

    pub fn render(&self, scene: &LayoutResult, canvas: &mut Canvas<'_>) {
        if let Some(position) = scene.header_position {
            canvas.text(position, &self.header, style::header());
        }
        if let Some(position) = scene.footer_position {
            canvas.text(position, &self.footer, style::status(self.error));
        }
    }
}
