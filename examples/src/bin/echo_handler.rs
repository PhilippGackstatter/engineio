use async_std::io;
use async_std::prelude::*;
use async_trait::async_trait;
use engineio::{Client, EventHandler, PacketData};

#[async_std::main]
async fn main() -> std::io::Result<()> {
    log::set_max_level(log::LevelFilter::Info);
    simple_logger::init().unwrap();

    let url_str = "http://localhost:8080/engine.io/";
    let handler = Handler {};
    let mut client = Client::connect(url_str, handler).await.unwrap();

    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin);
    let mut lines = reader.lines();

    println!("Type something...");

    while let Some(line) = lines.next().await {
        let line = line.unwrap();
        client.emit_str(line).await;
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
