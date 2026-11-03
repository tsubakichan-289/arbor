use crate::command::executor::SignalGuard;
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, IsTerminal, stdout};

/// Guard exists before fallible setup; Drop is the panic/partial-setup fallback.
struct TerminalGuard {
    raw: bool,
    alternate: bool,
}
impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        let mut guard = Self {
            raw: false,
            alternate: false,
        };
        enable_raw_mode()?;
        guard.raw = true;
        guard.alternate = true;
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        Ok(guard)
    }
    fn restore(&mut self) -> io::Result<()> {
        let mut errors = Vec::new();
        if self.raw {
            match disable_raw_mode() {
                Ok(()) => self.raw = false,
                Err(e) => errors.push(e.to_string()),
            }
        }
        if self.alternate {
            if let Err(e) = execute!(stdout(), Show) {
                errors.push(e.to_string());
            }
            match execute!(stdout(), LeaveAlternateScreen) {
                Ok(()) => self.alternate = false,
                Err(e) => errors.push(e.to_string()),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(errors.join("; ")))
        }
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub fn run<T>(
    app: impl FnOnce(&mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<T>,
) -> io::Result<T> {
    if !io::stdin().is_terminal() || !stdout().is_terminal() {
        return Err(io::Error::other(
            "Arbor requires an interactive terminal on stdin and stdout",
        ));
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), Show, LeaveAlternateScreen);
        previous(info);
    }));
    let mut guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    let result = app(&mut terminal);
    let restored = guard.restore();
    match (result, restored) {
        (result, Ok(())) => result,
        (_, Err(error)) => Err(io::Error::other(format!(
            "terminal cleanup failed: {error}"
        ))),
    }
}

struct CookedState(libc::termios);
impl CookedState {
    fn capture() -> io::Result<Self> {
        let mut state = std::mem::MaybeUninit::uninit();
        // SAFETY: tcgetattr initializes state on success.
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, state.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(unsafe { state.assume_init() }))
    }
    fn restore(&self) -> io::Result<()> {
        // SAFETY: restoring a termios previously obtained from this terminal.
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}
impl Drop for CookedState {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// The operation streams directly to the real terminal. Every return path tries
/// to resume; a failed resume is fatal and reported after best-effort cleanup.
pub fn suspend<T>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    pause: bool,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    let _signals = SignalGuard::ignore(&[
        libc::SIGINT,
        libc::SIGQUIT,
        libc::SIGTSTP,
        libc::SIGTTOU,
        libc::SIGTTIN,
    ])?;
    disable_raw_mode()?;
    let cooked = CookedState::capture()?;
    execute!(stdout(), Show, LeaveAlternateScreen)?;
    let result = operation();
    let restored = cooked.restore();
    // Drop before enabling raw mode, so its fallback cannot overwrite raw settings.
    drop(cooked);
    // Programs can leave echo/canonical input disabled. Repair that before
    // waiting for Enter as well as before re-entering the TUI.
    let paused = if restored.is_ok() && result.is_ok() && pause {
        crate::command::executor::wait_for_enter()
    } else {
        Ok(())
    };
    let resumed = restored.and_then(|()| {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        terminal.clear()
    });
    match resumed {
        Ok(()) => {
            paused?;
            result
        }
        Err(error) => {
            let mut cleanup = TerminalGuard {
                raw: true,
                alternate: true,
            };
            let cleanup_result = cleanup.restore();
            Err(io::Error::other(format!(
                "cannot resume TUI: {error}; cleanup: {cleanup_result:?}"
            )))
        }
    }
}
