use anyhow::Result;
use futures::{
    sink::SinkExt,
    stream::{SplitSink, StreamExt},
};
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

#[derive(Debug)]
pub enum Event {
    NewPeer {
        addr: SocketAddr,
        tx: SplitSink<WebSocketStream<TcpStream>, Message>,
    },
    Disconnect {
        addr: SocketAddr,
    },
    WireMessage {
        addr: SocketAddr,
        msg: Message,
    },
}

pub async fn broker_loop(mut rx: UnboundedReceiver<Event>) -> Result<()> {
    use Event::*;

    let mut peer_map: HashMap<SocketAddr, SplitSink<WebSocketStream<TcpStream>, Message>> =
        HashMap::new();

    while let Some(event) = rx.next().await {
        match event {
            NewPeer { addr, tx } => {
                peer_map.insert(addr, tx);
            }
            Disconnect { addr } => {
                peer_map.remove(&addr);
            }
            WireMessage { addr, msg } => {
                let it =
                    peer_map
                        .iter_mut()
                        .filter_map(|(sock, tx)| if sock != &addr { Some(tx) } else { None });

                for peer in it {
                    peer.send(msg.clone()).await?;
                }
            }
        }
    }

    Ok(())
}
