
use std::path::PathBuf;
use clap::{ArgAction, ArgGroup, ArgMatches, arg, command, value_parser};


//TODO: write the clap code to grab all options
//TODO: impl From<Args> for app::AppContext

pub struct CliContext;

fn get_cli_args() -> Result<ArgMatches, clap::error::Error> {
    command!()
        .arg(
            arg!(-f --sql-file <FILE> "The report's source query.")
                .required(true)
                .value_parser(value_parser!(PathBuf))
        )
        .arg(
            arg!(--filters <FILE> "JSON file query filters.")
                .required(false)
                .value_parser(value_parser!(PathBuf))
        )
        .arg(
            arg!(--auth-user-pwd <FILE> "The connection string file.")
                .required(false)
                .value_parser(value_parser!(PathBuf))
        )
        .arg(
            arg!(--auth-az-cli-tokens)
                .required(false)
                .action(ArgAction::SetTrue)
        )
        .group(
            ArgGroup::new("auth")
                .args(["auth_user_pwd", "auth_az_cli_tokens"])
                .required(true)
                .multiple(false)
        )
        .try_get_matches()
}

impl From<CliContext> for Result<crate::app::AppContext, Box<dyn std::error::Error>> {
    fn from(value: CliContext) -> Self {
        let args = get_cli_args()?;

        todo!()
    }
}

