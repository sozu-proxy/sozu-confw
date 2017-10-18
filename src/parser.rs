use toml;
use sozu_command::config::Config;
use sozu_command::state::ConfigState;
use sozu_command::certificate::{split_certificate_chain, calculate_fingerprint};
use sozu_command::messages::{HttpFront, HttpsFront, Instance, CertFingerprint, CertificateAndKey, Order};

use std::path::PathBuf;
use std::collections::{HashMap, HashSet};

use util::errors::*;

pub fn parse_config_file(path: &PathBuf) -> Result<ConfigState> {
    let path = path.to_str().ok_or(ErrorKind::InvalidPath(path.to_path_buf()))?;
    let data = Config::load_file(path)?;

    parse_config(&data)
}

fn parse_config(data: &str) -> Result<ConfigState> {
    let mut state = ConfigState::new();

    let app_map: HashMap<String, Vec<RoutingConfig>> = toml::from_str(data)?;

    for (app_id, routing_configs) in app_map {
        for routing_config in routing_configs {
            let hostname = &routing_config.hostname.to_owned();
            let path_begin = &routing_config.path_begin.unwrap_or("/").to_owned();
            let sticky_session = routing_config.sticky_session.unwrap_or(false);

            let authorities = routing_config.backends.iter().map(|authority| {
                let mut split = authority.split(':');

                return match (split.next(), split.next()) {
                    (Some(host), Some(port)) => {
                        port.parse::<u16>().map(|port| (host.to_owned(), port))
                            .chain_err(|| ErrorKind::ParseError("Could not parse port".to_owned()))
                    },
                    (Some(host), None) => Ok((host.to_owned(), 80)),
                    _ => Err(ErrorKind::ParseError("Missing host".to_owned()).into())
                }
            }).collect::<Result<Vec<(String, u16)>>>()?;

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
                let certificate = routing_config.certificate
                    .ok_or(ErrorKind::MissingItem("Certificate".to_string()).into())
                    .and_then(|path| Config::load_file(path).chain_err(|| ErrorKind::FileLoad(path.to_string())))?;

                let key = routing_config.key
                    .ok_or(ErrorKind::MissingItem("Key".to_string()).into())
                    .and_then(|path| Config::load_file(path).chain_err(|| ErrorKind::FileLoad(path.to_string())))?;

                let certificate_chain = routing_config.certificate_chain
                    .ok_or(ErrorKind::MissingItem("Certificate Chain".to_string()).into())
                    .and_then(|path| Config::load_file(path).chain_err(|| ErrorKind::FileLoad(path.to_string())))
                    .map(|chain| split_certificate_chain(chain))
                    .unwrap_or_default();

                let certificate_and_key = CertificateAndKey {
                    certificate,
                    key,
                    certificate_chain
                };

                let fingerprint: CertFingerprint;
                {
                    let bytes = calculate_fingerprint(&certificate_and_key.certificate.as_bytes()[..])?;
                    fingerprint = CertFingerprint(bytes);
                }

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