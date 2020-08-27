mod unix;
mod version;

use clap::{App, Arg};
use futures::stream::TryStreamExt;
use rusk::services::echoer::EchoerServer;
use rusk::Rusk;
use rustc_tools_util::{get_version_info, VersionInfo};
use std::path::Path;
use tokio::net::UnixListener;
use tonic::transport::Server;
use version::show_version;

/// Default UDS path that Rusk GRPC-server will connect to.
pub const SOCKET_PATH: &'static str = "/tmp/rusk_listener";

#[tokio::main]
async fn main() {
    let crate_info = get_version_info!();
    let matches = App::new(&crate_info.crate_name)
        .version(show_version(crate_info).as_str())
        .author("Dusk Network B.V. All Rights Reserved.")
        .about("Rusk Server node.")
        .arg(
            Arg::with_name("socket")
                .short("s")
                .long("socket")
                .value_name(SOCKET_PATH)
                .help("Path for setting up the UDS")
                .default_value("./rusk_listener")
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
    match startup(matches.value_of("socket").unwrap_or_else(|| SOCKET_PATH))
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("{}", e),
    };
}

async fn startup(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(Path::new(path).parent().unwrap()).await?;

    let mut uds = UnixListener::bind(path)?;

    let rusk = Rusk::default();

    Server::builder()
        .add_service(EchoerServer::new(rusk))
        .serve_with_incoming(uds.incoming().map_ok(unix::UnixStream))
        .await?;

    Ok(())
}
