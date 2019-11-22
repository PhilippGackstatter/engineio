use async_std::io;
use async_std::prelude::*;
use async_trait::async_trait;
use futures::try_join;

use engineio::{Client, EventHandler, PacketData, Sender};
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let eio_handler = EngineIOHandler {};
    let mut client = Client::connect("http://localhost:8080/engine.io/", eio_handler)
        .await
        .unwrap();
    let sender = client.sender();

    // This elegantly handles disconnects from a server
    // On a disconnect, client_join returns with an error
    // and try_join returns the error.
    // emit_loop is no longer polled,
    // thereby implicitly canceling it.
    try_join!(client.join(), emit_loop(sender))?;

    Ok(())
}

async fn emit_loop(mut sender: Sender) -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin);
    let mut lines = reader.lines();

    println!("Type something...");

    while let Some(line) = lines.next().await {
        let line = line?;
        sender.emit_str(line).await;
    }
    Ok(())
}

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
