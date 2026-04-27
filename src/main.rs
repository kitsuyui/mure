use crate::app::{issues::show_issues_main, refresh::refresh_main};
use crate::config::ConfigSupport;
use Commands::*;
use clap::{ArgGroup, CommandFactory, Parser, Subcommand};
use clap_complete::env::{
    Bash as EnvBash, Elvish as EnvElvish, EnvCompleter, Fish as EnvFish,
    Powershell as EnvPowershell, Zsh as EnvZsh,
};
use clap_complete::{ArgValueCompleter, CompleteEnv, CompletionCandidate, Shell, generate};
use std::ffi::{OsStr, OsString};
use verbosity::Verbosity;

mod app;
mod config;
mod gh;
mod git;
mod github;
mod misc;
mod mure_error;
mod verbosity;

#[cfg(test)]
mod test_fixture;

fn main() -> Result<(), mure_error::Error> {
    if try_dynamic_completion()? {
        return Ok(());
    }

    let config = app::initialize::get_config_or_initialize()?;
    let cli = Cli::parse();
    let mut command = cli_command();
    let name = command.get_name().to_string();

    match cli.command {
        Init { shell: true } => {
            println!("{}", app::path::shell_shims(&config));
        }
        Init { shell: false } => match app::initialize::init() {
            Ok(_) => {
                println!("Initialized config file");
            }
            Err(e) => {
                println!("{e}");
            }
        },
        Completion { shell, cd: true } => {
            let shim_name = config.resolve_cd_shims();
            generate_mucd_dynamic_completion(shell, &shim_name)?;
        }
        Completion { shell, cd: false } => {
            generate(shell, &mut command, name, &mut std::io::stdout());
        }
        Refresh {
            repository,
            all,
            verbose,
            quiet,
        } => {
            let verbosity = Verbosity::from_bools(quiet, verbose);
            refresh_main(&config, all, repository, verbosity)?;
        }
        Issues { query } => {
            show_issues_main(&config, &query)?;
        }
        Clone {
            url,
            quiet,
            verbose,
        } => {
            let verbosity = Verbosity::from_bools(quiet, verbose);
            app::clone::clone(&config, &url, verbosity)?;
        }
        Path { name } => {
            app::path::path(&config, &name)?;
        }
        List { path, full } => {
            app::list::list(&config, path, full)?;
        }
        Edit { name } => {
            app::edit::edit(&config, name)?;
        }
    }
    Ok(())
}

fn cli_command() -> clap::Command {
    Cli::command()
}

fn mucd_command() -> clap::Command {
    MuCdCli::command()
}

fn completion_target_from_args(args: &[OsString]) -> Option<String> {
    let mut escaped = false;
    for arg in args.iter().skip(1) {
        if escaped {
            return arg.to_str().map(std::string::ToString::to_string);
        }
        if arg == OsStr::new("--") {
            escaped = true;
        }
    }
    None
}

fn resolve_cd_shim_name() -> String {
    config::get_config()
        .ok()
        .and_then(|c| c.shell.and_then(|s| s.cd_shims))
        .unwrap_or_else(|| "mucd".to_string())
}

fn try_dynamic_completion() -> Result<bool, mure_error::Error> {
    let Ok(complete_env) = std::env::var("COMPLETE") else {
        return Ok(false);
    };
    if complete_env.is_empty() || complete_env == "0" {
        return Ok(false);
    }

    let args: Vec<OsString> = std::env::args_os().collect();
    let shim_name = resolve_cd_shim_name();
    let target = completion_target_from_args(&args);

    let use_mucd_command = matches!(target.as_deref(), Some("mucd"))
        || target.as_deref().is_some_and(|t| t == shim_name);

    let current_dir = std::env::current_dir().ok();
    let handled = if use_mucd_command {
        CompleteEnv::with_factory(mucd_command)
            .try_complete(args, current_dir.as_deref())
            .map_err(|e| mure_error::Error::from_str(e.to_string().as_str()))?
    } else {
        CompleteEnv::with_factory(cli_command)
            .try_complete(args, current_dir.as_deref())
            .map_err(|e| mure_error::Error::from_str(e.to_string().as_str()))?
    };
    Ok(handled)
}

