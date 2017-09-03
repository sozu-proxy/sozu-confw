pub mod errors {
    use notify;
    use std::io;
    use toml::de;
    use serde_json;
    use std::sync::mpsc;

    error_chain! {
        foreign_links {
            Io(io::Error);
            Toml(de::Error);
            Notify(notify::Error);
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