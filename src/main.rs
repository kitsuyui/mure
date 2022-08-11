use clap::App;
mod clone;
mod config;
mod gh;
mod git;
mod github;
mod issues;
mod mure_error;
mod refresh;

fn main() {
    let config = config::get_config().expect("config error");
    let cmd = parser();
    let matches = cmd.get_matches();
    match matches.subcommand() {
        Some(("refresh", matches)) => {
            let current_dir = std::env::current_dir().unwrap();
            let repo_path = match matches.get_one::<String>("repository") {
                Some(repo) => repo.to_string(),
                None => current_dir.to_str().unwrap().to_string(),
            };
            match refresh::refresh(&repo_path) {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }
        }
        Some(("issues", _)) => match issues::show_issues() {
            Ok(_) => (),
            Err(e) => println!("{}", e),
        },
        Some(("clone", matches)) => {
            let repo_url = matches.get_one::<String>("url").unwrap();
            match clone::clone(&config, repo_url) {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }
        }
        _ => unreachable!("unreachable!"),
    };
}

/// Parser
fn parser() -> App<'static> {
    // TODO: subcommand "init" to create ~/.mure.toml
    let subcommand_refresh = App::new("refresh").about("refresh repository").arg(
        clap::Arg::with_name("repository")
            .help("repository to refresh. if not specified, current directory is used")
            .required(false)
            .index(1),
    );
    let subcommand_issues = App::new("issues").about("show issues");
    let subcommand_clone = App::new("clone").about("clone repository").arg(
        clap::Arg::with_name("url")
            .help("repository url")
            .required(true)
            .index(1),
    );
    let cmd = clap::Command::new("mure")
        .bin_name("mure")
        .subcommand_required(true)
        .subcommand(subcommand_refresh)
        .subcommand(subcommand_issues)
        .subcommand(subcommand_clone);
    cmd
}

#[test]
fn test_parser() {
    let cmd = parser();
    match cmd.get_matches_from_safe(["mure", "refresh"]) {
        Ok(matches) => {
            assert_eq!(matches.subcommand_name(), Some("refresh"));
        }
        Err(e) => {
            unreachable!("{}", e);
        }
    }
    let cmd = parser();
    match cmd.get_matches_from_safe(["mure", "issues"]) {
        Ok(matches) => {
            assert_eq!(matches.subcommand_name(), Some("issues"));
        }
        Err(e) => {
            unreachable!("{}", e);
        }
    }
    let cmd = parser();
    match cmd.get_matches_from_safe(["mure", "clone", "https://github.com/kitsuyui/mure"]) {
        Ok(matches) => {
            assert_eq!(matches.subcommand_name(), Some("clone"));
        }
        Err(e) => {
            unreachable!("{}", e);
        }
    }
}
