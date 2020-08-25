use rusk::services::echoer::{EchoRequest, EchoerClient, EchoerServer};
use rusk::Rusk;
use tonic::transport::Server;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;
pub mod basic_proto {
    tonic::include_proto!("basic_proto");
}

const SERVER_ADDRESS: &'static str = "127.0.1.1:50051";
const CLIENT_ADDRESS: &'static str = "http://127.0.1.1:50051";

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(threaded_scheduler)]
    async fn echo_works() -> Result<(), Box<dyn std::error::Error>> {
        let addr = SERVER_ADDRESS.parse()?;
        let rusk = Rusk::default();
        // Generate a subscriber with the desired log level.
        let subscriber =
            Subscriber::builder().with_max_level(Level::INFO).finish();
        // Set the subscriber as global.
        // so this subscriber will be used as the default in all threads for the remainder
        // of the duration of the program, similar to how `loggers` work in the `log` crate.
        subscriber::set_global_default(subscriber)
            .expect("Failed on subscribe tracing");
        tokio::spawn(async move {
            Server::builder()
                .add_service(EchoerServer::new(rusk))
                .serve(addr)
                .await
                .unwrap()
        });
        let mut client = EchoerClient::connect(CLIENT_ADDRESS).await?;

        let message = "Test echo is working!";
        let request = tonic::Request::new(EchoRequest {
            message: message.into(),
        });

        let response = client.echo(request).await?;

        assert_eq!(response.into_inner().message, message);

        Ok(())
    }
}
