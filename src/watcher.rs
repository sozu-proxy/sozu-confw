use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};
use toml;
use sozu_command::certificate::{split_certificate_chain, calculate_fingerprint};
use sozu_command::data::{ConfigCommand, ConfigMessage, ConfigMessageAnswer, ConfigMessageStatus};
use sozu_command::messages::{HttpFront, HttpsFront, Instance, CertFingerprint, CertificateAndKey, Order};
use sozu_command::state::ConfigState;
use sozu_command::config::Config;
use mio_uds::UnixStream;
use sozu_command::channel::Channel;
use rand::{thread_rng, Rng};
use serde_json;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::PathBuf;
use std::str::FromStr;

use error::errors;

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

fn parse_config_file(path: &PathBuf) -> errors::Result<ConfigState> {
    let path = path.to_str().expect("Could not convert path to str");
    let data = Config::load_file(path)?;

    parse_config(&data)
}

fn parse_config(data: &str) -> errors::Result<ConfigState> {
    let mut state = ConfigState::new();

    let app_map: HashMap<String, Vec<RoutingConfig>> = toml::from_str(data)?;

    for (app_id, routing_configs) in app_map {
        for routing_config in routing_configs {
            let hostname = &routing_config.hostname.to_owned();
            let path_begin = &routing_config.path_begin.unwrap_or("/").to_owned();
            let sticky_session = routing_config.sticky_session.unwrap_or(false);

            let authorities: Vec<(String, u16)> = routing_config.backends.iter().map(|authority| {
                let mut split = authority.split(":");

                let host = split.next().expect("host is required").to_owned();
                let port = split.next().unwrap_or("80").parse::<u16>().expect("could not parse port");

                (host, port)
            }).collect();

            if routing_config.frontends.contains(&"HTTP") {
                let add_http_front = &Order::AddHttpFront(HttpFront {
                    app_id: app_id.clone(),
                    hostname: hostname.clone(),
                    path_begin: path_begin.clone(),
                    sticky_session: sticky_session
                });

                state.handle_order(add_http_front);
            }

            if routing_config.frontends.contains(&"HTTPS") {
                let certificate = routing_config.certificate.map(|path| {
                    let certificate = Config::load_file(path).expect("could not load certificate");
                    certificate
                }).expect("HTTPS requires a certificate");

                let key = routing_config.key.map(|path| {
                    let key: String = Config::load_file(path).expect("could not load key");
                    key
                }).expect("HTTPS requires a key");

                let certificate_chain = routing_config.certificate_chain.map(|path| {
                    let chain = Config::load_file(&path).expect("could not load certificate chain");

                    split_certificate_chain(chain)
                }).unwrap_or(Vec::new());

                let certificate_and_key = CertificateAndKey {
                    certificate: certificate,
                    key: key,
                    certificate_chain: certificate_chain
                };

                let fingerprint = calculate_fingerprint(&certificate_and_key.certificate.as_bytes()[..])
                    .map(|it| CertFingerprint(it))
                    .expect("could not calculate fingerprint");

                let add_certificate = &Order::AddCertificate(certificate_and_key);
                let add_https_front = &Order::AddHttpsFront(HttpsFront {
                    app_id: app_id.clone(),
                    hostname: hostname.clone(),
                    path_begin: path_begin.clone(),
                    fingerprint: fingerprint,
                    sticky_session: sticky_session
                });

                state.handle_order(add_certificate);
                state.handle_order(add_https_front);
            }

            {
                let add_instances: Vec<Order> = authorities.iter().map(|authority| {
                    let (ref host, port): (String, u16) = *authority;

                    Order::AddInstance(Instance {
                        app_id: app_id.clone(),
                        ip_address: host.clone(),
                        port: port
                    })
                }).collect();

                for order in add_instances {
                    state.handle_order(&order);
                }
            }
        }
    }

    Ok(state)
}

fn order_command(channel: &mut Channel<ConfigMessage, ConfigMessageAnswer>, order: Order) {
    let id = generate_id();
    channel.write_message(&ConfigMessage::new(
        id.clone(),
        ConfigCommand::ProxyConfiguration(order.clone()),
        None,
    ));

    match channel.read_message() {
        None => println!("the proxy didn't answer"),
        Some(message) => {
            if id != message.id {
                println!("received message with invalid id: {:?}", message);
                return;
            }
            match message.status {
                ConfigMessageStatus::Processing => {
                    // do nothing here
                    // for other messages, we would loop over read_message
                    // until an error or ok message was sent
                }
                ConfigMessageStatus::Error => {
                    println!("Could not execute order: {}", message.message);
                }
                ConfigMessageStatus::Ok => {
                    match order {
                        Order::AddInstance(_) => println!("Backend added : {}", message.message),
                        Order::RemoveInstance(_) => println!("Backend removed : {} ", message.message),
                        Order::AddCertificate(_) => println!("Certificate added: {}", message.message),
                        Order::RemoveCertificate(_) => println!("Certificate removed: {}", message.message),
                        Order::AddHttpFront(_) => println!("Http front added: {}", message.message),
                        Order::RemoveHttpFront(_) => println!("Http front removed: {}", message.message),
                        Order::AddHttpsFront(_) => println!("Https front added: {}", message.message),
                        Order::RemoveHttpsFront(_) => println!("Https front removed: {}", message.message),
                        _ => {
                            // do nothing for now
                        }
                    }
                }
            }
        }
    }
}

fn initialize_config_state(channel: &mut Channel<ConfigMessage, ConfigMessageAnswer>) -> errors::Result<ConfigState> {
    let id = generate_id();
    channel.write_message(&ConfigMessage::new(
        id.clone(),
        ConfigCommand::DumpState,
        None
    ));

    return match channel.read_message() {
        None => Err(errors::ErrorKind::NoResponse("initialize".to_owned()).into()),
        Some(answer) => {
            let response: ConfigStateResponse = serde_json::from_str(&answer.message)?;
            Ok(response.state)
        }
    };
}

fn generate_id() -> String {
    let s: String = thread_rng().gen_ascii_chars().take(6).collect();
    format!("ID-{}", s)
}

#[derive(Debug, Default, Clone, Deserialize)]
struct RoutingConfig<'a> {
    hostname: &'a str,
    path_begin: Option<&'a str>,
    certificate: Option<&'a str>,
    key: Option<&'a str>,
    certificate_chain: Option<&'a str>,
    frontends: HashSet<&'a str>,
    backends: Vec<&'a str>,
    sticky_session: Option<bool>
}

#[derive(Debug, Default, Clone, Deserialize)]
struct ConfigStateResponse<'a> {
    id: &'a str,
    state: ConfigState
}