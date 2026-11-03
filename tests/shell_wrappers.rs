use std::{
    ffi::OsString,
    fs,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::PermissionsExt,
    },
    process::Command,
};

#[test]
fn bash_and_zsh_wrappers_preserve_bytes_args_status_and_cleanup() {
    for shell in ["bash", "zsh"] {
        if Command::new(shell).arg("--version").output().is_err() {
            eprintln!("SKIP: {shell} is unavailable; install it to validate this wrapper");
            continue;
        }
        for code in [0, 7] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            fs::create_dir(root.join("bin")).unwrap();
            fs::create_dir(root.join("temps")).unwrap();
            let destination = root.join(OsString::from_vec(b"space \n'$(touch PWN)\xff".to_vec()));
            fs::create_dir(&destination).unwrap();
            let fake = root.join("bin/arbor");
            fs::write(&fake, b"#!/bin/sh\n[ \"$1\" = --cwd-file ] || exit 99\nout=$2\nshift 2\nprintf '%s\\0' \"$@\" > \"$TEST_ARGS\"\nprintf '%s\\0' \"$TEST_DEST\" > \"$out\"\nexit \"$TEST_EXIT\"\n").unwrap();
            fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
            let mut paths = vec![root.join("bin")];
            paths.extend(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ));
            let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("scripts/arbor.{shell}"));
            let output = Command::new(shell).args(["-f", "-c", "source \"$1\"; arbor --example 'two words'; arbor_result=$?; printf '%s\\0%d\\0' \"$PWD\" \"$arbor_result\"; exit \"$arbor_result\"", "wrapper-test"]).arg(script)
                .current_dir(root).env("PATH", std::env::join_paths(paths).unwrap()).env("TMPDIR", root.join("temps"))
                .env("TEST_DEST", &destination).env("TEST_ARGS", root.join("args")).env("TEST_EXIT", code.to_string()).output().unwrap();
            assert_eq!(
                output.status.code(),
                Some(code),
                "{shell}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let fields: Vec<_> = output.stdout.split(|&b| b == 0).collect();
            assert_eq!(
                fields[0],
                if code == 0 {
                    destination.as_os_str().as_bytes()
                } else {
                    root.as_os_str().as_bytes()
                },
                "{shell}"
            );
            assert_eq!(fields[1], code.to_string().as_bytes());
            assert_eq!(
                fs::read(root.join("args")).unwrap(),
                b"--example\0two words\0"
            );
            assert_eq!(fs::read_dir(root.join("temps")).unwrap().count(), 0);
            assert!(!root.join("PWN").exists());
        }
    }
}
