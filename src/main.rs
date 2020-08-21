use rusk;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // For now the address is hardcoded, but we should
    // take it from the clap args.
    rusk::startup("http://[::1]:50051").await?;
    Ok(())
}
