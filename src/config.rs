//! module for parse config file
//!
//! Usually config file is located at ~/.mure.toml

use crate::mure_error::Error;

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
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
    pub editor: Option<String>,
}

/// Format string for the fallback query used when neither `query` nor `queries` is set.
/// Substitute `{}` with the GitHub username. The resulting query limits results to
/// public, non-fork, non-archived repositories owned by that user.
pub const DEFAULT_QUERY_TEMPLATE: &str = "user:{} is:public fork:false archived:false";
const LEGACY_QUERY_DEPRECATION_WARNING: &str = concat!(
    "`github.query` is deprecated. ",
    "Use `github.queries = [\"...\"]` instead; ",
    "`github.query` is kept only for legacy config files and will be removed in a future breaking release."
);

#[derive(Serialize, Deserialize)]
pub struct GitHub {
    // TODO: try .gitconfig.user.name if not set
    pub username: String,
    /// Deprecated single GitHub search query string.
    ///
    /// This field is mutually exclusive with `queries` and is kept only for
    /// existing config files. New configs should use `queries`; this compatibility
    /// shim can be removed in a future breaking release after release notes call
    /// out the migration path.
    pub query: Option<String>,
    /// Multiple GitHub search query strings. Mutually exclusive with `query`.
    /// When set to an empty list (`queries = []`), no repositories are searched.
    pub queries: Option<Vec<String>>,
}

impl GitHub {
    /// Return the list of GitHub search queries to use.
    ///
    /// Resolution order:
    /// 1. `queries` if set (may be empty, in which case no search is performed).
    /// 2. `query` if set (wrapped in a single-element `Vec`).
    /// 3. [`DEFAULT_QUERY_TEMPLATE`] expanded with `username` — filters to public,
    ///    non-fork, non-archived repositories. **This fallback excludes private and
    ///    forked repositories.** Set `queries` or `query` explicitly to change scope.
    pub fn get_queries(&self) -> Result<Vec<String>, Error> {
        if self.query.is_some() && self.queries.is_some() {
            return Err(Error::from_str(
                "Both query and queries are set. Please set only one of them.",
            ));
        }
        if let Some(qs) = &self.queries {
            return Ok(qs.clone());
        }
        if let Some(q) = &self.query {
            eprintln!("{LEGACY_QUERY_DEPRECATION_WARNING}");
            return Ok(vec![q.to_string()]);
        }
        Ok(vec![DEFAULT_QUERY_TEMPLATE.replace("{}", &self.username)])
    }
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

pub fn validate_cd_shim_name(name: &str) -> Result<(), Error> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(Error::from_str(
            "shell.cd_shims must be a valid shell function name",
        ));
    };

    if !(first == '_' || first.is_ascii_alphabetic())
        || chars.any(|c| !(c == '_' || c.is_ascii_alphanumeric()))
    {
        return Err(Error::from_str(
            "shell.cd_shims must be a valid shell function name",
        ));
    }

    Ok(())
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
    fn repo_work_path(&self, domain: &str, owner: &str, repo: &str) -> PathBuf {
        self.base_path().join(domain).join(owner).join(repo)
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
            editor: None,
        },
        github: GitHub {
            username: "".to_string(),
            query: None,
            queries: Some(vec![]),
        },
        shell: Some(Shell {
            cd_shims: Some("mucd".to_string()),
        }),
    };
    let content = toml::to_string(&config)?;
    write_config_atomically(path, &content)?;
    Ok(config)
}

fn write_config_atomically(path: &Path, content: &str) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| Error::from_str("config path has no file name"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_path = parent.join(format!(
        ".{}.tmp-{}-{timestamp}",
        file_name.to_string_lossy(),
        std::process::id()
    ));

    let result = (|| -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

/// resolve config path
///
/// Resolve mure configuration path. Usually this is $HOME/.mure.toml
pub(crate) fn resolve_config_path() -> Result<PathBuf, Error> {
    // TODO: Is $HOME/.murerc better?
    // Or should try ~/.config/mure.toml?

    if let Ok(home) = std::env::var("MURE_CONFIG_PATH") {
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
pub mod tests {
    use super::*;
    use assay::assay;
    use mktemp::Temp;

    pub fn get_test_config() -> Config {
        Config {
            core: Core {
                base_dir: "~/.dev".to_string(),
                editor: Some("great_editor".to_string()),
            },
            github: GitHub {
                username: "".to_string(),
                query: None,
                queries: Some(vec![]),
            },
            shell: Some(Shell {
                cd_shims: Some("mucd".to_string()),
            }),
        }
    }

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
        match resolve_config_path() {
            Ok(path) => assert_eq!(path.to_str().unwrap(), "/tmp/mure.toml"),
            Err(err) => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn test_get_queries_default_excludes_private_and_forks() {
        // When neither query nor queries is configured, get_queries() falls back to
        // DEFAULT_QUERY_TEMPLATE which filters to public, non-fork, non-archived repos.
        // Users with private or forked repositories should set query/queries explicitly.
        let github = GitHub {
            username: "testuser".to_string(),
            query: None,
            queries: None,
        };
        let queries = github.get_queries().unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(
            queries[0],
            "user:testuser is:public fork:false archived:false"
        );
    }

    #[test]
    fn test_get_queries_empty_queries_returns_empty() {
        // Setting queries to an empty list disables repository search entirely,
        // even though neither query nor queries carries an explicit value.
        let github = GitHub {
            username: "testuser".to_string(),
            query: None,
            queries: Some(vec![]),
        };
        let queries = github.get_queries().unwrap();
        assert!(queries.is_empty());
    }

    #[test]
    fn test_get_queries_legacy_query_returns_single_query() {
        let github = GitHub {
            username: "testuser".to_string(),
            query: Some("owner:kitsuyui".to_string()),
            queries: None,
        };
        let queries = github.get_queries().unwrap();
        assert_eq!(queries, vec!["owner:kitsuyui"]);
    }

    #[test]
    fn test_get_queries_rejects_query_and_queries() {
        let github = GitHub {
            username: "testuser".to_string(),
            query: Some("owner:kitsuyui".to_string()),
            queries: Some(vec!["owner:gitignore-in".to_string()]),
        };
        let err = github.get_queries().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Both query and queries are set. Please set only one of them."
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

            [shell]
            cd_shims = "mucd"
        "#,
        )
        .unwrap();
        assert!(config.core.base_dir == "~/.dev");
        assert_eq!(config.github.username, "kitsuyui");
    }

    #[test]
    fn test_repo_work_path_includes_domain_owner_and_repo() {
        let config = get_test_config();
        assert_eq!(
            config.repo_work_path("github.com", "kitsuyui", "mure"),
            config
                .base_path()
                .join("github.com")
                .join("kitsuyui")
                .join("mure")
        );
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
        let temp_files: Vec<_> = std::fs::read_dir(temp_dir.as_path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(temp_files.is_empty());
    }
}
