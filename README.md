# Simple WS chat

## Compile and run

```rust
cargo run
```

## Attach client
Install `websocat`:
```sh
cargo install websocat
```

and then:

```sh
websocat ws://127.0.0.1:8080
```

## Configuration
### Enable logging
```sh
cat .env << EOF
RUST_LOG=info
EOF
```

### Listener address
Add `SOCKET_ADDR` env to your `.env` file:

```sh
SOCKET_ADDR=<host>:<port>
```

for example:
```sh
SOCKET_ADDR=127.0.0.1:9090
```

