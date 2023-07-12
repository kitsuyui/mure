use std::path::PathBuf;

use crate::config::{Config, ConfigSupport};
use crate::mure_error::Error;

pub fn path(config: &Config, name: &str) -> Result<(), Error> {
    println!("{}", resolve(config, name)?.display());
    Ok(())
}

pub fn shell_shims(config: &Config) -> String {
    let fn_name = config.resolve_cd_shims();
    shell_shims_for_cd_directly("mure", &fn_name)
}

fn shell_shims_for_cd_directly(bin_name: &str, fn_name: &str) -> String {
    format!("function {fn_name}() {{ local p=$({bin_name} path \"$1\") && cd \"$p\" }}\n")
}

fn resolve(config: &Config, name: &str) -> Result<PathBuf, Error> {
    let path_ = config.base_path().join(name);
    if path_.is_dir() && path_.exists() {
        return Ok(path_);
    }
    Err(Error::from_str(
        format!("{} is not a git repository", path_.display()).as_str(),
    ))
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
        assert!(path2
            .unwrap_err()
            .to_string()
            .ends_with("test_repo2 is not a git repository"));
    }

    #[test]
    fn test_shell_shims() {
        let config = Config {
            core: Core {
                base_dir: "".to_string(),
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
            shims,
            "function mucd() { local p=$(mure path \"$1\") && cd \"$p\" }\n"
        );
    }
}
