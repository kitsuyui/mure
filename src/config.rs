//! module for parse config file
//!
//! Usually config file is located at ~/.mure.toml

use crate::mure_error::Error;

use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub core: Core,
    pub github: GitHub,
    pub shell: Option<Shell>,
}

#[derive(Serialize, Deserialize)]
pub struct Core {
    pub base_dir: String,
}

#[derive(Serialize, Deserialize)]
pub struct GitHub {
    // TODO: try .gitconfig.user.name if not set
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct Shell {
    pub cd_shims: Option<String>,
}

pub trait ConfigSupport {
    fn base_path(&self) -> PathBuf;
    fn repos_store_path(&self) -> PathBuf;
    fn repo_store_path(&self, domain: &str, owner: &str, repo: &str) -> PathBuf;
    fn repo_work_path(&self, domain: &str, owner: &str, repo: &str) -> PathBuf;
    fn resolve_cd_shims(&self) -> String;
}

impl ConfigSupport for Config {
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
    fn resolve_cd_shims(&self) -> String {
        let default = "mucd".to_string();
        match &self.shell {
            Some(shell) => shell.cd_shims.clone().unwrap_or(default),
            None => default,
        }
    }
}

/// read $HOME/.mure.toml to get config
pub fn get_config() -> Result<Config, Error> {
    let config_path = resolve_config_path();
    let content = std::fs::read_to_string(config_path?)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn initialize_config() -> Result<Config, Error> {
    let path = resolve_config_path()?;
    if path.exists() {
        return Err(Error::from_str("config file already exists"));
    }
    let config = create_config(&path)?;
    Ok(config)
}

fn create_config(path: &Path) -> Result<Config, Error> {
    let config = Config {
        core: Core {
            base_dir: "~/.dev".to_string(),
        },
        github: GitHub {
            username: "".to_string(),
        },
        shell: Some(Shell {
            cd_shims: Some("mucd".to_string()),
        }),
    };
    let content = toml::to_string(&config)?;
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(config)
}

/// resolve config path
///
/// Resolve mure configuration path. Usually this is $HOME/.mure.toml
fn resolve_config_path() -> Result<PathBuf, Error> {
    // TODO: Is $HOME/.murerc better?
    // Or should try ~/.config/mure.toml?

    if let Some(home) = std::env::var("MURE_CONFIG_PATH").ok() {
        return Ok(PathBuf::from(home));
    }

    Ok(PathBuf::from(
        shellexpand::tilde("~/.mure.toml").to_string(),
    ))
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Error {
        Error::from_str(&e.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(e: toml::ser::Error) -> Error {
        Error::from_str(&e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay::assay;
    use mktemp::Temp;

    #[test]
    fn test_resolve_config_path() {
        let home = std::env::var("HOME").unwrap();
        match resolve_config_path() {
            Ok(path) => assert_eq!(path.to_str().unwrap(), &format!("{home}/.mure.toml")),
            Err(err) => unreachable!("{:?}", err),
        }
    }

    #[test]
    #[assay(
        env = [
            ("MURE_CONFIG_PATH", "/tmp/mure.toml"),
        ]
      )]
    fn test_env_mure_config_path() {
        std::env::set_var("MURE_CONFIG_PATH", "/tmp/mure.toml");
        match resolve_config_path() {
            Ok(path) => assert_eq!(path.to_str().unwrap(), "/tmp/mure.toml"),
            Err(err) => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn test_parse_config() {
        let config: Config = toml::from_str(
            r#"
            [core]
            base_dir = "~/.dev"

            [github]
            username = "kitsuyui"

            [shell]
            cd_shims = "mucd"
        "#,
        )
        .unwrap();
        assert!(config.core.base_dir == "~/.dev");
        assert_eq!(config.github.username, "kitsuyui");
    }

    #[test]
    fn test_create_config() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let config_path = temp_dir.as_path().join(".mure.toml");

        create_config(&config_path).unwrap();

        // test parse config
        let config: Config =
            toml::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap();
        assert!(config.core.base_dir == "~/.dev");
        assert_eq!(config.github.username, "");
    }
}
