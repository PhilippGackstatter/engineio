use crate::payload::Payload;
use reqwest;
use serde::{Deserialize, Serialize};

enum ConnectionState {
    Disconnected,
    Connected,
}

pub struct Client {
    conn_state: ConnectionState,
    ping_interval: u32,
    ping_timeout: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenPacket {
    sid: String,
    upgrades: Vec<String>,
    pingInterval: u32,
    pingTimeout: u32,
}

#[derive(Debug)]
pub struct ConnectionError {}

impl From<reqwest::Error> for ConnectionError {
    fn from(_error: reqwest::Error) -> ConnectionError {
        ConnectionError {}
    }
}

impl Client {
    pub async fn connect(url: &str) -> Result<Client, ConnectionError> {
        let client = reqwest::Client::builder().build().unwrap();
        let payload = client
            .get(url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
            .parse::<Payload>()
            .unwrap();

        let packet: OpenPacket =
            serde_json::from_str(payload.packets().first().unwrap().data()).unwrap();

        Ok(Client {
            conn_state: ConnectionState::Connected,
            ping_interval: packet.pingInterval,
            ping_timeout: packet.pingTimeout,
        })
    }
}
