use engineio::ClientBuilder;

fn main() {
    async_std::task::block_on(async {
        let url_str = "http://localhost:8080/engine.io/";
        let mut client = ClientBuilder::new()
            .connect_handler(connect)
            .disconnect_handler(disconnect)
            .message_handler(message)
            .build(url_str)
            .await
            .unwrap();
        client.serve().await;
    });
}

async fn connect() {
    println!("connect");
}

async fn disconnect() {
    println!("disconnect");
}

async fn message() {
    println!("msg");
}
