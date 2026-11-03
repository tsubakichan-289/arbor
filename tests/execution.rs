use arbor::command::{
    executor::{self, ExternalRequest},
    shell::Shell,
};
use std::{fs, path::PathBuf};

#[test]
fn external_status_cwd_and_spawn_failure_are_reported() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = ExternalRequest {
        shell: Shell {
            path: "/bin/sh".into(),
        },
        directory: temp.path().into(),
        command: Some("printf correct > created".into()),
    };
    assert!(executor::execute(&request, false).unwrap().success());
    assert_eq!(fs::read(temp.path().join("created")).unwrap(), b"correct");
    request.command = Some("exit 7".into());
    assert_eq!(executor::execute(&request, false).unwrap().code(), Some(7));
    request.shell.path = temp.path().join("missing-shell");
    assert!(executor::execute(&request, false).is_err());
    assert_eq!(
        Shell::resolve(Some(request.shell.path.into_os_string())).path,
        PathBuf::from("/bin/sh")
    );
}
