use serde_json;
use rand::{thread_rng, Rng};
use sozu_command::messages::Order;
use sozu_command::channel::Channel;
use sozu_command::state::ConfigState;
use sozu_command::data::{ConfigCommand, ConfigMessage, ConfigMessageAnswer, ConfigMessageStatus};

use error::errors;
use util::ConsoleMessage;

fn generate_id() -> String {
    let s: String = thread_rng().gen_ascii_chars().take(6).collect();
    format!("ID-{}", s)
}

pub fn order_command(channel: &mut Channel<ConfigMessage, ConfigMessageAnswer>, order: Order) {
    let id = generate_id();
    channel.write_message(&ConfigMessage::new(
        id.clone(),
        ConfigCommand::ProxyConfiguration(order.clone()),
        None,
    ));

    match channel.read_message() {
        None => {
            let print = format!("No response from the proxy");
            ConsoleMessage::Error(&print).println();
        }
        Some(response) => {
            if id != response.id {
                let print = format!("Received message with invalid id: {:?}", response);
                ConsoleMessage::Error(&print).println();
                return;
            }
            match response.status {
                ConfigMessageStatus::Processing => {
                    // do nothing here
                    // for other messages, we would loop over read_message
                    // until an error or ok message was sent
                }
                ConfigMessageStatus::Error => {
                    let print = format!("Could not execute order: {}", response.message);
                    ConsoleMessage::Error(&print).println();
                }
                ConfigMessageStatus::Ok => {
                    let print = match order {
                        Order::AddInstance(_) => format!("Backend added : {}", response.message),
                        Order::RemoveInstance(_) => format!("Backend removed : {} ", response.message),
                        Order::AddCertificate(_) => format!("Certificate added: {}", response.message),
                        Order::RemoveCertificate(_) => format!("Certificate removed: {}", response.message),
                        Order::AddHttpFront(_) => format!("Http front added: {}", response.message),
                        Order::RemoveHttpFront(_) => format!("Http front removed: {}", response.message),
                        Order::AddHttpsFront(_) => format!("Https front added: {}", response.message),
                        Order::RemoveHttpsFront(_) => format!("Https front removed: {}", response.message),
                        order => {
                            let message = format!("Unsupported order: {:?}", order);
                            ConsoleMessage::Warn(&message).println();
                            return;
                        }
                    };

                    ConsoleMessage::Success(&print).println();
                }
            }
        }
    }
}

pub fn initialize_config_state(channel: &mut Channel<ConfigMessage, ConfigMessageAnswer>) -> errors::Result<ConfigState> {
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

#[derive(Debug, Default, Clone, Deserialize)]
struct ConfigStateResponse<'a> {
    id: &'a str,
    state: ConfigState
}