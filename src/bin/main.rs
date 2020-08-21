mod commands;
mod config;
use clap::{App, Arg};
use config::Config;

fn main() {
    run();
}

#[tokio::main]
async fn run() {
    let matches = App::new("Rusk")
        .version("v0.1.0")
        .author("Dusk Network B.V. All Rights Reserved.")
        .about("Rusk Server node.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .takes_value(true),
        )
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
        .get_matches();

    // If we get a configfile path, we just try to parse it and run rusk with this config.
    if let Some(config_path) = matches.value_of("config") {
        match Config::from_configfile(config_path) {
            Ok(config) => commands::startup::startup(config).await.unwrap(),
            Err(e) => {
                // We should probably log the error and specify that there was a problem reading
                // the config.
                println!("{:?}", e);
                ()
            }
        }
    };

    // Generate a default `Config` mutable object and edit the fields that were provided.
    let mut config = Config::default();

    // If a port was specified, modify the config and overwrite the default one.
    if let Some(port) = matches.value_of("port") {
        config.port = port.to_string();
    };

    // If a host was specified, modify the config and overwrite the default one.
    if let Some(host) = matches.value_of("host") {
        config.host_address = host.to_string();
    };

    // Continued program logic goes here...
    commands::startup::startup(config).await.unwrap();
}
