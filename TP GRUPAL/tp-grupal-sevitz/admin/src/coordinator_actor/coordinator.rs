use crate::coordinator_actor::coordinator_messages::*;
use crate::utils::admin_errors::AdminError;
use crate::utils::consts::MAX_RETRIES;
use crate::utils::consts::MAX_TIME_WITHOUT_PINGING;
use actix::prelude::*;
use common::tcp_sender::{TcpMessage, TcpSender};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::io::split;
use tokio::net::TcpStream;

/// This actor is responsible for the comunication between Coordinator and Admins
/// so it can balance trips between them
pub struct Coordinator {
    pub addr: SocketAddr,
    pub peers: Vec<SocketAddr>,
    pub peer_handles: Peers,
    pub peer_counter: u8,
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Coordinator {
    pub fn new(addr: SocketAddr, peers: Vec<SocketAddr>) -> Addr<Self> {
        Coordinator::create(|_ctx| Coordinator {
            addr,
            peers,
            peer_handles: Arc::new(HashMap::new()),
            peer_counter: 0,
        })
    }
}

impl Handler<BecomeCoordinator> for Coordinator {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: BecomeCoordinator, _ctx: &mut Self::Context) -> Self::Result {
        let addr = self.addr;
        let peers = self.peers.clone();
        let actor_addr = _ctx.address();

        Box::pin(
            async move {
                if let Err(e) = connect_to_peers(addr, peers, actor_addr).await {
                    eprintln!("Failed to connect to peers: {:?}", e);
                }
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<HandleTrip> for Coordinator {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: HandleTrip, _ctx: &mut Self::Context) -> Self::Result {
        let peers = self.peers.clone();
        let actor_addr = _ctx.address();

        Box::pin(
            async move {
                let mut handle = None;
                let mut times_retry = peers.len();

                while handle.is_none() && times_retry > 0 {
                    let counter = actor_addr.send(GetPeerCounter).await.unwrap_or_default();
                    let peer = peers[counter as usize % peers.len()];
                    let handles = actor_addr.send(GetPeerDict).await.unwrap_or_default();
                    if let Some(tuple) = handles.get(&peer) {
                        if tuple.1.elapsed().as_secs() >= MAX_TIME_WITHOUT_PINGING {
                            times_retry -= 1;
                            continue;
                        }
                        handle = Some(tuple.clone());
                    }
                    times_retry -= 1;
                }

                if let Some(h) = handle {
                    match serde_json::to_string(&msg) {
                        Ok(handle_trip_message) => {
                            h.0.try_send(TcpMessage(handle_trip_message))
                                .expect("Failed to send HandleTrip");
                        }
                        Err(err) => {
                            eprintln!("Error serializing HandleTrip message: {}", err);
                        }
                    }
                }
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<UpdatePassengers> for Coordinator {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: UpdatePassengers, _ctx: &mut Self::Context) -> Self::Result {
        let msg = serde_json::to_string(&msg).expect("Error converting to JSON");
        let actor_addr = _ctx.address();

        Box::pin(
            async move {
                broadcast_update(actor_addr, msg).await;
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<UpdateDrivers> for Coordinator {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: UpdateDrivers, _ctx: &mut Self::Context) -> Self::Result {
        let msg = serde_json::to_string(&msg).expect("Error converting to JSON");
        let actor_addr = _ctx.address();

        Box::pin(
            async move {
                broadcast_update(actor_addr, msg).await;
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<GetPeerCounter> for Coordinator {
    type Result = u8;

    fn handle(&mut self, _msg: GetPeerCounter, _ctx: &mut Self::Context) -> Self::Result {
        let current = self.peer_counter;
        self.peer_counter += 1;
        current
    }
}

impl Handler<ConnectNewPeer> for Coordinator {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: ConnectNewPeer, _ctx: &mut Self::Context) -> Self::Result {
        let addr = self.addr;
        let peer = msg.new_peer;
        let actor_addr = _ctx.address();

        Box::pin(
            async move {
                let handles = actor_addr.send(GetPeerDict).await.unwrap_or_default();
                if handles.contains_key(&peer) {
                    // reset the time since last ping and return
                    if let Err(e) = actor_addr.try_send(AddPeerToDict {
                        peer_addr: peer,
                        peer_sender: handles[&peer].0.clone(),
                    }) {
                        println!("[{}] Failed to update peer: {:?}", addr, e);
                    }
                    return;
                }

                match connect_to_peer(addr, peer, actor_addr).await {
                    Ok(_) => (),
                    Err(e) => println!("[{}] Failed to connect to {:?}: {:?}", addr, peer, e),
                }
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<AddPeerToDict> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: AddPeerToDict, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(handles) = Arc::get_mut(&mut self.peer_handles) {
            handles.insert(msg.peer_addr, (msg.peer_sender, Instant::now()));
        }
    }
}

impl Handler<GetPeerDict> for Coordinator {
    type Result = Peers;

    fn handle(&mut self, _msg: GetPeerDict, _ctx: &mut Self::Context) -> Self::Result {
        self.peer_handles.clone()
    }
}

pub async fn broadcast_update(coord_actor: Addr<Coordinator>, msg: String) {
    let handles = coord_actor.send(GetPeerDict).await.unwrap_or_default();
    for handle in handles.values() {
        let res = handle.0.try_send(TcpMessage(msg.clone()));
        if res.is_err() {
            continue;
        }
    }
}

async fn connect_to_peers(
    addr: SocketAddr,
    peers: Vec<SocketAddr>,
    coord_actor: Addr<Coordinator>,
) -> Result<(), AdminError> {
    println!("[{}] Connecting to peers {:?}", addr, peers);

    for &peer in peers.iter() {
        connect_to_peer(addr, peer, coord_actor.clone()).await?;
    }

    Ok(())
}

async fn connect_to_peer(
    addr: SocketAddr,
    peer: SocketAddr,
    coord_actor: Addr<Coordinator>,
) -> Result<(), AdminError> {
    if addr == peer {
        return Ok(()); // Skip connecting to self
    }

    let mut attempts = 0;
    let mut stream = None;

    while attempts < MAX_RETRIES {
        match TcpStream::connect(peer).await {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(_) => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }

    if let Some(stream) = stream {
        println!("[{}] Connected to {:?}", addr, peer);
        let (_, w_half) = split(stream);

        let write = Some(w_half);
        let sender_actor = TcpSender { write }.start();

        coord_actor
            .try_send(AddPeerToDict {
                peer_addr: peer,
                peer_sender: sender_actor,
            })
            .expect("Error adding peer to dictionary.");
    } else {
        println!("[{}] Could not connect to peer {:?}", addr, peer);
    }

    Ok(())
}
