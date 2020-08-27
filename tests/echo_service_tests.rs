#[cfg(not(target_os = "windows"))]
mod unix;
use futures::stream::TryStreamExt;
use rusk::services::echoer::{EchoRequest, EchoerClient, EchoerServer};
use rusk::Rusk;
use std::convert::TryFrom;
use std::path::Path;
use tokio::net::UnixListener;
use tokio::net::UnixStream;
use tonic::transport::Server;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use tracing::{subscriber, Level};
use tracing_subscriber::fmt::Subscriber;

/// Default UDS path that Rusk GRPC-server will connect to.
const SOCKET_PATH: &'static str = "/tmp/rusk_listener";
const SERVER_ADDRESS: &'static str = "127.0.1.1:50051";
const CLIENT_ADDRESS: &'static str = "http://127.0.1.1:50051";

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(threaded_scheduler)]
    async fn echo_works_uds() -> Result<(), Box<dyn std::error::Error>> {
        // Generate a subscriber with the desired log level.
        let subscriber =
            Subscriber::builder().with_max_level(Level::INFO).finish();
        // Set the subscriber as global.
        // so this subscriber will be used as the default in all threads for the remainder
        // of the duration of the program, similar to how `loggers` work in the `log` crate.
        subscriber::set_global_default(subscriber)
            .expect("Failed on subscribe tracing");

        // Create the server binded to the default UDS path.
        tokio::fs::create_dir_all(Path::new(SOCKET_PATH).parent().unwrap())
            .await?;

        let mut uds = UnixListener::bind(SOCKET_PATH)?;
        let rusk = Rusk::default();
        // We can't avoid the unwrap here until the async closure (#62290) lands.
        // And therefore we can force the closure to return a Result.
        // See: https://github.com/rust-lang/rust/issues/62290
        tokio::spawn(async move {
            Server::builder()
                .add_service(EchoerServer::new(rusk))
                .serve_with_incoming(uds.incoming().map_ok(unix::UnixStream))
                .await
                .unwrap();
        });

        // Create the client binded to the default testing UDS path.
        let channel = Endpoint::try_from("http://[::]:50051")?
            .connect_with_connector(service_fn(|_: Uri| {
                // Connect to a Uds socket
                UnixStream::connect(SOCKET_PATH)
            }))
            .await?;
        let mut client = EchoerClient::new(channel);

        // Actual test case.
        let message = "Test echo is working!";
        let request = tonic::Request::new(EchoRequest {
            message: message.into(),
        });

        let response = client.echo(request).await?;

        assert_eq!(response.into_inner().message, message);

        Ok(())
    }

    #[tokio::test(threaded_scheduler)]
    async fn echo_works_tcp_ip() -> Result<(), Box<dyn std::error::Error>> {
        let addr = SERVER_ADDRESS.parse()?;
        let rusk = Rusk::default();
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
