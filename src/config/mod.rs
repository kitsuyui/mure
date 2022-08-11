//! module for parse config file
//!
//! Usually config file is located at ~/.mure.toml

use std::{io::Error, path::{PathBuf, Path}};

use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub core: Core,
    pub github: GitHub,
}

#[derive(Deserialize)]
pub struct Core {
    pub base_dir: String,
}

#[derive(Deserialize)]
pub struct GitHub {
    // TODO: try .gitconfig.user.name if not set
    pub username: String,
}

pub trait ConfigService {
    fn base_path(&self) -> PathBuf;
    fn repos_store_path(&self) -> PathBuf;
    fn repo_store_path(&self, domain: &str, owner: &str, repo: &str) -> PathBuf;
    fn repo_work_path(&self, domain: &str, owner: &str, repo: &str) -> PathBuf;
}

impl ConfigService for Config {
    fn base_path(&self) -> PathBuf {
        let expand_path = shellexpand::tilde(self.core.base_dir.as_str()).to_string();
        Path::new(expand_path.as_str()).to_path_buf()
    }
    fn repos_store_path(&self) -> PathBuf {
        self.base_path().join("repo")
    }
    fn repo_store_path(&self, domain: &str, owner: &str, repo: &str) -> PathBuf {
        self.repos_store_path().join(domain).join(owner).join(repo)
    }
    fn repo_work_path(&self, _domain: &str, _owner: &str, repo: &str) -> PathBuf {
        self.base_path().join(repo)
    }
}

/// read $HOME/.mure.toml to get config
pub fn get_config() -> Result<Config, Error> {
    let config_path = resolve_config_path();
    let content = std::fs::read_to_string(config_path?)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

/// resolve config path
///
/// Resolve mure configuration path. Usually this is $HOME/.mure.toml
fn resolve_config_path() -> Result<String, Error> {
    // TODO: Is $HOME/.murerc better?
    // Or should try ~/.config/mure.toml?
    Ok(shellexpand::tilde("~/.mure.toml").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_resolve_config_path() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            resolve_config_path().unwrap(),
            format!("{}/.mure.toml", home)
        );
    }

    #[test]
    fn test_parse_config() {
        let config: Config = toml::from_str(
            r#"
            [core]
            base_dir = "~/.dev"

            [github]
            username = "kitsuyui"
        "#,
        )
        .unwrap();
        assert!(config.core.base_dir == "~/.dev");
        assert_eq!(config.github.username, "kitsuyui");
    }
}
