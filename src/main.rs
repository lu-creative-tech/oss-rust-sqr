mod cli;
mod context;
mod app;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::CliArgs::read_and_validate_args()?;
    println!("args: {:?}", args);

    let app_context = context::from_cli_args(args).await?;
    println!("app_context: {:?}", app_context);
    
    Ok(())
}
