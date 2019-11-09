use engineio::Client;

#[tokio::main]
async fn main() {
    let url_str = "http://localhost:8080/engine.io/?transport=polling";
    let _client = Client::connect(&url_str).await.unwrap();
}
