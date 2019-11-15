use crate::packet::{Packet, PacketData, PacketType};
use crate::payload::Payload;
use async_std::future::join;
use async_std::task::JoinHandle;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;

// const SUPPORTED_TRANSPORT: [&str; 1] = ["polling"];

#[derive(PartialEq)]
enum ConnectionState {
    // Disconnected,
    Connected,
}

pub struct Client {
    ping_handle: JoinHandle<()>,
    poll_handle: JoinHandle<()>,
    // config: std::sync::Arc<ClientConfig>,
}

pub struct ClientConfig {
    conn_state: ConnectionState,
    sid: String,
    base_url: String,
    ping_interval: u32,
    // ping_timeout: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct OpenPacket {
    sid: String,
    upgrades: Vec<String>,
    pingInterval: u32,
    pingTimeout: u32,
}

#[derive(Debug)]
pub struct ConnectionError {}

// impl From<surf::Exception> for ConnectionError {
//     fn from(_error: surf::Exception) -> ConnectionError {
//         ConnectionError {}
//     }
// }

impl Client {
    pub async fn serve(&mut self) {
        let ping = &mut self.ping_handle;
        let poll = &mut self.poll_handle;
        join!(poll, ping).await;
    }

    pub async fn connect(url: &str) -> Result<Client, ConnectionError> {
        let connect_url = format!("{}?transport=polling&EIO=3", url);
        println!("Establishing connection to {}", connect_url);
        let bytes = surf::get(&connect_url).recv_bytes().await.unwrap();
        let payload = Payload::from_str_colon_msg_format(&bytes).unwrap();

        if let PacketData::Str(string) = payload.packets().first().unwrap().data() {
            let packet: OpenPacket = serde_json::from_str(string).unwrap();
            println!("Spawning tasks, sid is {}", packet.sid);
            let config = std::sync::Arc::new(ClientConfig {
                conn_state: ConnectionState::Connected,
                sid: packet.sid,
                base_url: url.to_owned(),
                ping_interval: packet.pingInterval,
                // ping_timeout: packet.pingTimeout,
            });

            let poll_handle = async_std::task::spawn(Client::poll_loop(Arc::clone(&config)));
            let ping_handle = async_std::task::spawn(Client::ping_loop(Arc::clone(&config)));
            let eio_client = Client {
                ping_handle,
                poll_handle,
                // config,
            };

            Ok(eio_client)
        } else {
            panic!("expected string");
        }
    }

    fn get_url(config: &Arc<ClientConfig>) -> String {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        format!(
            "{}?transport=polling&EIO=3&sid={}&t={}.{}",
            config.base_url,
            config.sid,
            time.as_secs(),
            time.subsec_nanos()
        )
    }

    async fn ping_loop(config: Arc<ClientConfig>) {
        let interval = std::time::Duration::new((config.ping_interval / 1000) as u64, 0);
        loop {
            let payload = Payload::from_packet(Packet::new(PacketType::Ping, "probe"));
            let url = Client::get_url(&config);
            println!("Pinging (interval {}) {}", interval.as_secs(), url);
            let _response = surf::post(&url).body_bytes(payload.encode_binary()).await;
            async_std::task::sleep(interval).await;
        }
    }

    async fn poll_loop(config: Arc<ClientConfig>) {
        #[allow(clippy::while_immutable_condition)]
        while config.conn_state == ConnectionState::Connected {
            let url = Client::get_url(&config);
            println!("Polling {}", url);
            let bytes = surf::get(&url).recv_bytes().await.unwrap();
            let payload = match Payload::new(&bytes) {
                Ok(pl) => pl,
                Err(_) => Payload::from_str_colon_msg_format(&bytes).unwrap(),
            };

            for packet in payload.packets() {
                println!("{:?} - {:?}", packet.packet_type(), packet.data());
            }
        }
    }
}
