// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::{App, Arg, ArgMatches};
use consensus::util::pending_queue::PendingQueue;
use kadcast::config::Config;
use rustc_tools_util::{get_version_info, VersionInfo};

mod consensus_service;
mod network_service;
mod wire;

#[tokio::main]
pub async fn main() {
    let crate_info = get_version_info!();
    let matches = gen_matches(crate_info);

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
    // so this subscriber will be used as the default in all threads for the
    // remainder of the duration of the program, similar to how `loggers`
    // work in the `log` crate.
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed on subscribe tracing");

    let mut conf = Config::default();
    conf.public_address =
        matches.value_of("public_address").unwrap().to_string();
    conf.listen_address =
        matches.value_of("listen_address").map(|a| a.to_string());
    conf.bootstrapping_nodes = matches
        .values_of("bootstrap")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect();

    let prov_id = matches
        .value_of("prov-id")
        .unwrap()
        .to_string()
        .parse::<usize>()
        .unwrap();
    let prov_num = matches
        .value_of("preloaded-num")
        .unwrap()
        .to_string()
        .parse::<usize>()
        .unwrap();

    run_main_loop(conf, prov_num, prov_id).await;
}

/// Spawns all node layers.
async fn run_main_loop(
    conf: Config,
    provisioners_num: usize,
    provisioner_id: usize,
) {
    let inbound = PendingQueue::new("inbound_main_loop");
    let outbound = PendingQueue::new("outbound_main_loop");

    let agr_inbound = PendingQueue::new("inbound_agreement");
    let agr_outbound = PendingQueue::new("outbound_agreement");

    // Spawn consensus layer
    consensus_service::run_main_loop(
        provisioners_num,
        provisioner_id,
        inbound.clone(),
        outbound.clone(),
        agr_inbound.clone(),
        agr_outbound.clone(),
    );

    // Spawn network layer
    network_service::run_main_loop(
        conf,
        inbound,
        outbound,
        agr_inbound,
        agr_outbound,
    )
    .await;
}

fn show_version(info: VersionInfo) -> String {
    let version = format!("{}.{}.{}", info.major, info.minor, info.patch);
    let build = format!(
        "{} {}",
        info.commit_hash.unwrap_or_default(),
        info.commit_date.unwrap_or_default()
    );

    if build.len() > 1 {
        format!("{} ({})", version, build)
    } else {
        version
    }
}

fn gen_matches(crate_info: VersionInfo) -> ArgMatches<'static> {
    App::new(&crate_info.crate_name)
       .version(show_version(crate_info).as_str())
       .author("Dusk Network B.V. All Rights Reserved.")
       .about("Consensus example nodee impl.")
       .arg(
           Arg::with_name("listen_address")
               .short("l")
               .long("listen")
               .help("Internal address you want to use to listen incoming connections. Eg: 127.0.0.1:696")
               .takes_value(true)
               .required(false),
       )
       .arg(
           Arg::with_name("public_address")
               .short("p")
               .long("address")
               .help("Public address you want to be identified with. Eg: 193.xxx.xxx.198:696")
               .takes_value(true)
               .required(true),
       )
       .arg(
           Arg::with_name("bootstrap")
               .long("bootstrap")
               .short("b")
               .multiple(true)
               .help("List of bootstrapping server instances")
               .takes_value(true)
               .required(true),
       )
       .arg(
           Arg::with_name("log-level")
               .long("log-level")
               .value_name("LOG")
               .possible_values(&["error", "warn", "info", "debug", "trace"])
               .default_value("info")
               .help("Output log level")
               .takes_value(true),
       ).arg(
           Arg::with_name("prov-id")
               .long("provisioner-unique-id")
               .help("provisioner id of a hard-coded list of provisioners")
               .takes_value(true)
               .required(true),
       ).arg(
           Arg::with_name("preloaded-num")
               .long("preloaded-num")
               .takes_value(true)
               .default_value("3")
               .required(true),
       )
       .get_matches()
}
