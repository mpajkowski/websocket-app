use crate::{broker::Event, channel::Channel, frame::Frame};
use anyhow::{Context, Result};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::{convert::TryFrom, net::SocketAddr, sync::Arc};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

pub type ClientTx = SplitSink<WebSocketStream<TcpStream>, Message>;

/// Contains client session info
#[derive(Debug)]
pub struct Client {
    tx: ClientTx,
    addr: SocketAddr,
    last_message: Option<Value>,
    channels: HashSet<Arc<dyn Channel>>,
}

impl Client {
    /// Creates new client
    ///
    /// # Arguments:
    /// * `tx` - websocket write half
    /// * `addr` - socket
    /// * `last_message` - last delivered message
    /// * `channels` - subscribed channels
    pub fn new(tx: ClientTx, addr: SocketAddr) -> Client {
        Client {
            tx,
            addr,
            last_message: Some(json!({})),
            channels: HashSet::new(),
        }
    }

    /// Subscribes to channel
    ///
    /// # Arguments:
    /// * `channel` - channel pointer
    pub fn subscribe(&mut self, channel: Arc<dyn Channel>) {
        self.channels.insert(channel);
    }

    /// Unsubscribes from channel
    ///
    /// # Arguments:
    /// * `channel` - channel pointer
    pub fn unsubscribe(&mut self, channel: Arc<dyn Channel>) {
        self.channels.remove(&channel);
    }

    /// Returns socket addr
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns subscription list
    pub fn channels(&self) -> &HashSet<Arc<dyn Channel>> {
        &self.channels
    }

    /// Yanks last message
    pub fn take_last_message(&mut self) -> Option<Value> {
        self.last_message.take()
    }

    /// Setter for last message
    pub fn set_last_message(&mut self, last_message: Value) {
        self.last_message = Some(last_message)
    }

    /// Writes frame to websocket
    ///
    /// # Arguments:
    /// * `frame` - frame from broker
    pub async fn send_msg(&mut self, frame: Frame) -> Result<()> {
        let message = frame.socket_msg();
        self.tx.send(message).await?;

        Ok(())
    }
}

/// Client connection loop
///
/// Manages client's connection lifecycle - negotates the session, pushes incoming messages towards broker
/// and sends disconnect event at the end of the session
///
/// # Arguments:
/// * `raw_stream` - TCP connection to client
/// * `broker_tx` - broker's mpsc channel write half
pub async fn handle_connection(
    raw_stream: TcpStream,
    broker_tx: UnboundedSender<Event>,
) -> Result<()> {
    let addr = raw_stream.peer_addr()?;
    log::info!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .with_context(|| "Error during the websocket handshake occurred")?;

    log::info!("WebSocket connection established: {}", addr);

    let (outgoing, mut incoming) = ws_stream.split();

    // push session info towards broker
    broker_tx.send(Event::new_client(addr, Client::new(outgoing, addr)))?;

    // read incoming messages
    while let Some(msg) = incoming.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                log::info!("Connection closed: {}", e);
                break;
            }
        };

        log::debug!("Received msg from addr={}", addr);

        // unpack message or wait for next one
        let frame = match Frame::try_from(&msg) {
            Ok(frame) => frame,
            Err(e) => {
                log::info!("Failed to unpack msg: {:?}, {}", msg, e);
                continue;
            }
        };

        log::debug!("Unpacked frame: {:?}", frame);

        broker_tx.send(Event::new_client_frame(addr, frame))?;
    }

    // TODO: Handle websocket close handshake

    // EOF - send disconnect event
    broker_tx.send(Event::disconnect(addr))?;

    log::info!("{} disconnected", &addr);

    Ok(())
}
