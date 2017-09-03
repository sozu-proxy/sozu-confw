use mio_uds::UnixStream;
use sozu_command::channel::Channel;
use sozu_command::data::{ConfigMessage, ConfigMessageAnswer};
use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};

use std::time::Duration;
use std::sync::mpsc::channel;

use error::errors;
use util::ConsoleMessage;
use parser::parse_config_file;
use rpc::{initialize_config_state, order_command};

pub fn watch(config_file: &str, socket_path: &str, update_interval: Duration) -> errors::Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, update_interval)?;
    watcher.watch(config_file, RecursiveMode::NonRecursive)?;

    {
        let message = format!("Watching file `{}`. Updating every {} second(s).", config_file, update_interval.as_secs());
        ConsoleMessage::Info(&message).println();
    }

    let stream = UnixStream::connect(socket_path).expect("Could not connect to the command unix socket.");
    let mut channel: Channel<ConfigMessage, ConfigMessageAnswer> = Channel::new(stream, 10000, 20000);
    channel.set_nonblocking(false);

    ConsoleMessage::Info("Retrieving current proxy state").println();
    let mut current_state = initialize_config_state(&mut channel).unwrap();
    ConsoleMessage::Success("Current state initialized. Waiting for changes...").println();

    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    DebouncedEvent::Write(path) | DebouncedEvent::Create(path) | DebouncedEvent::Chmod(path) => {
                        ConsoleMessage::Info("File written, generating diff.").println();

                        match parse_config_file(&path) {
                            Ok(new_state) => {
                                let orders = current_state.diff(&new_state);

                                if orders.len() > 0 {
                                    ConsoleMessage::Info("Sending new configuration to server.").println();
                                }

                                for order in orders {
                                    order_command(&mut channel, order);
                                }

                                current_state = new_state;
                            }
                            Err(_) => {
                                ConsoleMessage::Error("Error reading file.").println();
                                continue;
                            }
                        }
                    }
                    DebouncedEvent::Rename(old_path, new_path) => {
                        // Track changed filename
                        let message = format!("File renamed:\n\tOld path: {}\n\tNew path: {}",
                                              old_path.to_str().expect("missing old path"),
                                              new_path.to_str().expect("missing new path")
                        );
                        ConsoleMessage::Info(&message).println();

                        watcher.unwatch(old_path)?;
                        watcher.watch(new_path, RecursiveMode::NonRecursive)?;
                    }
                    event => {
                        let message = format!("Unhandled event: {:?}", event);
                        ConsoleMessage::Error(&message).println();
                    }
                }
            }
            Err(e) => {
                ConsoleMessage::Fatal("Cannot poll file").println();
                return Err(e.into());
            }
        }
    }
}