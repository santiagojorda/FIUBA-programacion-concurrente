use crate::admin_actor::admin::Admin;
use crate::coordinator_actor::coordinator_messages::ConnectNewPeer;
use crate::elections::election::CoordinatorElection;
use crate::elections::election_messages::GetCoordAddr;
use crate::elections::election_messages::{PingCoordinator, PingMessage};
use crate::utils::consts::PING_INTERVAL;
use crate::utils::logs::log_elections;
use actix::prelude::*;
use common::messages::WhoIsCoordinatorResponse;
use common::tcp_sender::TcpMessage;
use std::sync::Arc;
use tokio::time::Duration;

#[derive(Message)]
#[rtype(result = "()")]
pub struct WhoIsCoordinator;

impl Handler<PingMessage> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, message: PingMessage, _: &mut Self::Context) -> Self::Result {
        let tcp_sender_clone = self.tcp_sender.clone();
        let coord_clone = self.coordinator.clone();
        Box::pin(
            async move {
                tcp_sender_clone
                    .try_send(TcpMessage("Ack".to_string()))
                    .expect("Failed to send response");

                // connect to peer if not connected
                if let Err(e) = coord_clone.try_send(ConnectNewPeer {
                    new_peer: message.sender_id,
                }) {
                    log_elections(format!("Failed to send ConnectNewPeer: {:?}", e));
                }
            }
            .into_actor(self),
        )
    }
}

impl Handler<WhoIsCoordinator> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _: WhoIsCoordinator, _: &mut Self::Context) -> Self::Result {
        let coord_elect_clone = self.coordinator_election.clone();
        let tcp_sender_clone = self.tcp_sender.clone();

        Box::pin(
            async move {
                let coord_addr = coord_elect_clone
                    .send(GetCoordAddr {})
                    .await
                    .expect("Failed to get coordinator address");

                if coord_addr.is_none() {
                    // no coordinator
                    tcp_sender_clone
                        .try_send(TcpMessage("Ack".to_string()))
                        .expect("Failed to send response");
                    return;
                }

                let who_is_coord_msg = WhoIsCoordinatorResponse {
                    coord_id: coord_addr.unwrap(),
                };

                tcp_sender_clone
                    .try_send(TcpMessage(
                        serde_json::to_string(&who_is_coord_msg).unwrap(),
                    ))
                    .expect("Failed to send response");
            }
            .into_actor(self),
        )
    }
}

pub fn spawn_ping_task(coordinator_election: Arc<Addr<CoordinatorElection>>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(PING_INTERVAL)).await;
            if let Err(e) = coordinator_election.try_send(PingCoordinator) {
                log_elections(format!("Failed to send ping: {:?}", e));
            }
        }
    });
}
