mod version;
use clap::{App, Arg};
use rusk::services::echoer::EchoerServer;
use rusk::Rusk;
use rustc_tools_util::{get_version_info, VersionInfo};
use tonic::transport::Server;
use version::show_version;

/// Default port that Rusk GRPC-server will listen to.
pub(crate) const PORT: &'static str = "8585";

/// Default host_address that Rusk GRPC-server will listen to.
pub(crate) const HOST_ADDRESS: &'static str = "127.0.0.1";

#[tokio::main]
async fn main() {
    let crate_info = get_version_info!();
    let matches = App::new(&crate_info.crate_name)
        .version(show_version(crate_info).as_str())
        .author("Dusk Network B.V. All Rights Reserved.")
        .about("Rusk Server node.")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("port")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("host")
                .short("h")
                .long("host")
                .value_name("host")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("log-level")
                .long("log-level")
                .value_name("LOG")
                .possible_values(&["error", "warn", "info", "debug", "trace"])
                .default_value("info")
                .help("Output log level")
                .takes_value(true),
        )
        .get_matches();

    // Match tracing desired level.
    let log = match matches
        .value_of("log-level")
        .expect("Failed parsing log-level arg")
    {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        "trace" => tracing::Level::TRACE,
        _ => unreachable!(),
    };

    // Generate a subscriber with the desired log level.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(log)
        .finish();
    // Set the subscriber as global.
    // so this subscriber will be used as the default in all threads for the remainder
    // of the duration of the program, similar to how `loggers` work in the `log` crate.
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed on subscribe tracing");

    // Startup call sending the possible args passed
    match startup(
        matches.value_of("host").unwrap_or_else(|| HOST_ADDRESS),
        matches.value_of("port").unwrap_or_else(|| PORT),
    )
    .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("{}", e),
    };
}

async fn startup(
    host: &str,
    port: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut full_address = host.to_string();
    full_address.extend(":".chars());
    full_address.extend(port.to_string().chars());
    let addr: std::net::SocketAddr = full_address.parse()?;
    let rusk = Rusk::default();

    // Build the Server with the `Echo` service attached to it.
    Ok(Server::builder()
        .add_service(EchoerServer::new(rusk))
        .serve(addr)
        .await?)
}
