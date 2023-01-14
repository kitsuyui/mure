use clap::{command, Parser, Subcommand};

mod app;
mod config;
mod gh;
mod git;
mod github;
mod mure_error;

#[cfg(test)]
mod test_fixture;

fn main() -> Result<(), mure_error::Error> {
    let config = app::initialize::get_config_or_initialize()?;
    let cli = Cli::parse();
    use Commands::*;
    match cli.command {
        Init { shell: true } => {
            println!("{}", app::path::shell_shims(&config));
        }
        Init { shell: false } => match app::initialize::init() {
            Ok(_) => {
                println!("Initialized config file");
            }
            Err(e) => {
                println!("{}", e);
            }
        },
        Refresh { repository } => {
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
                        println!("{}", message);
                    }
                }
                Err(e) => println!("{}", e),
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
                Err(e) => println!("{}", e),
            }
        }
        Clone { url } => match app::clone::clone(&config, &url) {
            Ok(_) => (),
            Err(e) => println!("{}", e),
        },
        Path { name } => match app::path::path(&config, &name) {
            Ok(_) => (),
            Err(e) => println!("{}", e),
        },
        List { path, full } => match app::list::list(&config, path, full) {
            Ok(_) => (),
            Err(e) => println!("{}", e),
        },
    }
    Ok(())
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
#[command(next_line_help = true)]
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
    #[command(about = "refresh repository")]
    Refresh {
        #[arg(
            index = 1,
            help = "repository to refresh. if not specified, current directory is used"
        )]
        repository: Option<String>,
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
            command: Commands::Refresh { repository: None },
        } => (),
        _ => panic!("failed to parse"),
    }

    match Cli::parse_from(vec!["mure", "refresh", "react"]) {
        Cli {
            command: Commands::Refresh {
                repository: Some(repo),
            },
        } => assert_eq!(repo, "react"),
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
