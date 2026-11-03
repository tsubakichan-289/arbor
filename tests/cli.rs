use std::process::Command;

#[test]
fn cli_help_errors_and_nonterminal_exit_without_entering_raw_mode() {
    let binary = env!("CARGO_BIN_EXE_arbor");
    let help = Command::new(binary).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage: arbor"));
    let temp = tempfile::tempdir().unwrap();
    let missing = Command::new(binary)
        .arg(temp.path().join("missing"))
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("cannot open"));
    assert!(!missing.stdout.contains(&0x1b));
    let no_tty = Command::new(binary).arg(temp.path()).output().unwrap();
    assert!(!no_tty.status.success());
    assert!(String::from_utf8_lossy(&no_tty.stderr).contains("interactive terminal"));
    assert!(!no_tty.stdout.contains(&0x1b));
}
