use serde_json;
use futures::future;
use futures::IntoFuture;
use tokio_uds::UnixStream;
use rand::{thread_rng, Rng};
use futures::future::Future;
use command::SozuCommandClient;
use tokio_core::reactor::Handle;
use sozu_command::messages::Order;
use sozu_command::state::ConfigState;
use sozu_command::data::{ConfigCommand, ConfigMessage, ConfigMessageStatus};

use util::errors;

fn generate_id() -> String {
    let s: String = thread_rng().gen_ascii_chars().take(6).collect();
    format!("ID-{}", s)
}

pub fn execute_orders(socket_path: &str, handle: &Handle, orders: &[Order]) -> Box<Future<Item=Vec<()>, Error=errors::Error>> {
    let stream = UnixStream::connect(socket_path, handle).unwrap();
    let mut client = SozuCommandClient::new(stream);

    let mut message_futures: Vec<Box<Future<Item=(), Error=errors::Error>>> = Vec::new();
    for order in orders {
        let id = generate_id();
        let message = ConfigMessage::new(
            id.clone(),
            ConfigCommand::ProxyConfiguration(order.clone()),
            None
        );

        let order = order.clone();
        let future = client.send(message)
            .map_err(|e| {
                let new_error: errors::Error = e.into();
                new_error
            })
            .and_then(move |response| {
                if id != response.id {
                    error!("Received message with invalid id: {:?}.", response);
                    return Err(errors::ErrorKind::ErrorProxyResponse("".to_string()).into());
                }

                match response.status {
                    ConfigMessageStatus::Processing => {
                        // do nothing here
                        // for other messages, we would loop over read_message
                        // until an error or ok message was sent
                        Ok(())
                    }
                    ConfigMessageStatus::Error => {
                        error!("Could not execute order: {}", response.message);
                        Err(errors::ErrorKind::ErrorProxyResponse("".to_string()).into())
                    }
                    ConfigMessageStatus::Ok => {
                        let (item, action) = match order {
                            Order::AddInstance(_) => ("Backend", "added"),
                            Order::RemoveInstance(_) => ("Backend", "removed"),
                            Order::AddCertificate(_) => ("Certificate", "added"),
                            Order::RemoveCertificate(_) => ("Certificate", "removed"),
                            Order::AddHttpFront(_) => ("HTTP front", "added"),
                            Order::RemoveHttpFront(_) => ("HTTP front", "removed"),
                            Order::AddHttpsFront(_) => ("HTTPS front", "added"),
                            Order::RemoveHttpsFront(_) => ("HTTPS front", "removed"),
                            order => {
                                warn!("Unsupported order: {:?}", order);
                                return Err(errors::ErrorKind::ErrorProxyResponse("".to_owned()).into());
                            }
                        };

                        info!("{} {}: {}.", item, action, response.message);
                        Ok(())
                    }
                }
            })
            .into_future();

        message_futures.push(Box::new(future));
    }

    let future = future::join_all(message_futures).into_future();

    Box::new(future)
}

pub fn get_config_state(socket_path: &str, handle: &Handle) -> Box<Future<Item=ConfigState, Error=errors::Error>> {
    let stream = UnixStream::connect(socket_path, handle).unwrap();
    let mut client = SozuCommandClient::new(stream);

    let message = ConfigMessage::new(
        generate_id(),
        ConfigCommand::DumpState,
        None
    );

    let future = client.send(message)
        .map_err(|e| {
            let new_error: errors::Error = e.into();
            new_error
        })
        .and_then(|answer| {
            let config_state: Result<ConfigState, errors::Error> = serde_json::from_str(&answer.message)
                .map(|config_state: ConfigStateResponse| config_state.state)
                .map_err(|e| {
                    let new_error: errors::Error = e.into();
                    new_error
                });

            config_state
        })
        .into_future();

    Box::new(future)
}

#[derive(Debug, Default, Clone, Deserialize)]
struct ConfigStateResponse<'a> {
    id: &'a str,
    state: ConfigState
}