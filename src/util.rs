use ansi_term::Colour::{White, Green, Red, Yellow};

use std::fmt;

pub enum ConsoleMessage<'a> {
    Info(&'a str),
    Success(&'a str),
    Warn(&'a str),
    Error(&'a str),
    Fatal(&'a str)
}

impl<'a> ConsoleMessage<'a> {
    pub fn println(&self) {
        println!("{}", self)
    }
}

impl<'a> fmt::Display for ConsoleMessage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            ConsoleMessage::Info(message) => White.paint(message),
            ConsoleMessage::Success(message) => Green.paint(message),
            ConsoleMessage::Warn(message) => Yellow.italic().paint(message),
            ConsoleMessage::Error(message) => Red.paint(message),
            ConsoleMessage::Fatal(message) => Red.bold().paint(message),
        };

        message.fmt(f)
    }
}

