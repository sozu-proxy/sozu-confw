use serde_json;
use failure::Error;
use tokio_uds::UnixStream;
use rand::{thread_rng, Rng};
use command::SozuCommandClient;
use tokio_core::reactor::Handle;

use futures::{
    future,
    IntoFuture,
    future::Future,
};

use sozu_command::{
    messages::Order,
    state::ConfigState,
    data::{
        ConfigCommand,
        ConfigMessage,
        ConfigMessageStatus,
    },
};

use util::RpcError;

fn generate_id() -> String {
    let s: String = thread_rng().gen_ascii_chars().take(6).collect();
    format!("ID-{}", s)
}

pub fn execute_orders(socket_path: &str, handle: &Handle, orders: &[Order]) -> Box<dyn Future<Item=Vec<()>, Error=Error>> {
    let stream = match UnixStream::connect(socket_path, handle) {
        Ok(stream) => stream,
        Err(e) => return Box::new(future::err(e.into()))
    };

    let mut client = SozuCommandClient::new(stream);

    let mut message_futures: Vec<Box<dyn Future<Item=(), Error=Error>>> = Vec::new();
    for order in orders {
        let id = generate_id();
        let message = ConfigMessage::new(
            id.clone(),
            ConfigCommand::ProxyConfiguration(order.clone()),
            None,
        );

        let order = order.clone();
        let future = client.send(message)
            .map_err(|e| {
                let new_error: Error = e.into();
                new_error
            })
            .and_then(move |response| {
                if id != response.id {
                    error!("Received message with invalid id: {:?}.", response);
                    return Err(RpcError::MalformedMessage("Invalid message ID".to_string()).into());
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
                        Err(RpcError::ExecutionFailure(response.message).into())
                    }
                    ConfigMessageStatus::Ok => {
                        let (item, action) = match order {
                            Order::AddApplication(_) => ("Application", "added"),
                            Order::RemoveApplication(_) => ("Application", "removed"),
                            Order::AddCertificate(_) => ("Certificate", "added"),
                            Order::RemoveCertificate(_) => ("Certificate", "removed"),
                            Order::ReplaceCertificate(_) => ("Certificate", "replaced"),
                            Order::AddHttpFront(_) => ("HTTP front", "added"),
                            Order::RemoveHttpFront(_) => ("HTTP front", "removed"),
                            Order::AddHttpsFront(_) => ("HTTPS front", "added"),
                            Order::RemoveHttpsFront(_) => ("HTTPS front", "removed"),
                            order => {
                                warn!("Unsupported order: {:?}", order);
                                return Err(RpcError::UnsupportedOrder(order).into());
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

    Box::new(future::join_all(message_futures).into_future())
}

pub fn get_config_state(socket_path: &str, handle: &Handle) -> Box<dyn Future<Item=ConfigState, Error=Error>> {
    let stream = match UnixStream::connect(socket_path, handle) {
        Ok(stream) => stream,
        Err(e) => return Box::new(future::err(e.into()))
    };

    let mut client = SozuCommandClient::new(stream);

    let message = ConfigMessage::new(
        generate_id(),
        ConfigCommand::DumpState,
        None,
    );

    let future = client.send(message)
        .map_err(|e| {
            let new_error: Error = e.into();
            new_error
        })
        .and_then(|answer| {
            let config_state: Result<ConfigState, Error> = serde_json::from_str(&answer.message)
                .map(|config_state: ConfigStateResponse| config_state.state)
                .map_err(|e| {
                    let new_error: Error = e.into();
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
    state: ConfigState,
}