use basic_proto::echoer_client::EchoerClient;
use basic_proto::EchoRequest;
use rusk_lib::startup;

pub mod basic_proto {
    tonic::include_proto!("basic_proto");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(threaded_scheduler)]
    async fn echo_works() -> Result<(), Box<dyn std::error::Error>> {
        tokio::spawn(async move { startup("http://[::1]:50051").await });
        let mut client = EchoerClient::connect("http://[::1]:50051").await?;

        let message = "Test echo is working!";
        let request = tonic::Request::new(EchoRequest {
            message: message.into(),
        });

        let response = client.echo(request).await?;

        assert!(response.into_inner().message == message);

        Ok(())
    }
}
