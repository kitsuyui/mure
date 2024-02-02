/// Implementation of the edit subcommand
use std::path::PathBuf;

use git2::Repository;

use crate::config::{Config, ConfigSupport};
use crate::mure_error::Error;

pub fn edit(config: &Config, repository: String) -> Result<(), Error> {
    let mure_root_dir = config.base_path();
    let path = mure_root_dir.join(repository);
    let editor = get_editor(config, &path)?;
    open_editor(&editor, &path)?;
    Ok(())
}

pub fn open_editor(editor: &str, path: &PathBuf) -> Result<(), Error> {
    // editor is not only a command name but also can have arguments, so first separate the arguments
    // maybe we can use shlex crate to parse the command and arguments
    let mut editor_args = editor.split_whitespace();
    if let Some(editor) = editor_args.next() {
        let result = std::process::Command::new(editor)
            .args(editor_args)
            .arg(path)
            .status()?;
        if !result.success() {
            return Err(Error::from_str("Failed to open editor"));
        }
    }
    Ok(())
}

/// Get the editor by priority
/// 1. editor in the config file
/// 2. git config core.editor
/// 3. $EDITOR environment variable
/// 4. error if none of the above is set
fn get_editor(config: &Config, path: &PathBuf) -> Result<String, Error> {
    if let Ok(editor) = get_editor_from_config(config) {
        return Ok(editor);
    }

    if let Ok(editor) = get_editor_from_git_config(path) {
        return Ok(editor);
    }

    if let Ok(editor) = get_editor_from_env() {
        return Ok(editor);
    }

    Err(Error::from_str("No editor found"))
}

fn get_editor_from_config(config: &Config) -> Result<String, Error> {
    if let Some(editor) = config.core.editor.as_ref() {
        return Ok(editor.to_string());
    }
    Err(Error::from_str("No editor found"))
}

fn get_editor_from_git_config(path: &PathBuf) -> Result<String, Error> {
    let repo = Repository::open(path)?;
    let config = repo.config()?;
    if let Ok(editor) = config.get_string("core.editor") {
        return Ok(editor);
    }
    Err(Error::from_str("No editor found"))
}

fn get_editor_from_env() -> Result<String, Error> {
    if let Ok(editor) = std::env::var("EDITOR") {
        return Ok(editor);
    }
    if let Ok(editor) = std::env::var("VISUAL") {
        return Ok(editor);
    }
    Err(Error::from_str("No editor found"))
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use assay::assay;

    use super::*;
    use crate::config::tests::get_test_config;

    #[test]
    fn test_get_get_editor_from_config() {
        let config = get_test_config();
        let result = get_editor_from_config(&config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "great_editor");
    }

    #[test]
    fn test_get_get_editor_from_git_config() {
        let temp = mktemp::Temp::new_dir().unwrap();
        let repo = git2::Repository::init(temp.as_path()).unwrap();
        repo.config()
            .unwrap()
            .set_str("core.editor", "git_editor")
            .unwrap();
        let result = get_editor_from_git_config(temp.borrow());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "git_editor");
    }

    #[assay(
        env = [
            ("EDITOR", "super_editor"),
        ]
    )]
    #[test]
    fn test_get_get_editor_from_env() {
        let result = get_editor_from_env();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "super_editor");
    }

    #[test]
    fn test_open_editor() {
        let temp = mktemp::Temp::new_dir().unwrap();
        let path = temp.as_path().join("test_dir");
        std::fs::write(&path, "test_dir").unwrap();
        let result = open_editor("test", &path);
        assert!(result.is_ok());
    }
}
