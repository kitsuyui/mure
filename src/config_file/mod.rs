//! module for parse config file
//!
//! Usually config file is located at ~/.mure.toml

use std::io::Error;

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
