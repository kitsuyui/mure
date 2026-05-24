/// Implementation of the edit subcommand
use std::path::PathBuf;

use git2::Repository;

use crate::config::Config;
use crate::mure_error::Error;

pub fn edit(config: &Config, repository: String) -> Result<(), Error> {
    let path = crate::app::path::resolve(config, &repository)?;
    let editor = get_editor(config, &path)?;
    open_editor(&editor, &path)?;
    Ok(())
}

pub fn open_editor(editor: &str, path: &PathBuf) -> Result<(), Error> {
    if editor.is_empty() {
        return Err(Error::from_str("No editor found"));
    }
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
    if let Some(editor) = config.core.editor.as_ref()
        && !editor.is_empty()
    {
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

    #[test]
    fn test_open_editor_empty_string_returns_error() {
        let temp = mktemp::Temp::new_dir().unwrap();
        let path = temp.as_path().join("test_dir");
        let result = open_editor("", &path);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_editor_from_config_empty_string_falls_through() {
        use crate::config::tests::get_test_config;
        let mut config = get_test_config();
        config.core.editor = Some("".to_string());
        let result = get_editor_from_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_rejects_absolute_path() {
        let config = get_test_config();
        let result = edit(&config, "/etc".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("simple relative name")
        );
    }

    #[test]
    fn test_edit_rejects_parent_traversal() {
        let config = get_test_config();
        let result = edit(&config, "../etc".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("simple relative name")
        );
    }
}
