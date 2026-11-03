use super::parser::{Parsed, Word};
use crate::{
    app::BrowserState,
    fs::{DirectorySource, entry::safe_text},
};
use std::path::{Path, PathBuf};

pub fn execute(
    command: &Parsed,
    browser: &mut BrowserState,
    source: &mut impl DirectorySource,
    home: Option<&Path>,
) -> Result<String, String> {
    match command {
        Parsed::Pwd => Ok(safe_text(&browser.active().directory.to_string_lossy())),
        Parsed::Cd(argument) => {
            let destination = destination(argument.as_ref(), browser, home)?;
            browser.navigate_to(&destination, source)?;
            Ok(safe_text(&browser.active().directory.to_string_lossy()))
        }
        _ => Err("Not an Arbor builtin".into()),
    }
}

fn destination(
    argument: Option<&Word>,
    browser: &BrowserState,
    home: Option<&Path>,
) -> Result<PathBuf, String> {
    let Some(word) = argument else {
        return home.map(Path::to_path_buf).ok_or("HOME is not set".into());
    };
    if word.text == "-" {
        return browser
            .previous_directory
            .clone()
            .ok_or("No previous directory".into());
    }
    if word.text.is_empty() {
        return Err("Directory must not be empty".into());
    }
    let path = if word.expand_home {
        let home = home.ok_or("HOME is not set")?;
        if word.text == "~" {
            home.to_path_buf()
        } else if let Some(relative) = word.text.strip_prefix("~/") {
            home.join(relative)
        } else {
            return Err("Only ~ and ~/... home expansion are supported".into());
        }
    } else {
        PathBuf::from(&word.text)
    };
    Ok(if path.is_absolute() {
        path
    } else {
        browser.active().directory.join(path)
    })
}
