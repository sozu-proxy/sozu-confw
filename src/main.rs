#[macro_use]
extern crate clap;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate error_chain;

extern crate notify;
extern crate toml;
extern crate serde;
extern crate ansi_term;
extern crate rand;
extern crate mio_uds;
extern crate sozu_command_lib as sozu_command;

mod watcher;
mod error;

use clap::{App, Arg};

use std::time::Duration;

fn main() {
    let matches = App::new("sozuconfw")
        .version(crate_version!())
        .about("Watch sozu app routing configs for updates")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("What config file to watch")
            .default_value("applications.toml")
            .takes_value(true)
            .required(false)
        )
        .arg(Arg::with_name("socket")
            .short("s")
            .long("socket")
            .value_name("SOCKET_PATH")
            .help("What socket sozu is listening on")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("interval")
            .short("i")
            .long("interval")
            .value_name("SECONDS")
            .help("How often to check for file changes")
            .default_value("5")
            .takes_value(true)
            .required(false)
        )
        .get_matches();

    let config_file = matches.value_of("config").unwrap();
    let socket_path = matches.value_of("socket").unwrap();
    let update_interval = matches.value_of("interval").map(|value| {
        let parsed_value = value.parse::<u64>().expect("interval must be an integer");
        Duration::from_secs(parsed_value)
    }).unwrap();

    println!("Watching file `{}`. Updating every {} second(s).", config_file, update_interval.as_secs());

    watcher::watch(config_file, socket_path, update_interval).unwrap();
}
