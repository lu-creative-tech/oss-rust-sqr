mod app;
mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args = cli::CliArgs::read_and_validate_args()?;
    println!("{:?}", args);
    
    Ok(())
}
