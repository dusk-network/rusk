use rusk::basic_proto::echoer_client::EchoerClient;
use rusk::basic_proto::echoer_server::EchoerServer;
use rusk::basic_proto::EchoRequest;
use rusk::Rusk;
use tonic::transport::Server;

pub mod basic_proto {
    tonic::include_proto!("basic_proto");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(threaded_scheduler)]
    async fn echo_works() -> Result<(), Box<dyn std::error::Error>> {
        let addr = "[::1]:50051".parse()?;
        let rusk = Rusk::default();

        tokio::spawn(async move {
            Server::builder()
                .add_service(EchoerServer::new(rusk))
                .serve(addr)
                .await
                .unwrap()
        });
        let mut client = EchoerClient::connect("http://[::1]:50051").await?;

        let message = "Test echo is working!";
        let request = tonic::Request::new(EchoRequest {
            message: message.into(),
        });

        let response = client.echo(request).await?;

        assert_eq!(response.into_inner().message, message);

        Ok(())
    }
}
