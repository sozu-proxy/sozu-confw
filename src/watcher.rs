use mio_uds::UnixStream;
use sozu_command::channel::Channel;
use sozu_command::data::{ConfigMessage, ConfigMessageAnswer};
use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};

use std::time::Duration;
use std::sync::mpsc::channel;

use error::errors;
use parser::parse_config_file;
use rpc::{initialize_config_state, order_command};

pub fn watch(config_file: &str, socket_path: &str, update_interval: Duration) -> errors::Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, update_interval)?;
    watcher.watch(config_file, RecursiveMode::NonRecursive)?;

    let stream = UnixStream::connect(socket_path).expect("Could not connect to the command unix socket.");
    let mut channel: Channel<ConfigMessage, ConfigMessageAnswer> = Channel::new(stream, 10000, 20000);
    channel.set_nonblocking(false);

    println!("Retrieving current proxy state");
    let mut current_state = initialize_config_state(&mut channel).unwrap();
    println!("Current state initialized. Waiting for changes...");

    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    DebouncedEvent::Write(path) | DebouncedEvent::Create(path) | DebouncedEvent::Chmod(path) => {
                        println!("File written, generating diff.");

                        match parse_config_file(&path) {
                            Ok(new_state) => {
                                println!("Creating diff");
                                let orders = current_state.diff(&new_state);

                                if orders.len() > 0 {
                                    println!("Sending new configuration to server.");
                                }

                                for order in orders {
                                    order_command(&mut channel, order);
                                }

                                current_state = new_state;
                            }
                            Err(e) => {
                                println!("Error reading file.");
                                continue;
                            }
                        }
                    }
                    DebouncedEvent::Rename(old_path, new_path) => {
                        // Track changed filename
                        println!("File renamed:\n\tOld path: {}\n\tNew path: {}",
                                 old_path.to_str().expect("missing old path"),
                                 new_path.to_str().expect("missing new path")
                        );
                        watcher.unwatch(old_path)?;
                        watcher.watch(new_path, RecursiveMode::NonRecursive)?;
                    }
                    event => {
                        // Error
                        println!("{:?}", event);
                    }
                }
            }
            Err(e) => {}
        }
    }
}