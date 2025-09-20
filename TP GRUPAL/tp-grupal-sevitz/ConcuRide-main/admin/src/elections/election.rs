use crate::coordinator_actor::coordinator::Coordinator;
use crate::coordinator_actor::coordinator_messages::BecomeCoordinator;
use crate::elections::election_messages::{
    AmICoordinator, CoordinatorMessage, ElectionMessage, GetCoordAddr, GetCoordId, PingCoordinator,
    PingMessage, SetCoordId, StartElection,
};
use crate::utils::logs::log_elections;
use actix::prelude::*;
use common::messages::WhoIsCoordinatorResponse;
use common::tcp_sender::TcpMessage;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{split, BufReader};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const MIN_PEER_PORT: u16 = 8080;
const MAX_PEER_PORT: u16 = 8084;

#[derive(Debug, Clone)]
/// This actor is responsible for the election of the coordinator.
/// It implements the Ring election algorithm.
pub struct CoordinatorElection {
    pub id: SocketAddr,
    pub coordinator_id: Option<SocketAddr>,
    pub coordinator: Arc<Addr<Coordinator>>,
    pub peers: Arc<Vec<SocketAddr>>,
    pub in_election: bool,
}

impl Actor for CoordinatorElection {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        _ctx.notify(AskForCoordinator);
    }
}

impl CoordinatorElection {
    pub fn new(
        id: SocketAddr,
        coordinator: Arc<Addr<Coordinator>>,
        peers: Arc<Vec<SocketAddr>>,
    ) -> Addr<Self> {
        CoordinatorElection::create(|_ctx| CoordinatorElection {
            id,
            coordinator_id: None,
            coordinator,
            peers,
            in_election: false,
        })
    }
}

impl Handler<SetCoordId> for CoordinatorElection {
    type Result = ();

    fn handle(&mut self, msg: SetCoordId, _ctx: &mut Self::Context) {
        self.coordinator_id = Some(msg.coord_id);
    }
}

impl Handler<GetCoordId> for CoordinatorElection {
    type Result = MessageResult<GetCoordId>;

