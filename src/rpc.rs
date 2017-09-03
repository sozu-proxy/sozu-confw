use serde_json;
use rand::{thread_rng, Rng};
use sozu_command::messages::Order;
use sozu_command::channel::Channel;
use sozu_command::state::ConfigState;
use sozu_command::data::{ConfigCommand, ConfigMessage, ConfigMessageAnswer, ConfigMessageStatus};

use error::errors;

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