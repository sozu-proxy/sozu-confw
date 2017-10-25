use futures::Future;
use futures::future::err;
use tokio_core::reactor::Core;
use sozu_command::state::ConfigState;
use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};

use std::time::{Duration, Instant};
use std::sync::mpsc::{channel, TryRecvError};

use util::errors::*;
use parser::parse_config_file;
use rpc::{get_config_state, execute_orders};

pub fn watch(application_file: &str, socket_path: &str, watch_interval: Duration, refresh_interval: Duration) -> Result<()> {
    let (tx, rx) = channel();

    info!("Watching file `{}`. Updating every {} second(s).", application_file, watch_interval.as_secs());
    let mut watcher: RecommendedWatcher = Watcher::new(tx, watch_interval)?;
    watcher.watch(application_file, RecursiveMode::NonRecursive)?;

    let mut core = Core::new()?;
    let handle = core.handle();

    info!("Retrieving current proxy state.");
    let config_state_future = get_config_state(socket_path, &handle)?;
    let mut current_state: ConfigState = core.run(config_state_future)?;
    info!("Current state initialized. Waiting for changes...");

    let mut last_sync = Instant::now();
    loop {
        if last_sync.elapsed().ge(&refresh_interval) {
            info!("Refreshing config state.");
            let execution_future = get_config_state(socket_path, &handle)?;
            current_state = core.run(execution_future)?;
            last_sync = Instant::now();
            info!("Config state refreshed.");
        }

        match rx.try_recv() {
            Ok(event) => {
                match event {
                    DebouncedEvent::Write(path) | DebouncedEvent::Create(path) | DebouncedEvent::Chmod(path) => {
                        info!("File written, generating diff.");

                        match parse_config_file(&path) {
                            Ok(new_state) => {
                                let orders = current_state.diff(&new_state);

                                if !orders.is_empty() {
                                    info!("Sending new configuration to server.");

                                    let execution_future = execute_orders(socket_path, &handle, &orders)?
                                        .map(|_| new_state)
                                        .or_else(|_| {
                                            info!("Error sending orders to proxy. Resynchronizing state.");
                                            get_config_state(socket_path, &handle).unwrap_or_else(|e| Box::new(err(e)))
                                        });

                                    current_state = core.run(execution_future)?;
                                    last_sync = Instant::now();
                                } else {
                                    warn!("No changes made.");
                                }
                            }
                            Err(e) => {
                                error!("Error reading file. Reason: {}", e);
                                continue;
                            }
                        }
                    }
                    DebouncedEvent::Rename(old_path, new_path) => {
                        // Track changed filename
                        info!("File renamed:\n\tOld path: {}\n\tNew path: {}.",
                              old_path.to_str().ok_or_else(|| ErrorKind::InvalidPath(old_path.clone()))?,
                              new_path.to_str().ok_or_else(|| ErrorKind::InvalidPath(new_path.clone()))?
                        );

                        watcher.unwatch(old_path)?;
                        watcher.watch(new_path, RecursiveMode::NonRecursive)?;
                    }
                    event => {
                        debug!("Unhandled event: {:?}.", event);
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(e) => {
                error!("Cannot poll file.");
                return Err(e.into());
            }
        }
    }
}