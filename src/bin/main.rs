pub(crate) mod commands;
pub(crate) mod config;
use clap::{App, Arg};

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

    // Startup call sending the possible args passed
    commands::startup::startup(
        matches.value_of("host"),
        matches.value_of("port"),
    )
    .await
    .unwrap();
}
