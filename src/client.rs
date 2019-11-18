use crate::packet::{Packet, PacketData, PacketType};
use crate::payload::Payload;
use async_std::future::join;
use async_std::task::{self, JoinHandle};
use fnv::FnvHashMap;
use futures::channel::mpsc;
use futures::future::{BoxFuture, FutureExt};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::future::Future;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

use log::{debug, error, info};

#[derive(Default)]
pub struct ClientBuilder {
    handlers: FnvHashMap<String, Box<DynEventHandler>>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        ClientBuilder {
            handlers: FnvHashMap::default(),
        }
    }

    pub fn connect_handler(mut self, event_handler: impl EventHandler) -> Self {
        self.add("connect", event_handler);
        self
    }

    pub fn disconnect_handler(mut self, event_handler: impl EventHandler) -> Self {
        self.add("disconnect", event_handler);
        self
    }

    pub fn message_handler(mut self, event_handler: impl EventHandler) -> Self {
        self.add("message", event_handler);
        self
    }

    fn add(&mut self, event_name: &str, event_handler: impl EventHandler) {
        let dyn_handler = Box::new(move |data| event_handler.call(data).boxed());
        self.handlers.insert(event_name.to_owned(), dyn_handler);
    }

    pub fn build(self, url: &str) -> impl Future<Output = Result<Client, ConnectionError>> + '_ {
        Client::connect(url, self.handlers)
    }
}

pub struct Client {
    write_channel: mpsc::UnboundedSender<Packet>,
    ping_handle: JoinHandle<()>,
    poll_handle: JoinHandle<()>,
    write_handle: JoinHandle<()>,
}

pub(crate) type DynEventHandler =
    dyn (Fn(PacketData) -> BoxFuture<'static, ()>) + 'static + Send + Sync;

pub struct ClientConfig {
    is_connected: AtomicBool,
    sid: String,
    base_url: String,
    ping_interval: u32,
    ping_timeout: u32,
    ping_received: AtomicBool,
    handlers: FnvHashMap<String, Box<DynEventHandler>>,
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
        let write = &mut self.write_handle;
        join!(poll, ping, write).await;
    }

    pub async fn connect(
        url: &str,
        handlers: FnvHashMap<String, Box<DynEventHandler>>,
    ) -> Result<Client, ConnectionError> {
        let connect_url = format!("{}?transport=polling&EIO=3", url);
        info!("Establishing connection to {}", connect_url);
        let bytes = surf::get(&connect_url).recv_bytes().await.unwrap();
        let payload = Payload::from_str_colon_msg_format(&bytes).unwrap();

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
                handlers,
            });

            let poll_handle = task::spawn(Arc::clone(&config).poll_loop());
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

    async fn ping_loop(self: Arc<ClientConfig>, mut write_channel: mpsc::UnboundedSender<Packet>) {
        let timeout = std::time::Duration::new((self.ping_timeout / 1000) as u64, 0);
        let interval = std::time::Duration::new(
            (self.ping_interval / 1000 - self.ping_timeout / 1000) as u64,
            0,
        );
        debug!("Interval {:?}, Timeout {:?}", interval, timeout);
        loop {
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
    }

    async fn poll_loop(self: Arc<ClientConfig>) {
        while self.is_connected.load(Ordering::Relaxed) {
            let url = Self::get_url(&self);
            debug!("Polling {}", url);
            let bytes = surf::get(&url).recv_bytes().await.unwrap();
            let payload = match Payload::new(&bytes) {
                Ok(pl) => pl,
                Err(_) => Payload::from_str_colon_msg_format(&bytes).unwrap(),
            };

            for packet in payload.into_packets() {
                debug!("Received {:?}", packet);
                Self::handle_packet(&self, packet).await;
            }
        }
        debug!("Exit poll loop");
    }

    async fn write_loop(self: Arc<ClientConfig>, mut receiver: mpsc::UnboundedReceiver<Packet>) {
        while let Some(packet) = receiver.next().await {
            debug!("Sending {:?}", packet);
            let payload = Payload::from_packet(packet);
            let url = Self::get_url(&self);
            let _response = surf::post(&url).body_bytes(payload.encode_binary()).await;
        }
        debug!("Exit write loop");
    }

    async fn handle_packet(config: &Arc<ClientConfig>, packet: Packet) {
        match packet.packet_type() {
            PacketType::Pong => {
                config.ping_received.store(true, Ordering::SeqCst);
            }
            PacketType::Close => {
                Self::disconnect(config).await;
            }
            PacketType::Message => {
                Self::trigger_event("message", config, packet.into_data()).await;
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
        Self::trigger_event("disconnect", config, PacketData::Str("".to_owned())).await;
    }

    async fn trigger_event(event_name: &str, config: &Arc<ClientConfig>, event_data: PacketData) {
        if let Some(event_handler) = config.handlers.get(event_name) {
            debug!("Trigger event {}", event_name);
            event_handler(event_data).await;
        }
    }
}

pub trait EventHandler: Send + Sync + 'static {
    /// The async result of `call`.
    type Fut: Future<Output = ()> + Send + 'static;

    /// Invoke the event handler with the received data
    fn call(&self, data: PacketData) -> Self::Fut;
}

// Implement EventHandler for all functions with a
// Future as return type
// Allows passing async fns as event handlers
impl<F: Send + Sync + 'static, Fut> EventHandler for F
where
    F: Fn(PacketData) -> Fut,
    Fut: Future + Send + 'static,
    // Fut::Output: (),
{
    type Fut = BoxFuture<'static, ()>;

    fn call(&self, data: PacketData) -> Self::Fut {
        let fut = (self)(data);

        Box::pin(async move {
            fut.await;
        })
    }
}
