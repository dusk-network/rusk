use rusk_lib;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // For now the address is hardcoded, but we should
    // take it from the clap args.
    rusk_lib::startup("http://[::1]:50051").await?;
    Ok(())
}
