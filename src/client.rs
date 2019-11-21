use crate::packet::{Packet, PacketData, PacketType};
use crate::payload::{Payload, PayloadDecodeError};
use async_std::task::{self, JoinHandle};
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use futures::try_join;
use serde::{Deserialize, Serialize};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

use log::{debug, error, info};

#[async_trait]
pub trait EventHandler {
    async fn on_connect(&mut self);

    async fn on_disconnect(&mut self);

    async fn on_message(&mut self, data: PacketData);
}

pub struct Client {
    write_channel: mpsc::UnboundedSender<Packet>,
    ping_handle: JoinHandle<Result<(), EIOError>>,
    poll_handle: JoinHandle<Result<(), EIOError>>,
    write_handle: JoinHandle<Result<(), EIOError>>,
}

pub struct ClientConfig {
    is_connected: AtomicBool,
    sid: String,
    base_url: String,
    ping_interval: u32,
    ping_timeout: u32,
    ping_received: AtomicBool,
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
pub enum EIOErrorKind {
    /// An error with the underlying transport
    Transport(String),
    /// A violation of the engine.io protocol
    Protocol(String),
}

#[derive(Debug)]
pub struct EIOError {
    kind: EIOErrorKind,
}

impl From<surf::Exception> for EIOError {
    fn from(err: surf::Exception) -> Self {
        Self {
            kind: EIOErrorKind::Transport(format!("{}", err)),
        }
    }
}

impl From<PayloadDecodeError> for EIOError {
    fn from(err: PayloadDecodeError) -> Self {
        Self {
            kind: EIOErrorKind::Protocol(format!("{}", err)),
        }
    }
}

impl Client {
    pub async fn join(self) -> Result<(), EIOError> {
        let poll = self.poll_handle;
        let write = self.write_handle;
        let ping = self.ping_handle;

        try_join!(poll, write, ping)?;

        Ok(())
    }

    pub async fn connect(
        url: &str,
        event_handler: impl EventHandler + Send + Sync + 'static,
    ) -> Result<Client, EIOError> {
        let connect_url = format!("{}?transport=polling&EIO=3", url);
        info!("Establishing connection to {}", connect_url);
        let bytes = surf::get(&connect_url).recv_bytes().await.unwrap();
        let payload = Payload::new(&bytes).unwrap();

        if let PacketData::Str(string) = payload.packets().first().unwrap().data() {
            let packet: OpenPacket = serde_json::from_str(string).unwrap();
            debug!("Spawning tasks, sid is {}", packet.sid);
            let (sender, receiver) = mpsc::unbounded();

            let config = Arc::new(ClientConfig {
                is_connected: AtomicBool::new(true),
                sid: packet.sid,
                base_url: url.to_owned(),
                ping_interval: packet.pingInterval,
                ping_timeout: packet.pingTimeout,
                ping_received: AtomicBool::new(true),
            });

            let poll_handle = task::spawn(Arc::clone(&config).poll_loop(event_handler));
            let ping_handle = task::spawn(Arc::clone(&config).ping_loop(sender.clone()));
            let write_handle = task::spawn(Arc::clone(&config).write_loop(receiver));

            let eio_client = Client {
                write_channel: sender.clone(),
                ping_handle,
                poll_handle,
                write_handle,
            };

            Ok(eio_client)
        } else {
            panic!("expected string");
        }
    }

    pub async fn emit_str(&mut self, data: String) {
        self.emit(PacketData::Str(data)).await;
    }

    async fn emit(&mut self, data: PacketData) {
        info!("Emitting {:?}", data);
        self.write_channel
            .send(Packet::new(PacketType::Message, data))
            .await
            .unwrap();
    }
}

impl ClientConfig {
    // self: &Arc<ClientConfig> is not yet supported (#64325)
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

    async fn ping_loop(
        self: Arc<ClientConfig>,
        mut write_channel: mpsc::UnboundedSender<Packet>,
    ) -> Result<(), EIOError> {
        let timeout = std::time::Duration::new((self.ping_timeout / 1000) as u64, 0);
        let interval = std::time::Duration::new(
            (self.ping_interval / 1000 - self.ping_timeout / 1000) as u64,
            0,
        );
        debug!("Interval {:?}, Timeout {:?}", interval, timeout);
        while self.is_connected.load(Ordering::Relaxed) {
            write_channel
                .send(Packet::with_str(PacketType::Ping, "probe"))
                .await
                .unwrap();

            self.ping_received.store(false, Ordering::SeqCst);

            async_std::task::sleep(timeout).await;

            // We expect to have received a pong after this time
            if !self.ping_received.load(Ordering::SeqCst) {
                error!("Pong not received, aborting");
                Self::disconnect(&self).await;
                break;
            }

            async_std::task::sleep(interval).await;
        }
        debug!("Exit ping loop");
        Ok(())
    }

    async fn poll_loop(
        self: Arc<ClientConfig>,
        mut event_handler: impl EventHandler + Send + Sync,
    ) -> Result<(), EIOError> {
        while self.is_connected.load(Ordering::Relaxed) {
            let url = Self::get_url(&self);
            debug!("Polling {}", url);

            let bytes = match surf::get(&url).recv_bytes().await {
                Ok(bytes) => bytes,
                Err(exc) => {
                    Self::disconnect(&self).await;
                    return Err(exc.into());
                }
            };

            let payload = Payload::new(&bytes)?;

            for packet in payload.into_packets() {
                debug!("Received {:?}", packet);
                Self::handle_packet(&self, packet, &mut event_handler).await;
            }
        }
        debug!("Exit poll loop");
        Ok(())
    }

    async fn write_loop(
        self: Arc<ClientConfig>,
        mut receiver: mpsc::UnboundedReceiver<Packet>,
    ) -> Result<(), EIOError> {
        while let Some(packet) = receiver.next().await {
            debug!("Sending {:?}", packet);
            let payload = Payload::from_packet(packet);
            let url = Self::get_url(&self);
            let _response = surf::post(&url).body_bytes(payload.encode_binary()).await;
        }
        debug!("Exit write loop");
        Ok(())
    }

    async fn handle_packet(
        config: &Arc<ClientConfig>,
        packet: Packet,
        event_handler: &mut (dyn EventHandler + Send + Sync),
    ) {
        match packet.packet_type() {
            PacketType::Pong => {
                config.ping_received.store(true, Ordering::SeqCst);
            }
            PacketType::Close => {
                Self::disconnect(config).await;
                event_handler.on_disconnect().await;
            }
            PacketType::Message => {
                event_handler.on_message(packet.into_data()).await;
            }
            PacketType::Noop => (),
            _ => {
                error!("Unexpected packet {:?}", packet);
            }
        }
    }

    async fn disconnect(config: &Arc<ClientConfig>) {
        info!("Disconnecting");
        config.is_connected.store(false, Ordering::Relaxed);
    }
}
