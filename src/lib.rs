use std::sync::Arc;

use cluster::{views::PartialClusterView, Cluster};
use grpc::{
    client::{CouncilClient, HeartbeatMessage},
    TonicChannelFactory,
};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
    time::Interval,
};
use url::Url;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod builder;

pub mod cluster;
pub mod grpc;
pub mod node;

pub use self::builder::*;

pub struct Council {
    cluster_events_receiver: broadcast::Receiver<Cluster>,
    tonic_channel_factory: Arc<dyn TonicChannelFactory + Send + Sync>,
    main_thread_message_sender: mpsc::Sender<Message>,
    main_thread: JoinHandle<()>,
}

impl Council {
    pub fn builder(this_node_advertised_url: Url) -> CouncilBuilder {
        CouncilBuilder::new(this_node_advertised_url)
    }

    /// The [TonicChannelFactory] that this Council instance uses to communicate with other nodes.
    /// This is exposed so you can use it your application to obtain gRPC channels. This way you can resuse,
    /// in your own code, channels that Council has already opened and avoid creating redundant channels.
    pub fn tonic_channel_factory(&self) -> &(dyn TonicChannelFactory + Send + Sync) {
        self.tonic_channel_factory.as_ref()
    }

    pub(crate) async fn main_thread(
        mut outgoing_gossip_interval: Interval,
        mut cluster: Cluster,
        mut message_receiver: mpsc::Receiver<Message>,
        mut message_sender: mpsc::Sender<Message>,
        _cluster_events_sender: broadcast::Sender<Cluster>,
        client: Arc<CouncilClient>,
    ) {
        loop {
            select! {
                Some(incoming_message) = message_receiver.recv() => {
                    match incoming_message {
                        Message::ReconcileClusterView { incoming_cluster_view, reconciled_cluster_view_reply } => handle_incoming_cluster_view(incoming_cluster_view).await,
                        Message::ReconcileHeartbeats { incoming_heartbeats, reconciled_heartbeats_reply } => handle_incoming_heartbeat_message(incoming_heartbeats).await,
                    }
                },
                _ = outgoing_gossip_interval.tick() => {
                    cluster.increment_own_heartbeat();
                    // TODO: take leadership action here!
                    gossip(&mut cluster, &client, &mut message_sender).await;
                    // TODO: implement garbage collection here ?
                }
            }
        }
    }
}

async fn gossip(
    cluster: &mut Cluster,
    client: &Arc<CouncilClient>,
    main_thread_message_sender: &mut mpsc::Sender<Message>,
) {
    cluster.select_gossip_destinations(
        |url, heartbeat_message| {
            let client = Arc::clone(&client);
            let main_thread_message_sender = main_thread_message_sender.clone();
            tokio::spawn(async move {
                if let Ok(incoming_heartbeats) =
                    client.exchange_heartbeats(url, heartbeat_message).await
                {
                    let _ = main_thread_message_sender
                        .send(Message::ReconcileHeartbeats {
                            incoming_heartbeats,
                            reconciled_heartbeats_reply: None,
                        })
                        .await;
                }
            });
        },
        |url, partial_cluster_view| {
            let client = Arc::clone(&client);
            let main_thread_message_sender = main_thread_message_sender.clone();
            tokio::spawn(async move {
                if let Ok(incoming_cluster_view) = client
                    .exchange_cluster_views(url, partial_cluster_view)
                    .await
                {
                    let _ = main_thread_message_sender
                        .send(Message::ReconcileClusterView {
                            incoming_cluster_view,
                            reconciled_cluster_view_reply: None,
                        })
                        .await;
                }
            });
        },
    );
}

async fn handle_incoming_cluster_view(cluster_view: PartialClusterView) {}

async fn handle_incoming_heartbeat_message(message: HeartbeatMessage) {}

/// Messages are sent by the gRPC server when incoming requests are received
/// They allow a running [Council] instance to communicate with its gRPC server
enum Message {
    ReconcileClusterView {
        incoming_cluster_view: PartialClusterView,
        reconciled_cluster_view_reply: Option<oneshot::Sender<PartialClusterView>>,
    },

    ReconcileHeartbeats {
        incoming_heartbeats: HeartbeatMessage,
        reconciled_heartbeats_reply: Option<oneshot::Sender<HeartbeatMessage>>,
    },
}