    fn handle(&mut self, _msg: GetCoordId, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.coordinator_id)
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct AskForCoordinator;

impl Handler<AskForCoordinator> for CoordinatorElection {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: AskForCoordinator, ctx: &mut Self::Context) -> Self::Result {
        let mut election = self.clone();
        let addr = ctx.address();
        Box::pin(
            async move {
                if election.in_election || election.is_coordinator(addr.clone()).await {
                    return;
                }
                election
                    .ask_who_is_coordinator(election.peers.to_vec(), election.id, addr)
                    .await;
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<AmICoordinator> for CoordinatorElection {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: AmICoordinator, ctx: &mut Self::Context) -> Self::Result {
        let addr = ctx.address();
        let election = self.clone();
        Box::pin(async move { election.is_coordinator(addr).await })
    }
}

impl Handler<GetCoordAddr> for CoordinatorElection {
    type Result = MessageResult<GetCoordAddr>;

    fn handle(&mut self, _msg: GetCoordAddr, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.coordinator_id)
    }
}

impl Handler<PingCoordinator> for CoordinatorElection {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: PingCoordinator, ctx: &mut Self::Context) -> Self::Result {
        let election = self.clone();
        let actor_addr = ctx.address();
        Box::pin(
            async move {
                if election.is_coordinator(actor_addr.clone()).await || election.in_election {
                    return;
                }
                election.ping_coordinator(actor_addr).await;
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<StartElection> for CoordinatorElection {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: StartElection, _ctx: &mut Self::Context) -> Self::Result {
        let mut election = self.clone();
        let address = _ctx.address();

        Box::pin(
            async move {
                if election.in_election {
                    return;
                }
                election.start_election(address).await;
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<ElectionMessage> for CoordinatorElection {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: ElectionMessage, ctx: &mut Self::Context) -> Self::Result {
        println!("Received election message");
        let mut election = self.clone();
        let addr = ctx.address();

        Box::pin(
            async move {
                election.handle_election(msg, addr).await;
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl Handler<CoordinatorMessage> for CoordinatorElection {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: CoordinatorMessage, ctx: &mut Self::Context) -> Self::Result {
        let mut election = self.clone();
        let actor_addr = ctx.address();
        Box::pin(
            async move {
                election.receive_coordinator_message(msg, actor_addr).await;
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

impl CoordinatorElection {
    pub async fn start_election(&mut self, addr: Addr<CoordinatorElection>) {
        log_elections("Starting election".to_string());

        let candidates = vec![self.id];
        self.in_election = true;
        self.send_election_message(candidates, addr).await;
    }

    pub async fn handle_election(&mut self, msg: ElectionMessage, addr: Addr<CoordinatorElection>) {
        if self.in_election && !msg.candidates.contains(&self.id) {
            return;
        }

        log_elections(format!("[{:?}] Received election message", self.id));

        self.in_election = true;

        if msg.candidates.contains(&self.id) {
            log_elections(format!("Final Candidates {:?}", msg.candidates));
            self.handle_new_coordinator(msg.candidates, addr).await;
            return;
        }

        // join the election
        let mut candidates = msg.candidates;
        candidates.push(self.id);

        self.send_election_message(candidates, addr).await;
    }

    pub async fn send_election_message(
        &mut self,
        candidates: Vec<SocketAddr>,
        addr: Addr<CoordinatorElection>,
    ) {
        let msg = ElectionMessage {
            candidates: candidates.clone(),
        };
        let msg = TcpMessage(format!(
            "{}\n",
            serde_json::to_string(&msg).expect("Error converting to JSON")
        ));

        let mut next_peer_port = self.id.port();
        let mut got_ack = false;

        while !got_ack && next_peer_port != self.id.port() - 1 {
            next_peer_port = if next_peer_port >= MAX_PEER_PORT {
                MIN_PEER_PORT
            } else {
                next_peer_port + 1
            };

            let next_peer = SocketAddr::new(self.id.ip(), next_peer_port);
            log_elections(format!("Trying to connect to next peer: {:?}", next_peer));

            for _ in 0..3 {
                if let Ok(stream) = TcpStream::connect(next_peer).await {
                    let (reader, mut writer) = split(stream);
                    log_elections(format!("Connected to next election peer: {:?}", next_peer));

                    if writer.write_all(msg.0.as_bytes()).await.is_err() {
                        eprintln!("Error writing CoordinatorElection message");
                        continue;
                    }

                    let mut reader = BufReader::new(reader);
                    let mut line = String::new();

                    match timeout(Duration::from_secs(3), reader.read_line(&mut line)).await {
                        Ok(Ok(_)) => {
                            log_elections(format!("Received: {:?}", line));
                            got_ack = true;
                        }
                        Ok(Err(e)) => {
                            println!("[{:?}] Failed to read line: {:?}", next_peer, e);
                        }
                        Err(_) => {
                            log_elections("Timeout".to_string());
                        }
                    }
                    break;
                }
            }

            if !got_ack {
                log_elections("Failed to connect to next peer".to_string());
            }
        }

        if !got_ack {
            self.handle_new_coordinator(candidates, addr).await;
        }
    }

    pub async fn handle_new_coordinator(
        &mut self,
        candidates: Vec<SocketAddr>,
        addr: Addr<CoordinatorElection>,
    ) {
        let new_coordinator = match candidates.iter().min_by_key(|&x| x.port()) {
            Some(coordinator) => coordinator,
            None => {
                panic!("No candidates found");
            }
        };

        let msg = CoordinatorMessage {
            coordinator: *new_coordinator,
        };

        self.receive_coordinator_message(msg.clone(), addr).await;
        self.broadcast_coordinator(msg).await;
    }

    pub async fn receive_coordinator_message(
        &mut self,
        msg: CoordinatorMessage,
        addr: Addr<CoordinatorElection>,
    ) {
        log_elections(format!(
            "[{:?}] New coordinator: {:?}",
            self.id, msg.coordinator
        ));
        if let Err(e) = addr.try_send(SetCoordId {
            coord_id: msg.coordinator,
        }) {
            println!("Error setting coordinator id: {:?}", e);
        }

        if self.in_election {
            self.in_election = false;
        }

        if self.is_coordinator(addr.clone()).await {
            self.become_coordinator().await;
        }
    }

    pub async fn broadcast_coordinator(&self, new_coord: CoordinatorMessage) {
        let msg = format!(
            "{}\n",
            serde_json::to_string(&new_coord).expect("Error converting to JSON")
        );
        let msg = TcpMessage(msg);

        for &peer in self.peers.iter() {
            if peer == self.id {
                continue;
            }

            for _ in 0..3 {
                match TcpStream::connect(peer).await {
                    Ok(s) => {
                    let stream = Some(s);
                    let (_, mut writer) = match stream {
                        Some(stream) => split(stream),
                        None => panic!("Stream is None"),
                    };
                        if let Err(e) = writer.write_all(msg.0.as_bytes()).await {
                            eprintln!("Error writing broadcast message: {}", e);
                        }

                        log_elections(format!(
                            "[{:?}] Sent coordinator {} message to {:?}",
                            self.id,
                            new_coord.coordinator.port(),
                            peer
                        ));
                        break;
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        }
    }

    pub async fn ping_coordinator(&self, actor_addr: Addr<Self>) {
        log_elections(format!("[{:?}] Pinging coordinator", self.id));
        let coord_id = actor_addr
            .send(GetCoordId)
            .await
            .expect("Failed to get coordinator id");

        let coord = match coord_id {
            Some(coord) => coord,
            None => {
                log_elections("No coordinator".to_string());
                return;
            }
        };

        let mut got_ack = false;
        for _ in 0..3 {
            match TcpStream::connect(coord).await {
                Ok(s) => {
                    let stream = Some(s);
                    let (reader, mut writer) = match stream {
                        Some(stream) => split(stream),
                        None => panic!("Stream is None"),
                    };
                    // send message ping and wait for ack
                    let ping_msg = PingMessage { sender_id: self.id };
                    let msg =
                        TcpMessage(format!("{}\n", serde_json::to_string(&ping_msg).unwrap()));
                    if let Err(e) = writer.write_all(msg.0.as_bytes()).await {
                        eprintln!("Error writing CoordinatorElection message: {}", e);
                    }
                    let mut reader = BufReader::new(reader);
                    let mut line = String::new();

                    match timeout(Duration::from_secs(3), reader.read_line(&mut line)).await {
                        Ok(Ok(_)) => {
                            log_elections(format!("[{:?}] Received: {:?}", self.id, line));
                            got_ack = true;
                        }
                        Ok(Err(e)) => {
                            log_elections(format!("[{:?}] Failed to read line: {:?}", self.id, e));
                        }
                        Err(_) => {
                            log_elections(format!("[{:?}] Timeout", self.id));
                            got_ack = false;
                        }
                    }
                    break;
                }
                Err(_) => {
                    continue;
                }
            }
        }

        if !got_ack {
            log_elections(format!("[{:?}] Coordinator down", self.id));

            actor_addr
                .try_send(StartElection)
                .expect("Failed to start election");
        }
    }

    pub async fn become_coordinator(&self) {
        log_elections(format!("[{:?}] Becoming coordinator", self.id));

        self.coordinator
            .send(BecomeCoordinator)
            .await
            .expect("Failed to send BecomeCoordinator");
    }

    pub async fn is_coordinator(&self, addr: Addr<CoordinatorElection>) -> bool {
        let coord_id = addr
            .send(GetCoordId)
            .await
            .expect("Failed to get coordinator id");
        match coord_id {
            Some(coord_id) => self.id == coord_id,
            None => false,
        }
    }

    pub async fn ask_who_is_coordinator(
        &mut self,
        peers: Vec<SocketAddr>,
        id: SocketAddr,
        addr: Addr<CoordinatorElection>,
    ) {
        log_elections("Asking who is coordinator".to_string());
        let mut got_res = false;

        for &peer in peers.iter().filter(|&&peer| peer != id) {
            for _ in 0..3 {
                if let Ok(stream) = TcpStream::connect(peer).await {
                    let (reader, mut writer) = split(stream);
                    let msg = TcpMessage("WhoIsCoordinator\n".to_string());

                    if writer.write_all(msg.0.as_bytes()).await.is_err() {
                        eprintln!("Error writing CoordinatorElection message");
                        continue;
                    }

                    let mut reader = BufReader::new(reader);
                    let mut line = String::new();

                    match timeout(Duration::from_secs(3), reader.read_line(&mut line)).await {
                        Ok(Ok(_)) => {
                            if let Ok(who_is_coord_msg) =
                                serde_json::from_str::<WhoIsCoordinatorResponse>(&line)
                            {
                                self.receive_coordinator_message(
                                    CoordinatorMessage {
                                        coordinator: who_is_coord_msg.coord_id,
                                    },
                                    addr.clone(),
                                )
                                .await;
                                got_res = true;
                                break;
                            } else if line.trim() == "Ack" {
                                break;
                            }
                        }
                        Ok(Err(e)) => {
                            println!("Failed to read line: {:?}", e);
                        }
                        Err(_) => {
                            log_elections("Timeout".to_string());
                        }
                    }
                    break;
                }
            }
            if got_res {
                break;
            }
        }
        if !got_res {
            log_elections("No response from peers".to_string());
            self.handle_new_coordinator(vec![self.id], addr.clone())
                .await;
        }
    }
}
