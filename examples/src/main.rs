use engineio::Client;

fn main() {
    async_std::task::block_on(async {
        let url_str = "http://localhost:8080/engine.io/";
        let mut client = Client::connect(&url_str).await.unwrap();
        client.serve().await;
    });
}