fn generate_mucd_dynamic_completion(
    shell: Shell,
    shim_name: &str,
) -> Result<(), mure_error::Error> {
    let env_shell: &dyn EnvCompleter = match shell {
        Shell::Bash => &EnvBash,
        Shell::Elvish => &EnvElvish,
        Shell::Fish => &EnvFish,
        Shell::PowerShell => &EnvPowershell,
        Shell::Zsh => &EnvZsh,
        _ => {
            return Err(mure_error::Error::from_str(
                "dynamic completion is not supported for this shell",
            ));
        }
    };

    env_shell
        .write_registration(
            "COMPLETE",
            "mucd",
            shim_name,
            "mure",
            &mut std::io::stdout(),
        )
        .map_err(|e| mure_error::Error::from_str(e.to_string().as_str()))
}

fn mucd_target_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(prefix) = current.to_str() else {
        return vec![];
    };
    let Ok(config) = config::get_config() else {
        return vec![];
    };

    let mut names: Vec<String> = app::list::search_mure_repo(&config)
        .into_iter()
        .filter_map(Result::ok)
        .map(|repo| repo.repo.repo)
        .filter(|name| name.starts_with(prefix))
        .collect();

    names.sort();
    names.dedup();
    names.into_iter().map(CompletionCandidate::new).collect()
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, next_line_help = true, name = "mure")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, next_line_help = true, name = "mucd")]
struct MuCdCli {
    #[arg(
        index = 1,
        help = "repository name",
        add = ArgValueCompleter::new(mucd_target_completer)
    )]
    target: String,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    #[command(about = "create ~/.mure.toml")]
    Init {
        #[arg(short, long, help = "Output shims for mure. To be evaluated in shell.")]
        shell: bool,
    },
    #[command(about = "completion for shell")]
    Completion {
        #[arg(
            short,
            long,
            help = "Output completion for shell. To be evaluated in shell."
        )]
        shell: Shell,
        #[arg(
            short,
            long,
            help = "Output completion for mucd. To be evaluated in shell.",
            default_value = "false"
        )]
        cd: bool,
    },
    #[command(about = "refresh repository")]
    #[clap(group(ArgGroup::new("verbosity").args(&["verbose", "quiet"])))]
    Refresh {
        #[arg(
            index = 1,
            help = "repository to refresh. if not specified, current directory is used"
        )]
        repository: Option<String>,
        #[arg(
            short,
            long,
            help = "refresh all repositories",
            default_value = "false"
        )]
        all: bool,
        #[arg(short, long, help = "verbose", default_value = "false")]
        verbose: bool,
        #[arg(short, long, help = "quiet", default_value = "false")]
        quiet: bool,
    },
    #[command(about = "show issues")]
    Issues {
        // #[arg(short = 'Q', long, help = "query to search issues")]
        // query: Option<String>,

        // multiple arguments
        #[arg(short = 'Q', long, help = "query to search issues")]
        query: Vec<String>,
    },
    #[command(about = "clone repository")]
    #[clap(group(ArgGroup::new("verbosity").args(&["verbose", "quiet"])))]
    Clone {
        #[arg(index = 1, help = "repository url")]
        url: String,
        #[arg(short, long, help = "verbose", default_value = "false")]
        verbose: bool,
        #[arg(short, long, help = "quiet", default_value = "false")]
        quiet: bool,
    },
    #[command(about = "show repository path for name")]
    Path {
        #[arg(index = 1, help = "repository name")]
        name: String,
    },
    #[command(about = "list repositories")]
    List {
        #[arg(short, long, help = "show full name")]
        full: bool,
        #[arg(short, long, help = "show path")]
        path: bool,
    },
    #[command(about = "edit repository")]
    Edit {
        #[arg(index = 1, help = "repository name")]
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::Command;
    use mktemp::Temp;
    use predicates::prelude::*;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn test_help() {
        let assert = Command::new("cargo")
            .args(vec!["run", "--", "--help"])
            .assert();
        assert.success().stdout(predicate::str::contains("Usage:"));
    }

    #[test]
    fn test_init_shell() {
        let assert = Command::new("cargo")
            .args(vec!["run", "--", "init", "--shell"])
            .assert();
        assert
            .success()
            .stdout(predicate::str::contains("function mucd() {"));
    }

    #[test]
    fn test_init() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let mure_config_path = temp_dir.as_path().join(".mure.toml");
        let assert = Command::new("cargo")
            .env("MURE_CONFIG_PATH", mure_config_path)
            .args(vec!["run", "--", "init"])
            .assert();
        assert.success().stdout(
            predicate::str::contains("Initialized config file")
                .or(predicate::str::contains("config file already exists")),
        );
        drop(temp_dir);
    }

    #[test]
    fn test_refresh() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let mure_config_path = temp_dir.as_path().join(".mure.toml");
        let base_dir = Temp::new_dir().expect("failed to create temp dir");
        let content = format!(
            r#"
[core]
base_dir = "{}"

[github]
username = "kitsuyui"

[shell]
cd_shims = "mucd"
"#,
            base_dir.as_path().to_str().unwrap()
        );
        std::fs::write(&mure_config_path, content).unwrap();
        let assert = Command::new("cargo")
            .env("MURE_CONFIG_PATH", mure_config_path)
            .args(vec!["run", "--", "refresh", "--all"])
            .assert();
        assert.success();
        drop(temp_dir);
        drop(base_dir);
    }

    #[test]
    fn test_path_error_exits_with_failure() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let mure_config_path = temp_dir.as_path().join(".mure.toml");
        let base_dir = Temp::new_dir().expect("failed to create temp dir");
        let content = format!(
            r#"
[core]
base_dir = "{}"

[github]
username = "kitsuyui"

[shell]
cd_shims = "mucd"
"#,
            base_dir.as_path().to_str().unwrap()
        );
        std::fs::write(&mure_config_path, content).unwrap();
        let assert = Command::new("cargo")
            .env("MURE_CONFIG_PATH", mure_config_path)
            .args(vec!["run", "--", "path", "missing"])
            .assert();
        assert
            .failure()
            .stderr(predicate::str::contains("missing is not a git repository"));
        drop(temp_dir);
        drop(base_dir);
    }

    #[test]
    fn test_clone_and_refresh() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let mure_config_path = temp_dir.as_path().join(".mure.toml");
        let base_dir = Temp::new_dir().expect("failed to create temp dir");
        let content = format!(
            r#"
[core]
base_dir = "{}"

[github]
username = "kitsuyui"

[shell]
cd_shims = "mucd"
"#,
            base_dir.as_path().to_str().unwrap()
        );
        std::fs::write(&mure_config_path, content).unwrap();
        let assert = Command::new("cargo")
            .env("MURE_CONFIG_PATH", &mure_config_path)
            .args(vec![
                "run",
                "--",
                "clone",
                "https://github.com/kitsuyui/mure.git",
            ])
            .assert();
        assert.success();

        let assert = Command::new("cargo")
            .env("MURE_CONFIG_PATH", &mure_config_path)
            .args(vec!["run", "--", "refresh", "mure"])
            .assert();
        assert.success();

        drop(temp_dir);
        drop(base_dir);
    }
    #[test]
    fn test_completion() {
        let assert = Command::new("cargo")
            .args(vec!["run", "--", "completion", "--shell", "bash"])
            .assert();
        assert
            .success()
            .stdout(predicate::str::contains("complete -F _mure"));

        let assert = Command::new("cargo")
            .args(vec!["run", "--", "completion", "--shell", "zsh"])
            .assert();
        assert
            .success()
            .stdout(predicate::str::contains("_mure \"$@\""));

        let assert = Command::new("cargo")
            .args(vec!["run", "--", "completion", "--shell", "zsh", "--cd"])
            .assert();
        assert
            .success()
            .stdout(predicate::str::contains("_clap_dynamic_completer_mucd"));
    }

    #[test]
    #[cfg(unix)]
    fn test_dynamic_completion_for_mucd_target() {
        let temp_dir = Temp::new_dir().expect("failed to create temp dir");
        let mure_config_path = temp_dir.as_path().join(".mure.toml");
        let base_dir = Temp::new_dir().expect("failed to create temp dir");
        let repo_store_path = base_dir
            .as_path()
            .join("repo")
            .join("github.com")
            .join("kitsuyui")
            .join("mure");
        std::fs::create_dir_all(
            repo_store_path
                .parent()
                .expect("failed to get parent directory"),
        )
        .expect("failed to create repo store parent");
        git2::Repository::init(&repo_store_path).expect("failed to initialize git repository");
        symlink(&repo_store_path, base_dir.as_path().join("mure"))
            .expect("failed to create symlink");

        let content = format!(
            r#"
[core]
base_dir = "{}"

[github]
username = "kitsuyui"

[shell]
cd_shims = "mucd"
"#,
            base_dir.as_path().to_str().unwrap()
        );
        std::fs::write(&mure_config_path, content).expect("failed to write config");

        let assert = Command::new("cargo")
            .env("MURE_CONFIG_PATH", &mure_config_path)
            .env("COMPLETE", "bash")
            .env("_CLAP_COMPLETE_INDEX", "1")
            .env("_CLAP_COMPLETE_COMP_TYPE", "9")
            .env("_CLAP_COMPLETE_SPACE", "true")
            .env("_CLAP_IFS", "\n")
            .args(vec!["run", "--", "--", "mucd", "mu"])
            .assert();
        assert.success().stdout(predicate::str::contains("mure"));
    }

    #[test]
    fn test_parser() {
        match Cli::parse_from(vec!["mure", "init"]) {
            Cli {
                command: Commands::Init { shell: false },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "init", "--shell"]) {
            Cli {
                command: Commands::Init { shell: true },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "refresh"]) {
            Cli {
                command:
                    Commands::Refresh {
                        repository: None,
                        all: false,
                        quiet: false,
                        verbose: false,
                    },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "refresh", "react", "--quiet"]) {
            Cli {
                command:
                    Commands::Refresh {
                        repository: Some(repo),
                        all: false,
                        quiet: true,
                        verbose: false,
                    },
            } => assert_eq!(repo, "react"),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "refresh", "--all", "--verbose"]) {
            Cli {
                command:
                    Commands::Refresh {
                        repository: None,
                        all: true,
                        quiet: false,
                        verbose: true,
                    },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "issues"]) {
            Cli {
                command: Commands::Issues { query },
            } => {
                assert_eq!(query, vec![] as Vec<String>);
            }
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "issues", "--query", "is:public"]) {
            Cli {
                command: Commands::Issues { query },
            } => assert_eq!(query, vec!["is:public"]),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "clone", "https://github.com/kitsuyui/mure"]) {
            Cli {
                command:
                    Commands::Clone {
                        url,
                        quiet: false,
                        verbose: false,
                    },
            } => assert_eq!(url, "https://github.com/kitsuyui/mure"),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "path", "mure"]) {
            Cli {
                command: Commands::Path { name },
            } => assert_eq!(name, "mure"),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "list"]) {
            Cli {
                command:
                    Commands::List {
                        full: false,
                        path: false,
                    },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "list", "--full"]) {
            Cli {
                command:
                    Commands::List {
                        full: true,
                        path: false,
                    },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "list", "--path"]) {
            Cli {
                command:
                    Commands::List {
                        full: false,
                        path: true,
                    },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "list", "--full", "--path"]) {
            Cli {
                command:
                    Commands::List {
                        full: true,
                        path: true,
                    },
            } => (),
            _ => panic!("failed to parse"),
        }
    }
}
