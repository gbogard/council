pub use protos::gossip_service_server::GossipServiceServer;
use tokio::sync::{mpsc::Sender, oneshot};
use tonic::{async_trait, transport::Channel, Response, Status};

use super::protos;
use crate::{Council, Message};

pub struct CouncilGrpcServer {
    main_thread_message_sender: Sender<Message>,
}

impl Council {
    /// Returns a Tonic gRPC Server.
    /// To actually start accepting requests, you need to create a [Server](tonic::transport::Server),
    /// then add this service to it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tonic::transport::Server
    /// use url::Url;
    /// use council::*;
    ///
    /// let socket_addr =  "[::1]:50051".parse()?;
    /// let advertised_url = Url::parse("http://localhost:50051")?;
    /// let council = Council::builder(advertised_url).build();
    /// Server::builder()
    ///     .add_service(council.gossip_grpc_service())
    ///     .serve(socket_addr)
    ///     .await?;
    /// ```
    ///
    pub fn gossip_grpc_service(&self) -> GossipServiceServer<CouncilGrpcServer> {
        let server = CouncilGrpcServer {
            main_thread_message_sender: self.main_thread_message_sender.clone(),
        };
        GossipServiceServer::new(server)
    }
}

#[async_trait]
impl protos::gossip_service_server::GossipService for CouncilGrpcServer {
    async fn exchange_cluster_views(
        &self,
        request: tonic::Request<protos::PartialClusterView>,
    ) -> Result<tonic::Response<protos::PartialClusterView>, tonic::Status> {
        let (reply_tx, reply_rx) = oneshot::channel();
        // TODO: handle error
        self.main_thread_message_sender
            .send(Message::ReconcileClusterView {
                incoming_cluster_view: request.into_inner().into(),
                reconciled_cluster_view_reply: Some(reply_tx),
            })
            .await
            .map_err(|e| Status::unavailable(e.to_string()))?;
        let reply = reply_rx
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?
            .into();
        Ok(Response::new(reply))
    }

    async fn exchange_heartbeats(
        &self,
        request: tonic::Request<protos::HeartbeatMessage>,
    ) -> Result<tonic::Response<protos::HeartbeatMessage>, tonic::Status> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.main_thread_message_sender
            .send(Message::ReconcileHeartbeats {
                incoming_heartbeats: request.into_inner().into(),
                reconciled_heartbeats_reply: Some(reply_tx),
            })
            .await
            .map_err(|e| Status::unavailable(e.to_string()))?;
        let reply = reply_rx
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?
            .into();
        Ok(Response::new(reply))
    }
}
