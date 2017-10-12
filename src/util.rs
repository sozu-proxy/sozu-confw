pub mod errors {
    use notify;
    use openssl;
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
            OpenSSL(openssl::error::ErrorStack);
        }

        errors {
            InvalidPath {
                description("path is invalid")
                display("Path is invalid.")
            }
            FileLoad {
                description("could not load file")
                display("File could not be loaded")
            }
            MissingItem(item: String) {
                description("missing required item")
                display("Item `{}` required, but not present.", item)
            }
            NoProxyResponse(action: String) {
                description("no response from the proxy")
                display("No response from the proxy while attempting '{}'.", action)
            }
            ErrorProxyResponse(e: String) {
                description("no response from the proxy")
                display("Proxy responded with an error: {}.", e)
            }
        }
    }
}