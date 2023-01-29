use std::{collections::HashMap, error::Error, ops::Deref, sync::Arc};

use cluster::{views::PartialClusterView, Cluster};
use grpc::{
    client::{CouncilClient, HeartbeatMessage},
    TonicChannelFactory,
};
use node::NodeId;
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
    time::Interval,
};
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};
use url::Url;

#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[macro_use]
extern crate log;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde_with;

mod builder;

pub mod cluster;
pub mod grpc;
pub mod node;

pub use self::builder::*;

pub struct Council {
    pub this_node_id: NodeId,
    cluster_events_sender: broadcast::Sender<ClusterEvent>,
    tonic_channel_factory: Arc<dyn TonicChannelFactory + Send + Sync>,
    main_thread_message_sender: mpsc::Sender<Message>,
    _main_thread: JoinHandle<()>,
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

    /// Subscribes to cluster events and returns a stream of events happening in the cluster.
    /// This function can be called any number of times, from multiple threads. Each call
    /// will return a new stream that produces events starting from the moment the stream was created
    /// (events that happened before the stream was created won't be received)
    ///
    /// Every time the membership status of a node changes, all subscribing streams will receive a [ClusterEvent], containing
    /// a pointer to the state of the cluster at the time the event was produced.
    /// Drop the stream to cancel the subscription
    pub fn events(&self) -> impl Stream<Item = ClusterEvent> + Send + Sync {
        BroadcastStream::new(self.cluster_events_sender.subscribe()).filter_map(|i| i.ok())
    }

    /// Returns a clone of the cluster's state
    pub async fn cluster(&self) -> Result<Cluster, Box<dyn Error>> {
        let (tx, rx) = oneshot::channel();
        self.main_thread_message_sender
            .send(Message::GetCurrentClusterClone { reply: tx })
            .await?;
        Ok(rx.await?)
    }

    pub(crate) async fn main_thread(
        mut outgoing_gossip_interval: Interval,
        mut cluster: Cluster,
        mut message_receiver: mpsc::Receiver<Message>,
        mut message_sender: mpsc::Sender<Message>,
        mut cluster_events_sender: broadcast::Sender<ClusterEvent>,
        client: Arc<CouncilClient>,
    ) {
        outgoing_gossip_interval.tick().await;
        loop {
            select! {
                Some(incoming_message) = message_receiver.recv() => {
                    match incoming_message {
                        Message::ReconcileClusterView { incoming_cluster_view, reconciled_cluster_view_reply } =>
                        {
                            handle_incoming_cluster_view(&mut cluster, incoming_cluster_view, reconciled_cluster_view_reply, &mut cluster_events_sender).await;
                        },
                        Message::GetCurrentClusterClone { reply} => {
                            let _ = reply.send(cluster.clone());
                        }
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

impl Drop for Council {
    fn drop(&mut self) {
        log::info!("Council instance {} is shutting down!", self.this_node_id);
        // TODO: graceful shutdown!
    }
}

async fn gossip(
    cluster: &mut Cluster,
    client: &Arc<CouncilClient>,
    message_sender: &mut mpsc::Sender<Message>,
) {
    for dest in cluster.select_gossip_destinations() {
        let client = Arc::clone(&client);
        let message_sender = message_sender.clone();
        tokio::spawn(async move {
            if let Ok(res) = client
                .exchange_cluster_views(dest.destination_url, dest.cluster_view)
                .await
            {
                let _ = message_sender.send(Message::ReconcileClusterView {
                    incoming_cluster_view: res,
                    reconciled_cluster_view_reply: None,
                });
            }
        });
    }
}

async fn handle_incoming_cluster_view(
    cluster: &mut Cluster,
    incoming_cluster_view: PartialClusterView,
    reply: Option<oneshot::Sender<PartialClusterView>>,
    cluster_event_sender: &mut broadcast::Sender<ClusterEvent>,
) {
    let incoming_node_id = incoming_cluster_view.this_node_id;

    debug_assert_ne!(
        incoming_node_id, cluster.this_node_id,
        "Node {} received a view from itself, this is not supposed to happen",
        incoming_node_id
    );

    log::trace!(
        "[Node id: {}] Received incoming cluster view from node {} containing {} members",
        cluster.this_node_id,
        incoming_node_id,
        incoming_cluster_view.members.len()
    );

    for (_, mut member) in incoming_cluster_view.members {
        if member.id == incoming_node_id {
            cluster.unknwon_peer_nodes.remove(&member.advertised_addr);
        }

        if let Some(state) = &mut member.state {
            if member.id != cluster.this_node_id {
                cluster
                    .failure_detector
                    .record_heartbeat(member.id, state.heartbeat);
            }
        }

        cluster
            .cluster_view
            .merge_member_view(cluster.this_node_id, member);
    }

    // Notify outside subscribers that the cluster state has changed
    if cluster_event_sender.receiver_count() > 0 {
        let _ = cluster_event_sender.send(ClusterEvent {
            cluster: Arc::new(cluster.clone()),
        });
    }

    // Reply to the initiator of the gossip request, if applicable
    if let Some(reply) = reply {
        let partial_cluster_view_to_send = PartialClusterView {
            this_node_id: cluster.this_node_id,
            members: cluster.cluster_view.known_members.clone(),
        };
        let _ = reply.send(partial_cluster_view_to_send);
    }
}

/// Messages are sent by the gRPC server when incoming requests are received
/// They allow a running [Council] instance to communicate with its gRPC server
#[derive(Debug)]
enum Message {
    ReconcileClusterView {
        incoming_cluster_view: PartialClusterView,
        reconciled_cluster_view_reply: Option<oneshot::Sender<PartialClusterView>>,
    },
    GetCurrentClusterClone {
        reply: oneshot::Sender<Cluster>,
    },
}

#[derive(Debug, Clone)]
pub struct ClusterEvent {
    pub cluster: Arc<Cluster>,
}

impl Deref for ClusterEvent {
    type Target = Cluster;

    fn deref(&self) -> &Self::Target {
        self.cluster.deref()
    }
}
