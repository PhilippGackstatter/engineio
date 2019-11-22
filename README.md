# EngineIO Rust Client

An [engineio](https://github.com/socketio/engine.io) client in Rust with `async/await` support (in development).

EngineIO is usually not used directly, but through the higher-level abstraction of `socketio` (not yet developed).

## Example

For the full example see [here](examples/src/bin/echo.rs).

To build a simple echo server, we use the `Client` to connect to an `engineio` server.

```rust
#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let eio_handler = EngineIOHandler {};
    let mut client = Client::connect("http://localhost:8080/engine.io/", eio_handler).await?;

    Ok(())
}
```

The `EngineIOHandler` is a user-defined type that implements `EventHandler`. It defines methods that are called when the client receives certain events such as `connect`, `disconnect` or `message`. Messages can be of type `String` or bytes in the form of `Vec<u8>`.

```rust
struct EngineIOHandler {}

#[async_trait]
impl EventHandler for EngineIOHandler {
    async fn on_connect(&mut self) {
        println!("connect");
    }

    async fn on_disconnect(&mut self) {
        println!("disconnect");
    }

    async fn on_message(&mut self, data: PacketData) {
        match data {
            PacketData::Str(str_) => {
                println!("{}", str_);
            }
            PacketData::Bytes(bytes) => {
                println!("{:?}", bytes);
            }
        }
    }
}
```

Next we define an `emit_loop` that reads user input from the terminal and sends it to the EngineIO server. The interesting part here is that we pass a `Sender` to the method, which can be used to `emit` messages via the client, that are sent to the server. We can create one of those senders, by simply calling `client.sender()`. You can create as many of those senders as you need.

```rust
async fn emit_loop(mut sender: Sender) -> Result<(), Box<dyn Error>> {
    let lines = ... /* Create a stream from stdin */

    while let Some(line) = lines.next().await {
        let line = line?;
        sender.emit_str(line).await;
    }
    Ok(())
}
```

Now we only need to call this loop to get going. But what if, for example, the server goes down and we need to exit from the `emit_loop` function? Luckily, Rust's futures are lazy and we can use that to our advantage. To handle this gracefully, we'll extend the main method as follows.

```rust
try_join!(client.join(), emit_loop(sender))?;
```

`try_join!` allows us to poll multiple futures concurrently. As soon as one returns an error, it stops polling and returns. The `join` method on `Client` returns a `Result`. For instance, if it loses connection to the server, it returns an error. This in turn causes `try_join!` to stop polling all of its futures. We thereby implicitly stop polling `emit_loop`, which would otherwise ask for user input forever.

With that we have setup a client connection to an `engineio` server, defined an event handler to react to server-sent events and emitted any input from the user. Finally, we handled a graceful shutdown of the application in case of an error.

See the full example [here](examples/src/bin/echo.rs).

Run it from the `examples` directory using `cargo run --bin echo`.

In the absence of a Rust implementation of an `engineio` server, we'll use the JavaScript `engineio` implementation to send back any received data.

```js
// file engineio.js
var engine = require("engine.io");
var server = engine.listen(8080);

server.on("connection", function(socket) {
  socket.on("message", function(data) {
    console.log(data);
    socket.send(data);
  });
});
```

You can start that up with `npm install engine.io` and a subsequent `node engineio.js`.
