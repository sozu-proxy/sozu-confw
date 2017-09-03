# sozu-confw
Watch [sozu](https://github.com/sozu-proxy/sozu) app routing configs and automatically update the reverse proxy to reflect changes. 

## Usage
```
sozuconfw [OPTIONS] --socket <SOCKET_PATH>
```

Here is an overview and brief description of the options made available:
```
-c, --config <FILE>           What config file to watch [default: applications.toml]
-i, --interval <SECONDS>      How often to check for file changes [default: 5]
-s, --socket <SOCKET_PATH>    What socket sozu is listening on
```

 