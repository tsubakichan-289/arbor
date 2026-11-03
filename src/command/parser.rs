#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub expand_home: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    Empty,
    Cd(Option<Word>),
    Pwd,
    External(String),
}

/// A deliberately small lexer, not a shell interpreter. Complex shell syntax
/// goes to the user's shell; unsupported expansion in a simple builtin is an error.
pub fn parse(input: &str) -> Result<Parsed, String> {
    let mut words = Vec::new();
    let mut text = String::new();
    let mut started = false;
    let mut expand_home = false;
    let mut quote = None;
    let mut complex = false;
    let mut unsupported = false;
    let mut error = None;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if quote == Some('\'') {
            if c == '\'' {
                quote = None;
            } else {
                text.push(c);
            }
            continue;
        }
        if c == '\\' {
            started = true;
            match chars.next() {
                Some('\n') => {}
                Some(next) => {
                    if quote == Some('"') && !matches!(next, '$' | '`' | '"' | '\\') {
                        text.push('\\');
                    }
                    text.push(next);
                }
                None => error = Some("trailing backslash"),
            }
            continue;
        }
        if quote == Some('"') {
            if c == '"' {
                quote = None;
            } else {
                unsupported |= matches!(c, '$' | '`');
                text.push(c);
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            started = true;
            continue;
        }
        if matches!(c, ';' | '&' | '|' | '<' | '>' | '(' | ')' | '\n') || (c == '#' && !started) {
            complex = true;
        }
        if c.is_whitespace() {
            if started {
                words.push(Word {
                    text: std::mem::take(&mut text),
                    expand_home,
                });
                started = false;
                expand_home = false;
            }
        } else {
            unsupported |= matches!(c, '$' | '`' | '*' | '?' | '[' | '{' | '}');
            if !started && c == '~' {
                expand_home = true;
            }
            started = true;
            text.push(c);
        }
    }
    if started {
        words.push(Word { text, expand_home });
    }
    let Some(first) = words.first() else {
        return Ok(Parsed::Empty);
    };
    if complex || !matches!(first.text.as_str(), "cd" | "pwd") {
        return Ok(Parsed::External(input.to_owned()));
    }
    if quote.is_some() {
        return Err("Unclosed quote in builtin command".into());
    }
    if let Some(error) = error {
        return Err(error.into());
    }
    if unsupported {
        return Err("Builtins do not support variables, command substitution, globbing or brace expansion; quote literal names".into());
    }
    if first.text == "pwd" {
        return if words.len() == 1 {
            Ok(Parsed::Pwd)
        } else {
            Err("Usage: pwd (no arguments)".into())
        };
    }
    words.remove(0);
    if words.first().is_some_and(|word| word.text == "--") {
        words.remove(0);
    }
    if words.len() > 1 {
        return Err("Usage: cd [directory]".into());
    }
    Ok(Parsed::Cd(words.pop()))
}
