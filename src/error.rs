pub mod errors {
    use notify;
    use std::io;
    use toml::de;
    use std::sync::mpsc;

    error_chain! {
        foreign_links {
            Notify(notify::Error);
            Io(io::Error);
            Deserialize(de::Error);
            Channel(mpsc::RecvError);
        }
    }
}