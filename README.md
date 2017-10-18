# sozu-confw
Watch [sozu](https://github.com/sozu-proxy/sozu) app routing configs and automatically update the reverse proxy to reflect changes. 

## Usage
```
sozuconfw [OPTIONS]
```

Here is an overview and brief description of the options made available:
```
-c, --config <FILE>           What sozu config to read from [default: config.toml]
-a, --applications <FILE>     What application config file to watch [default: applications.toml]
-i, --interval <SECONDS>      How often to check for file changes [default: 5]
```

You can turn on logging by setting the `RUST_LOG` environment variable. More details can be found [here](https://doc.rust-lang.org/log/env_logger/index.html#enabling-logging).

## Config file

Your app routing config file defines what applications sozu will manage. The most basic example can be written as:

```
[[magus]]
hostname = "tohsa.ka"
frontends = ["HTTP"]
backends  = ["127.0.0.1:1021"]
```

Here we are defining an HTTP app named `magus` which has the host `tohsa.ka` and is routed towards `127.0.0.1:1021`.

If we wanted to also define this as an HTTPS app, we would need to include some more details:

```
[[magus]]
hostname = "tohsa.ka"
key = "assets/key.pem"
certificate = "assets/certificate.pem"
certificate_chain = "assets/certificate_chain.pem"
frontends = ["HTTP", "HTTPS"]
backends  = ["127.0.0.1:1021"]
```

Here we see three new parameters: `key`, `certificate`, and `certificate_chain`. These are the paths to the associated files and are *required* to create an HTTPS app.

Now, what if we wanted the same host, but pointing towards two sets of backends based on a path suffix?

This is as simple as defining a `path_begin`:

```
[[magus]]
hostname = "tohsa.ka"
path_begin = "/archer"
frontends = ["HTTP"]
backends  = ["127.0.0.1:1021"]

[[magus]]
hostname = "tohsa.ka"
path_begin = "/saber"
frontends = ["HTTP"]
backends  = ["127.0.0.1:1022"]
```

Additionally, you may also set sticky sessions with `sticky_session` like so:

```
[[magus]]
hostname = "tohsa.ka"
path_begin = "/archer"
frontends = ["HTTP"]
backends  = ["127.0.0.1:1021"]
sticky_session = false
```

If `sticky_session` is omitted, it is set to false.

A final note, this file may be named anything you want as long as you set `--applications`. Otherwise `sozuconf` will look for `applications.toml` in `$PWD`.