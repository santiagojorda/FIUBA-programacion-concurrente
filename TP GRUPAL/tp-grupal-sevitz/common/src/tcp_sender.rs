use actix::prelude::*;
use actix_async_handler::async_handler;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;

pub struct TcpSender {
    pub write: Option<WriteHalf<TcpStream>>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpMessage(pub String);

impl Actor for TcpSender {
    type Context = Context<Self>;
}

#[async_handler]
impl Handler<TcpMessage> for TcpSender {
    type Result = ();

    async fn handle(&mut self, msg: TcpMessage, _ctx: &mut Self::Context) -> Self::Result {
        let to_send = format!("{}\n", msg.0);

        if let Some(mut write) = self.write.take() {
            let ret_write = async move {
                if let Err(e) = write.write_all(to_send.as_bytes()).await {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        return None;
                    } else {
                        panic!("Should have sent: {:?}", e);
                    }
                }
                Some(write)
            }
            .await;

            self.write = ret_write;
        }
    }
}
