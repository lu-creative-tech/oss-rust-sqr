
use std::{path::{PathBuf}};
use clap::{ArgAction, ArgGroup, ArgMatches, arg, command, value_parser};


#[derive(Debug)]
pub enum ArgAuthType {
    UseAzCliToken,
    UseConnectionString(PathBuf)
}

#[derive(Debug)]
pub struct CliArgs
{
    sql_file: PathBuf,
    filters_file: PathBuf,
    auth: ArgAuthType
}

impl CliArgs {

    pub fn read_and_validate_args() -> Result<Self, Box<dyn std::error::Error>> {
        let args = CliArgs::get_cli_args();
        
        let sql_file = args
            .get_one::<PathBuf>("sql_file")
            .cloned()
            .ok_or_else(|| "The report's source query file is required".to_string())?;

        let filters_file = args
            .get_one::<PathBuf>("filters")
            .cloned()
            .ok_or_else(|| "The JSON file query filters is required".to_string())?;
        
        let auth: ArgAuthType;
        let is_auth_az_cli_tokens = args.get_flag("auth_az_cli_tokens");
        if is_auth_az_cli_tokens {
            auth = ArgAuthType::UseAzCliToken;
        }
        else {
            let conn_string = args
                .get_one::<PathBuf>("auth_connection_string")
                .cloned()
                .ok_or_else(|| "The connection string is required".to_string())?;

            auth = ArgAuthType::UseConnectionString(conn_string);
        }

        Ok(
            CliArgs {
                sql_file: sql_file,
                filters_file: filters_file,
                auth: auth
            }
        )
    }

    fn get_cli_args() -> ArgMatches {
        command!()
            .arg(
                arg!(-f --"sql-file" <FILE> "The report's source query.")
                    .required(true)
                    .value_parser(value_parser!(PathBuf))
            )
            .arg(
                arg!(--filters <FILE> "JSON file query filters.")
                    .required(false)
                    .value_parser(value_parser!(PathBuf))
            )
            .arg(
                arg!(--"auth-connection-string" <FILE> "The connection string file.")
                    .required(false)
                    .value_parser(value_parser!(PathBuf))
            )
            .arg(
                arg!(--"auth-az-cli-tokens")
                    .required(false)
                    .action(ArgAction::SetTrue)
            )
            .group(
                ArgGroup::new("auth")
                    .args(["auth-connection-string", "auth-az-cli-tokens"])
                    .required(true)
                    .multiple(false)
            )
            .get_matches()
    }
}

