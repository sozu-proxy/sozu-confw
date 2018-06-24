use sozu_command::messages::Order;

use std::path::PathBuf;

#[derive(Debug, Fail)]
pub enum OperationError {
    #[fail(display = "path provided was not valid: {:?}", _0)]
    InvalidPath(PathBuf),
    #[fail(display = "could not load file: {:?}", _0)]
    FileLoad(PathBuf),
}

#[derive(Debug, Fail)]
pub enum ParseError {
    #[fail(display = "missing required item: {}", _0)]
    MissingItem(String)
}

#[derive(Debug, Fail)]
pub enum RpcError {
    #[fail(display = "message wasn't properly formed: {}", _0)]
    MalformedMessage(String),
    #[fail(display = "failed to execute order: {}", _0)]
    ExecutionFailure(String),
    #[fail(display = "unknown order: {:?}", _0)]
    UnsupportedOrder(Order),
}