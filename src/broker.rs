use crate::{
    channel::Channel,
    client::Client,
    frame::{Frame, FrameData},
    state::State,
    utils::create_json_snapshot,
};
use anyhow::Result;
use futures::stream::StreamExt;
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;

/// Events occuring on client's websocket
#[derive(Debug)]
pub struct Event {
    addr: SocketAddr,
    data: EventData,
}

/// Specializations of events
#[derive(Debug)]
pub enum EventData {
    NewClient(Client),
    ClientFrame(Frame),
    Disconnect,
}

impl Event {
    /// Creates "new client" event
    ///
    /// # Arguments:
    /// * `addr` - socket
    /// * `client` - client info
    pub fn new_client(addr: SocketAddr, client: Client) -> Event {
        Event {
            addr,
            data: EventData::NewClient(client),
        }
    }

    /// Creates "new client frame" event
    ///
    /// # Arguments:
    /// * `addr` - socket
    /// * `client_frame` - frame unpacked from client's message
    pub fn new_client_frame(addr: SocketAddr, client_frame: Frame) -> Event {
        Event {
            addr,
            data: EventData::ClientFrame(client_frame),
        }
    }

    /// Creates "disconnect" event
    ///
    /// # Arguments:
    /// * `addr` - socket
    pub fn disconnect(addr: SocketAddr) -> Event {
        Event {
            addr,
            data: EventData::Disconnect,
        }
    }

    /// Consumes `Event` and returns associated data
    pub fn event_data(self) -> EventData {
        self.data
    }
}

/// Channel subscribtion events
#[derive(Debug, Clone, Copy)]
enum ManageSubscription {
    Subscribe,
    Unsubscribe,
}

type ClientMap = HashMap<SocketAddr, Client>;
type ChannelMap = HashMap<String, Arc<dyn Channel>>;

/// Event dispatcher
pub struct Broker {
    rx: UnboundedReceiver<Event>,
    state: Arc<Mutex<State>>,
    client_map: ClientMap,
    channel_map: ChannelMap,
}

impl Broker {
    /// Creates new broker
    ///
    /// # Arguments:
    /// * `rx` - reading half of event mpsc channel
    /// * `state` - a pointer to application state, protected by `Mutex`
    pub fn new(rx: UnboundedReceiver<Event>, state: Arc<Mutex<State>>) -> Broker {
        Broker {
            rx,
            state,
            client_map: HashMap::new(),
            channel_map: HashMap::new(),
        }
    }

    /// Adds channel to broker
    ///
    /// # Arguments:
    /// * `channel` - a pointer to channel
    pub fn add_channel(&mut self, channel: Arc<dyn Channel>) -> &mut Self {
        self.channel_map.insert(channel.name().to_string(), channel);
        self
    }

    /// Worker future, performs broker logic
    pub async fn worker(&mut self) -> Result<()> {
        while let Some(event) = self.rx.next().await {
            self.handle_event(event).await;
            log::info!("Connected clients: {}", self.client_map.len());
        }

        Ok(())
    }

    /// Handles incoming event
    ///
    /// # Arguments:
    /// * `event` - incoming event
    async fn handle_event(&mut self, event: Event) {
        use EventData::*;

        let addr = event.addr;

        match event.event_data() {
            NewClient(client) => {
                self.client_map.insert(addr, client);
            }
            Disconnect => {
                self.client_map.remove(&addr);
            }
            ClientFrame(frame) => {
                log::info!("Received frame: {:?}", frame);

                let send_msg_result = match frame.data() {
                    FrameData::Subscribe { channels } => {
                        self.manage_subscription(
                            addr,
                            &frame,
                            &*channels,
                            ManageSubscription::Subscribe,
                        )
                        .await
                    }
                    FrameData::Unsubscribe { channels } => {
                        self.manage_subscription(
                            addr,
                            &frame,
                            &*channels,
                            ManageSubscription::Unsubscribe,
                        )
                        .await
                    }
                    FrameData::Ready => self.fetch_data_from_channels(addr, &frame).await,
                    _ => unreachable!(),
                };

                if let Err(e) = send_msg_result {
                    log::error!("An error occurred while sending message: {}", e);
                }
            }
        }
    }

    /// Updates client's subscription state
    ///
    /// # Arguments:
    /// * `addr` - socket
    /// * `frame` - subscribe/unsubscribe frame received from client
    /// * `channels` - a list of channels to sub/unsub
    /// * `mode` - subscribe or unsubscribe
    async fn manage_subscription(
        &mut self,
        addr: SocketAddr,
        frame: &Frame,
        channels: &[String],
        mode: ManageSubscription,
    ) -> Result<()> {
        let client = Self::get_client(&mut self.client_map, addr);
        let chan_map = &self.channel_map;

        // find channels that are not registered within broker but requested by client
        let (requested_channels, not_registered): (Vec<&str>, Vec<&str>) = channels
            .iter()
            .map(|s| s.as_str())
            .partition(|chan| chan_map.contains_key(*chan));

        let resp = if not_registered.is_empty() {
            requested_channels.into_iter().for_each(|chan| {
                let channel_ptr = Arc::clone(&chan_map.get(chan).unwrap());

                match mode {
                    ManageSubscription::Subscribe => {
                        client.subscribe(channel_ptr);
                    }
                    ManageSubscription::Unsubscribe => {
                        client.unsubscribe(channel_ptr);
                    }
                }

                log::info!(
                    "{} {} channel {}",
                    addr,
                    match mode {
                        ManageSubscription::Subscribe => "subscribed to",
                        ManageSubscription::Unsubscribe => "unsubscribed from",
                    },
                    chan
                );
            });

            Frame::create_ok_frame(&frame)
        } else {
            log::info!(
                "Client {} attempted to {} following channels: {:?}",
                addr,
                match mode {
                    ManageSubscription::Subscribe => "subscribe to",
                    ManageSubscription::Unsubscribe => "unsubscribe from",
                },
                not_registered
            );

            Frame::create_err_frame(
                &frame,
                404,
                format!(
                    "Following channels were not found: {}",
                    not_registered.join(",")
                ),
            )
        };

        client.send_msg(resp).await
    }

    /// Fetches live data for client.
    ///
    /// Locks the state in `read` mode, extracts data from channels observed by the client.
    /// Sends only incremental diff of observed state.
    ///
    /// # Arguments:
    /// * `addr` - socket
    /// * `frame` - frame received from client
    /// * `channels` - a list of channels to sub/unsub
    /// * `mode` - subscribe or unsubscribe
    async fn fetch_data_from_channels(&mut self, addr: SocketAddr, frame: &Frame) -> Result<()> {
        use std::collections::BTreeMap;
        let client = Self::get_client(&mut self.client_map, addr);

        let payload = {
            let state = self.state.lock().await;
            let mut payload = BTreeMap::new();
            for chan in client.channels().iter() {
                let k = chan.name();
                let data = chan.extract_data(&state).await.unwrap();
                payload.insert(k, data);
            }

            payload
        };

        let payload = serde_json::to_value(&payload).unwrap();

        let mut snapshot = client.take_last_message().unwrap();
        create_json_snapshot(&mut snapshot, &payload);

        let response = Frame::create_data_frame(&frame, snapshot);
        client.set_last_message(payload);

        client.send_msg(response).await
    }

    /// Finds Client by socket
    ///
    /// # Arguments:
    /// * `client_map` - client map from broker
    /// * `addr` - socket
    fn get_client(client_map: &mut ClientMap, addr: SocketAddr) -> &mut Client {
        client_map.get_mut(&addr).unwrap()
    }
}
