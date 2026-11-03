use std::{io, os::unix::process::ExitStatusExt, process::ExitStatus};

pub fn result_message(result: &io::Result<ExitStatus>) -> String {
    match result {
        Ok(status) => match status.code() {
            Some(code) => format!("Command exited with status {code}"),
            None => format!(
                "Command terminated by signal {}",
                status.signal().unwrap_or(0)
            ),
        },
        Err(error) => format!("Cannot execute command: {error}"),
    }
}
