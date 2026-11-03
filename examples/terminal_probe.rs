//! Used by scripts/pty_smoke.py to verify cleanup on returned errors and panics.
fn main() -> std::io::Result<()> {
    arbor::ui::terminal::run(|_| {
        if std::env::args().nth(1).as_deref() == Some("panic") {
            panic!("intentional terminal restoration probe");
        }
        Err(std::io::Error::other("intentional terminal error probe"))
    })
}
