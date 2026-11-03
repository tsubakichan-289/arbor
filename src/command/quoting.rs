use super::{input::CommandLine, shell::Shell};
use std::ffi::OsStr;

pub fn quote_argument(value: &OsStr, filename: bool, shell: &Shell) -> Result<String, String> {
    if !shell.supports_posix_quoting() {
        return Err(
            "Argument insertion requires a POSIX-compatible shell (sh/bash/zsh/dash/ksh/ash)"
                .into(),
        );
    }
    let value = value
        .to_str()
        .ok_or("Cannot insert a non-UTF-8 path without changing its bytes")?;
    if value.contains('\0') {
        return Err("A shell argument cannot contain NUL".into());
    }
    let value = if filename && value.starts_with('-') {
        format!("./{value}")
    } else {
        value.to_owned()
    };
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

/// Only insert in ordinary shell text, never inside a quote, substitution,
/// comment, here-document, or escape where a quoted word would change meaning.
pub fn insert_argument(line: &mut CommandLine, quoted: &str) -> Result<(), String> {
    let prefix = &line.text()[..line.cursor()];
    if prefix.contains(['\n', '\r']) || prefix.contains("<<") {
        return Err("Insert arguments outside multiline commands and here-documents".into());
    }
    let mut quote = None;
    let mut escaped = false;
    let mut boundary = true;
    for c in prefix.chars() {
        if escaped {
            escaped = false;
            boundary = false;
            continue;
        }
        if quote == Some('\'') {
            if c == '\'' {
                quote = None;
            }
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if matches!(c, '$' | '`') {
            return Err("Insert arguments outside shell expansions".into());
        }
        if quote == Some('"') {
            if c == '"' {
                quote = None;
            }
            continue;
        }
        if matches!(c, '(' | ')' | '{' | '}') || (c == '#' && boundary) {
            return Err("Insert arguments outside shell groups and comments".into());
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
        }
        boundary = c.is_whitespace() || matches!(c, ';' | '|' | '&');
    }
    if quote.is_some() || escaped {
        return Err(
            "Move the cursor outside quotes and escapes before inserting an argument".into(),
        );
    }
    let separator = if prefix.is_empty()
        || prefix.ends_with(char::is_whitespace)
        || prefix.ends_with([';', '|', '&', '<', '>'])
    {
        ""
    } else {
        " "
    };
    line.insert(&format!("{separator}{quoted} "));
    Ok(())
}
