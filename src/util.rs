pub mod errors {
    use notify;
    use openssl;
    use std::io;
    use toml::de;
    use serde_json;

    use std::sync::mpsc;
    use std::path::PathBuf;

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
            InvalidPath(path: PathBuf) {
                description("path is invalid")
                display("Path '{:?}' is invalid.", path)
            }
            FileLoad(filename: String) {
                description("could not load file")
                display("File '{}' could not be loaded.", filename)
            }
            ParseError(issue: String) {
                description("encountered error while parsing")
                display("Parse error: {}.", issue)
            }
            MissingItem(item: String) {
                description("missing required item")
                display("Item `{}` required, but not present.", item)
            }
            ProxyError(error: String) {
                description("the proxy encountered an error")
                display("Proxy responded with an error: {}.", error)
            }
            FingerprintError {
                description("could not calculate fingerprint from cert")
                display("Unable to calculate a fingerprint for the provided certificate.")
            }
        }
    }
}