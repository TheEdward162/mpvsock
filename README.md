# mpvsock

mpv socket ipc client with MIT/Apache-2.0 license.

## cli_app

Try with (to spawn a child mpv instance):

```
cargo run --bin mpv-client -- --verbosity Trace --spawn-client interactive
```

or with (to connect to an existing `--input-ipc-server=/path/to/socket` instance):

```
cargo run --bin mpv-client -- --verbosity Trace --connect /path/to/socket interactive
```
