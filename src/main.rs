use clap::Command;
mod app;
mod config;
mod gh;
mod git;
mod github;
mod mure_error;

fn main() {
    let config = app::initialize::get_config_or_initialize().expect("config error");
    let cmd = parser();
    let matches = cmd.get_matches();
    match matches.subcommand() {
        Some(("init", matches)) => match matches.subcommand_matches("shell") {
            Some(_) => {
                println!("{}", app::path::shell_shims(&config));
            }
            None => match app::initialize::init() {
                Ok(_) => {
                    println!("Initialized config file");
                }
                Err(e) => {
                    println!("{}", e);
                }
            },
        },
        Some(("refresh", matches)) => {
            let current_dir = std::env::current_dir().unwrap();
            let repo_path = match matches.get_one::<String>("repository") {
                Some(repo) => repo.to_string(),
                None => current_dir.to_str().unwrap().to_string(),
            };
            match app::refresh::refresh(&repo_path) {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }
        }
        Some(("issues", matches)) => {
            let query = match matches.get_one::<String>("query") {
                Some(query) => query.to_string(),
                None => format!(
                    "user:{} is:public fork:false archived:false",
                    &config.github.username
                ),
            };
            match app::issues::show_issues(&query) {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }
        }
        Some(("clone", matches)) => {
            let repo_url = matches.get_one::<String>("url").unwrap();
            match app::clone::clone(&config, repo_url) {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }
        }
        Some(("path", matches)) => {
            let name = matches.get_one::<String>("name").unwrap();
            match app::path::path(&config, name) {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }
        }
        _ => unreachable!("unreachable!"),
    };
}

/// Parser
fn parser() -> Command {
    let subcommand_init = Command::new("init").about("create ~/.mure.toml").arg(
        clap::Arg::new("shell")
            .short('s')
            .long("shell")
            .help("Output shims for mure. To be evaluated in shell."),
    );

    let subcommand_refresh = Command::new("refresh").about("refresh repository").arg(
        clap::Arg::new("repository")
            .short('r')
            .long("repository")
            .help("repository to refresh. if not specified, current directory is used"),
    );

    let subcommand_issues = Command::new("issues").about("show issues").arg(
        clap::Arg::new("query")
            .short('q')
            .long("query")
            .help("query to search issues"),
    );

    let subcommand_clone = Command::new("clone").about("clone repository").arg(
        clap::Arg::new("url")
            .help("repository url")
            .required(true)
            .index(1),
    );

    let subcommand_path = Command::new("path")
        .about("show repository path for name")
        .arg(
            clap::Arg::new("name")
                .help("repository name")
                .required(true)
                .index(1),
        );

    clap::Command::new("mure")
        .bin_name("mure")
        .subcommand_required(true)
        .subcommand(subcommand_init)
        .subcommand(subcommand_refresh)
        .subcommand(subcommand_issues)
        .subcommand(subcommand_clone)
        .subcommand(subcommand_path)
}

#[test]
fn test_parser() {
    let cmd = parser();
    cmd.debug_assert();

    let cmd = parser();
    assert_eq!(
        cmd.get_matches_from(&["mure", "init"])
            .subcommand_name()
            .unwrap(),
        "init"
    );

    let cmd = parser();
    assert_eq!(
        cmd.get_matches_from(&["mure", "refresh"])
            .subcommand_name()
            .unwrap(),
        "refresh"
    );

    let cmd = parser();
    assert_eq!(
        cmd.get_matches_from(&["mure", "issues"])
            .subcommand_name()
            .unwrap(),
        "issues"
    );

    let cmd = parser();
    assert_eq!(
        cmd.get_matches_from(&["mure", "clone", "https://github.com/kitsuyui/mure"])
            .subcommand_name()
            .unwrap(),
        "clone"
    );

    let cmd = parser();
    assert_eq!(
        cmd.get_matches_from(&["mure", "path", "mure"])
            .subcommand_name()
            .unwrap(),
        "path"
    );
}
