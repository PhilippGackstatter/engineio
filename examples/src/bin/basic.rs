use async_trait::async_trait;
use engineio::{Client, EventHandler, PacketData};

#[async_std::main]
async fn main() {
    env_logger::init();

    let url_str = "http://localhost:8080/engine.io/";
    let handler = Handler {};
    let mut client = Client::connect(url_str, handler).await.unwrap();

    client.join().await.unwrap();
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
