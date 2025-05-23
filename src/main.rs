use crate::app::{issues::show_issues_main, refresh::refresh_main};
use clap::{command, ArgGroup, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use verbosity::Verbosity;
use Commands::*;

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
    let config = app::initialize::get_config_or_initialize()?;
    let cli = Cli::parse();
    let mut command = Cli::command();
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
        Completion { shell } => {
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
            match app::clone::clone(&config, &url, verbosity) {
                Ok(_) => (),
                Err(e) => println!("{e}"),
            }
        }
        Path { name } => match app::path::path(&config, &name) {
            Ok(_) => (),
            Err(e) => println!("{e}"),
        },
        List { path, full } => match app::list::list(&config, path, full) {
            Ok(_) => (),
            Err(e) => println!("{e}"),
        },
        Edit { name } => match app::edit::edit(&config, name) {
            Ok(_) => (),
            Err(e) => println!("{e}"),
        },
    }
    Ok(())
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, next_line_help = true, name = "mure")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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

    #[test]
    fn test_help() {
        let assert = Command::new("cargo")
            .args(vec![
                "llvm-cov",
                "--lcov",
                "--output-path",
                "coverage.lcov",
                "--no-report",
                "run",
                "--",
                "--help",
            ])
            .assert();
        assert.success().stdout(predicate::str::contains("Usage:"));
    }

    #[test]
    fn test_init_shell() {
        let assert = Command::new("cargo")
            .args(vec![
                "llvm-cov",
                "--lcov",
                "--output-path",
                "coverage.lcov",
                "--no-report",
                "run",
                "--",
                "init",
                "--shell",
            ])
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
            .args(vec![
                "llvm-cov",
                "--lcov",
                "--output-path",
                "coverage.lcov",
                "--no-report",
                "run",
                "--",
                "init",
            ])
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
            .args(vec![
                "llvm-cov",
                "--lcov",
                "--output-path",
                "coverage.lcov",
                "--no-report",
                "run",
                "--",
                "refresh",
                "--all",
            ])
            .assert();
        assert.success();
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
                "llvm-cov",
                "--lcov",
                "--output-path",
                "coverage.lcov",
                "--no-report",
                "run",
                "--",
                "clone",
                "https://github.com/kitsuyui/mure.git",
            ])
            .assert();
        assert.success();

        let assert = Command::new("cargo")
            .env("MURE_CONFIG_PATH", &mure_config_path)
            .args(vec![
                "llvm-cov",
                "--lcov",
                "--output-path",
                "coverage.lcov",
                "--no-report",
                "run",
                "--",
                "refresh",
                "mure",
            ])
            .assert();
        assert.success();

        drop(temp_dir);
        drop(base_dir);
    }
    #[test]
    fn test_completion() {
        let assert = Command::new("cargo")
            .args(vec![
                "llvm-cov",
                "--lcov",
                "--output-path",
                "coverage.lcov",
                "--no-report",
                "run",
                "--",
                "completion",
                "--shell",
                "bash",
            ])
            .assert();
        assert
            .success()
            .stdout(predicate::str::contains("complete -F _mure"));

        let assert = Command::new("cargo")
            .args(vec![
                "llvm-cov",
                "--lcov",
                "--output-path",
                "coverage.lcov",
                "--no-report",
                "run",
                "--",
                "completion",
                "--shell",
                "zsh",
            ])
            .assert();
        assert
            .success()
            .stdout(predicate::str::contains("_mure \"$@\""));
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
