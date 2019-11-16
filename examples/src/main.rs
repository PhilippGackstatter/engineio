use engineio::{Client, ClientError};

fn main() {
    async_std::task::block_on(async {
        let url_str = "http://localhost:8080/engine.io/";
        let mut client = Client::connect(&url_str, fun).await.unwrap();
        client.serve().await;
    });
}

async fn fun() -> ClientError {
    println!("fun!");
    ClientError {}
}
