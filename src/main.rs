#[macro_use]
extern crate log;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde_derive;

extern crate toml;
extern crate rand;
extern crate serde;
extern crate notify;
extern crate openssl;
extern crate futures;
extern crate tokio_uds;
extern crate tokio_core;
extern crate serde_json;
extern crate pretty_env_logger;
extern crate sozu_command_futures as command;
extern crate sozu_command_lib as sozu_command;

mod rpc;
mod util;
mod parser;
mod watcher;

use clap::{App, Arg};
use sozu_command::config::Config;

use std::time::Duration;

fn main() {
    pretty_env_logger::init();

    let matches = App::new("sozuconfw")
        .version(crate_version!())
        .about("Watch sozu app routing configs for updates")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("What sozu config to read from")
            .default_value("config.toml")
            .takes_value(true)
            .required(false)
        )
        .arg(Arg::with_name("apps")
            .short("a")
            .long("apps")
            .value_name("FILE")
            .help("What application config file to watch")
            .default_value("applications.toml")
            .takes_value(true)
            .required(false)
        )
        .arg(Arg::with_name("watch")
            .short("w")
            .long("watch")
            .value_name("SECONDS")
            .help("How often to check for file changes")
            .default_value("5")
            .takes_value(true)
            .required(false)
        )
        .get_matches();

    let applications_file = matches.value_of("apps").unwrap();

    let sozu_config_path = matches.value_of("config").unwrap();
    let sozu_config = Config::load_from_path(sozu_config_path).unwrap();

    let watch_interval = matches.value_of("watch").map(|value| {
        let parsed_value = value.parse::<u64>().expect("interval must be an integer");
        Duration::from_secs(parsed_value)
    }).unwrap();

    match watcher::watch(applications_file, &sozu_config.command_socket, watch_interval) {
        Ok(_) => {
            info!("Exiting sozuconfw");
        }
        Err(err) => {
            error!("{}", err);
        }
    };
}
