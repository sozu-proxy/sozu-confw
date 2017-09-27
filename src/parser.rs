use toml;
use sozu_command::config::Config;
use sozu_command::state::ConfigState;
use sozu_command::certificate::{split_certificate_chain, calculate_fingerprint};
use sozu_command::messages::{HttpFront, HttpsFront, Instance, CertFingerprint, CertificateAndKey, Order};

use std::path::PathBuf;
use std::collections::{HashMap, HashSet};

use util::errors;

pub fn parse_config_file(path: &PathBuf) -> errors::Result<ConfigState> {
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
                    sticky_session
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
                    certificate,
                    key,
                    certificate_chain
                };

                let fingerprint = calculate_fingerprint(&certificate_and_key.certificate.as_bytes()[..])
                    .map(|it| CertFingerprint(it))?;

                let add_certificate = &Order::AddCertificate(certificate_and_key);
                let add_https_front = &Order::AddHttpsFront(HttpsFront {
                    app_id: app_id.clone(),
                    hostname: hostname.clone(),
                    path_begin: path_begin.clone(),
                    fingerprint,
                    sticky_session
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
                        port
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