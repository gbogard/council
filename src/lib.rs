use std::{collections::HashMap, sync::Arc};

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

#[macro_use]
extern crate log;

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
        _cluster_events_sender: broadcast::Sender<Cluster>,
        client: Arc<CouncilClient>,
    ) {
        loop {
            select! {
                Some(incoming_message) = message_receiver.recv() => {
                    match incoming_message {
                        Message::ReconcileClusterView { incoming_cluster_view, reconciled_cluster_view_reply } =>
                        {
                            handle_incoming_cluster_view(&mut cluster, incoming_cluster_view, reconciled_cluster_view_reply).await;
                        }    ,
                        Message::ReconcileHeartbeats { incoming_heartbeats, reconciled_heartbeats_reply } =>
                            handle_incoming_heartbeat_message(&mut cluster, incoming_heartbeats, reconciled_heartbeats_reply).await,
                    }
                },
                _ = outgoing_gossip_interval.tick() => {
                    cluster.increment_own_heartbeat();
                    // TODO: take leadership action here!
                    gossip(&mut cluster, &client).await;
                    // TODO: implement garbage collection here ?
                }
            }
        }
    }
}

async fn gossip(cluster: &mut Cluster, client: &Arc<CouncilClient>) {
    let mut gossip_results = cluster.select_gossip_destinations(
        |url, heartbeat_message| {
            let client = Arc::clone(&client);
            async move {
                client
                    .exchange_heartbeats(url, heartbeat_message)
                    .await
                    .ok()
                    .map(|incoming_heartbeats| Message::ReconcileHeartbeats {
                        incoming_heartbeats,
                        reconciled_heartbeats_reply: None,
                    })
            }
        },
        |url, partial_cluster_view| {
            let client = Arc::clone(&client);
            async move {
                client
                    .exchange_cluster_views(url, partial_cluster_view)
                    .await
                    .ok()
                    .map(|incoming_cluster_view| Message::ReconcileClusterView {
                        incoming_cluster_view,
                        reconciled_cluster_view_reply: None,
                    })
            }
        },
    );

    while let Some(message) = gossip_results.join_next().await {
        match message {
            Ok(Some(Message::ReconcileClusterView {
                incoming_cluster_view,
                reconciled_cluster_view_reply,
            })) => {
                handle_incoming_cluster_view(
                    cluster,
                    incoming_cluster_view,
                    reconciled_cluster_view_reply,
                )
                .await
            }
            Ok(Some(Message::ReconcileHeartbeats {
                incoming_heartbeats,
                reconciled_heartbeats_reply,
            })) => {
                handle_incoming_heartbeat_message(
                    cluster,
                    incoming_heartbeats,
                    reconciled_heartbeats_reply,
                )
                .await
            }
            _ => (),
        }
    }
}

async fn handle_incoming_cluster_view(
    cluster: &mut Cluster,
    incoming_cluster_view: PartialClusterView,
    reply: Option<oneshot::Sender<PartialClusterView>>,
) {
    let incoming_node_id = incoming_cluster_view.this_node_id;
    for (_, member) in &incoming_cluster_view.members {
        if let Some(state) = &member.state {
            cluster
                .convergence_monitor
                .record_version(incoming_node_id, member.id, state.version);
            cluster
                .failure_detector
                .record_heartbeat(member.id, state.heartbeat)
        }
    }

    cluster
        .cluster_view
        .merge_partial_cluster_view(incoming_cluster_view);

    if let Some(reply) = reply {
        let partial_cluster_view_to_send = cluster
            .collect_partial_cluster_view_of_newer_nodes(incoming_node_id)
            .unwrap_or(PartialClusterView {
                this_node_id: cluster.this_node_id,
                members: HashMap::default(),
            });

        let _ = reply.send(partial_cluster_view_to_send);
    }
}

async fn handle_incoming_heartbeat_message(
    cluster: &mut Cluster,
    incoming_message: HeartbeatMessage,
    reply: Option<oneshot::Sender<HeartbeatMessage>>,
) {
    for (node_id, heartbeat) in incoming_message {
        cluster.cluster_view.record_heartbeat(node_id, heartbeat);
        cluster
            .failure_detector
            .record_heartbeat(node_id, heartbeat);
    }

    if let Some(reply) = reply {
        let _ = reply.send(cluster.cluster_view.heartbeats.clone());
    }
}

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
