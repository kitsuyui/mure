use clap::App;

fn main() {
    let cmd = parser();
    let matches = cmd.get_matches();
    match matches.subcommand() {
        Some(("refresh", matches)) => {
            println!("refresh");
            println!("{:?}", matches);
        }
        Some(("issues", matches)) => {
            println!("issues");
            println!("{:?}", matches);
        }
        _ => unreachable!("unreachable!"),
    };
}

/// Parser
fn parser() -> App<'static> {
    let subcommand_refresh = App::new("refresh").about("refresh repository");
    let subcommand_issues = App::new("issues").about("show issues");
    let cmd = clap::Command::new("mure")
        .bin_name("mure")
        .subcommand_required(true)
        .subcommand(subcommand_refresh)
        .subcommand(subcommand_issues);
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
}
