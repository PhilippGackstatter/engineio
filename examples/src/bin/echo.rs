use async_std::io;
use async_std::prelude::*;
use async_trait::async_trait;
use futures::try_join;

use engineio::{Client, EventHandler, PacketData, Sender};

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let url_str = "http://localhost:8080/engine.io/";
    let handler = Handler {};
    let mut client = Client::connect(url_str, handler).await.unwrap();
    let sender = client.sender();

    // Crude way to cast from EIOError to impl Error s.t. both inputs to
    // try_join! have the same Err
    // There's probably a better way...
    let client_join = async {
        match client.join().await {
            Ok(_) => Ok(()),
            Err(err) => Err(Box::new(err) as Box<dyn std::error::Error>),
        }
    };

    // This elegantly handles disconnects from a server
    // On a disconnect, client_join returns with an error
    // and try_join returns the error.
    // emit_loop is no longer polled,
    // thereby implicitly canceling it.
    try_join!(client_join, emit_loop(sender))?;

    Ok(())
}

async fn emit_loop(mut sender: Sender) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin);
    let mut lines = reader.lines();

    println!("Type something...");

    while let Some(line) = lines.next().await {
        let line = line.unwrap();
        sender.emit_str(line).await;
    }
    Ok(())
}

struct Handler {}

#[async_trait]
impl EventHandler for Handler {
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
