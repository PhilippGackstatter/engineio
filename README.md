# EngineIO Client Rust

An [engineio](https://github.com/socketio/engine.io) client in Rust with `async/await` support (in development).

EngineIO is usually not used directly, but used through the higher-level abstraction of `socketio`, which is not yet developed.

## Example

For the full example see [here](examples/src/bin/echo.rs).

To build a simple echo server, we first use the ClientBuilder to connect to an `engineio` server.

```rust
let mut client = ClientBuilder::new()
    .connect_handler(connect)
    .disconnect_handler(disconnect)
    .message_handler(message)
    .build("http://localhost:8080/engine.io/")
    .await?;
```

We need to define some async event handlers in order to receive messages from the server.

```rust
async fn connect(_data: PacketData) {
    println!("connect");
}

async fn disconnect(_data: PacketData) {
    println!("disconnect");
}

async fn message(data: PacketData) {
    match data {
        PacketData::Str(str_) => {
            println!("{}", str_);
        }
        PacketData::Bytes(bytes) => {
            println!("{:?}", bytes);
        }
    }
}
```

Now we can send data the user types into the terminal to the server.

```rust
let stdin = io::stdin();
let reader = io::BufReader::new(stdin);
let mut lines = reader.lines();

println!("Type something...");

while let Some(line) = lines.next().await {
    client.emit_str(line?).await;
}
```
