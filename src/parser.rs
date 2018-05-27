use toml;
use sozu_command::{
    state::ConfigState,
    certificate::{
        calculate_fingerprint,
        split_certificate_chain
    },
    config::{
        Config,
        ProxyProtocolConfig
    },
    messages::{
        Application,
        AddCertificate,
        CertificateAndKey,
        CertFingerprint,
        HttpFront,
        HttpsFront,
        Order,
    },
};

use std::{
    path::PathBuf,
    collections::{HashMap, HashSet},
};

use util::errors::*;

pub fn parse_config_file(path: &PathBuf) -> Result<ConfigState> {
    let path = path.to_str().ok_or_else(|| ErrorKind::InvalidPath(path.to_path_buf()))?;
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

            {
                let sticky_session = routing_config.sticky_session.unwrap_or(false);
                let https_redirect = routing_config.https_redirect.unwrap_or(false);

                let add_instance = &Order::AddApplication(Application {
                    app_id: app_id.clone(),
                    proxy_protocol: routing_config.proxy_protocol,
                    sticky_session,
                    https_redirect,
                });

                state.handle_order(add_instance);
            }

            if routing_config.frontends.contains(&"HTTP") {
                let add_http_front = &Order::AddHttpFront(HttpFront {
                    app_id: app_id.clone(),
                    hostname: hostname.clone(),
                    path_begin: path_begin.clone(),
                });

                state.handle_order(add_http_front);
            }

            if routing_config.frontends.contains(&"HTTPS") {
                let certificate = routing_config.certificate
                    .ok_or_else(|| ErrorKind::MissingItem("Certificate".to_string()).into())
                    .and_then(|path| Config::load_file(path).chain_err(|| ErrorKind::FileLoad(path.to_string())))?;

                let key = routing_config.key
                    .ok_or_else(|| ErrorKind::MissingItem("Key".to_string()).into())
                    .and_then(|path| Config::load_file(path).chain_err(|| ErrorKind::FileLoad(path.to_string())))?;

                let certificate_chain = routing_config.certificate_chain
                    .ok_or_else(|| ErrorKind::MissingItem("Certificate Chain".to_string()).into())
                    .and_then(|path| Config::load_file(path).chain_err(|| ErrorKind::FileLoad(path.to_string())))
                    .map(split_certificate_chain)
                    .unwrap_or_default();

                let certificate_and_key = CertificateAndKey {
                    certificate,
                    key,
                    certificate_chain,
                };

                let fingerprint: CertFingerprint;
                {
                    let bytes = calculate_fingerprint(&certificate_and_key.certificate.as_bytes()[..])
                        .ok_or_else(|| ErrorKind::FingerprintError)?;
                    fingerprint = CertFingerprint(bytes);
                }

                let add_certificate = &Order::AddCertificate(AddCertificate {
                    certificate: certificate_and_key,
                    names: vec![hostname.clone()],
                });
                let add_https_front = &Order::AddHttpsFront(HttpsFront {
                    app_id: app_id.clone(),
                    hostname: hostname.clone(),
                    path_begin: path_begin.clone(),
                    fingerprint,
                });

                state.handle_order(add_certificate);
                state.handle_order(add_https_front);
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
    sticky_session: Option<bool>,
    https_redirect: Option<bool>,
    proxy_protocol: Option<ProxyProtocolConfig>,
}