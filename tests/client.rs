use basic_proto::echoer_client::EchoerClient;
use basic_proto::EchoRequest;
pub mod basic_proto {
    tonic::include_proto!("basic_proto");
}

fn get_echoer_client(
) -> Result<EchoerClient<tonic::transport::Channel>, Box<dyn std::error::Error>>
{
    EchoerClient::connect("http://[::1]:50051")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(threaded_scheduler)]
    async fn echo_works() -> Result<(), Box<dyn std::error::Error>> {
        let mut client = get_echoer_client()?;

        let message = "Test echo is working!";
        let request = tonic::Request::new(EchoRequest {
            message: message.into(),
        });

        let response = client.echo(request).await?;

        assert!(response.into_inner().message == message);

        Ok(())
    }
}
