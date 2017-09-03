pub mod errors {
    use notify;
    use std::io;
    use toml::de;
    use std::sync::mpsc;
    use serde_json;

    error_chain! {
        foreign_links {
            Notify(notify::Error);
            Io(io::Error);
            Deserialize(de::Error);
            Json(serde_json::Error);
            Channel(mpsc::RecvError);
        }

        errors {
            NoResponse(action: String) {
                description("no response from the proxy")
                display("no response from the proxy while attempting '{}'", action)
            }
        }
    }
}