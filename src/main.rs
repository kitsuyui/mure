use clap::{command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

mod app;
mod config;
mod gh;
mod git;
mod github;
mod misc;
mod mure_error;

#[cfg(test)]
mod test_fixture;

fn main() -> Result<(), mure_error::Error> {
    use Commands::*;
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
        Refresh { repository, all } => {
            if all {
                app::refresh::refresh_all(&config)?;
            } else {
                let current_dir = std::env::current_dir()?;
                let Some(current_dir) = current_dir.to_str() else {
                return Err(mure_error::Error::from_str("failed to get current dir"));
            };
                let repo_path = match repository {
                    Some(repo) => repo,
                    None => current_dir.to_string(),
                };
                match app::refresh::refresh(&repo_path) {
                    Ok(r) => {
                        if let app::refresh::RefreshStatus::Update { message, .. } = r {
                            println!("{message}");
                        }
                    }
                    Err(e) => println!("{e}"),
                }
            }
        }
        Issues { query } => {
            let default_query = format!(
                "user:{} is:public fork:false archived:false",
                &config.github.username
            );
            let query = query.unwrap_or_else(|| default_query.to_string());
            match app::issues::show_issues(&query) {
                Ok(_) => (),
                Err(e) => println!("{e}"),
            }
        }
        Clone { url } => match app::clone::clone(&config, &url) {
            Ok(_) => (),
            Err(e) => println!("{e}"),
        },
        Path { name } => match app::path::path(&config, &name) {
            Ok(_) => (),
            Err(e) => println!("{e}"),
        },
        List { path, full } => match app::list::list(&config, path, full) {
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
    },
    #[command(about = "show issues")]
    Issues {
        #[arg(short, long, help = "query to search issues")]
        query: Option<String>,
    },
    #[command(about = "clone repository")]
    Clone {
        #[arg(index = 1, help = "repository url")]
        url: String,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::Command;
    use predicates::prelude::*;

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
        let assert = Command::new("cargo")
            .args(vec!["run", "--", "init"])
            .assert();
        assert.success().stdout(
            predicate::str::contains("Initialized config file")
                .or(predicate::str::contains("config file already exists")),
        );
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
                    },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "refresh", "react"]) {
            Cli {
                command:
                    Commands::Refresh {
                        repository: Some(repo),
                        all: false,
                    },
            } => assert_eq!(repo, "react"),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "refresh", "--all"]) {
            Cli {
                command:
                    Commands::Refresh {
                        repository: None,
                        all: true,
                    },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "issues"]) {
            Cli {
                command: Commands::Issues { query: None },
            } => (),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "issues", "--query", "is:public"]) {
            Cli {
                command: Commands::Issues { query: Some(query) },
            } => assert_eq!(query, "is:public"),
            _ => panic!("failed to parse"),
        }

        match Cli::parse_from(vec!["mure", "clone", "https://github.com/kitsuyui/mure"]) {
            Cli {
                command: Commands::Clone { url },
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
