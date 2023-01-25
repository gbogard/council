use std::{collections::HashMap, future::Future};

use rand::seq::SliceRandom;
use tokio::task::JoinSet;
use url::Url;

use super::{
    version_vector::VersionVectorOffset,
    views::{MemberView, PartialClusterView},
    Cluster,
};
use crate::node::NodeId;

const GOSSIP_DESTINATIONS_SAMPLE_SIZE: u8 = 5;

impl Cluster {
    /// Selects destinations to gossip with, up to a maximum of five, and triggers a gossip function for each destination.
    ///
    /// Gossips can take two forms:
    ///  - Heartbeats exchanges consist of sending the last recorded heartbeat for each cluster node,
    ///    and getting the last recorded heartbeats of the destination in return.
    ///    Such exchanges are relatively lightweight and are triggered when we know that the destination isn't lagging behind us
    ///    (as recorded by the ConvergenceMonitor)
    ///  - Cluster view exchanges consist of sending a partial view of the cluster (as seen by the running node) and getting a partial
    ///   of the cluster (as seen by the destination) in return.
    ///   They are heavier than heartbeats exchanges, and are triggered when we know that a node is lagging behind us,
    ///   (as recorded by the ConvergenceMonitor)
    pub(crate) fn select_gossip_destinations<ExchangeHeartbeatsF, ExchangeClusterViewsF>(
        &mut self,
        exchange_heartbeats: ExchangeHeartbeatsF,
        exchange_cluster_views: ExchangeClusterViewsF,
    ) where
        ExchangeClusterViewsF: Fn(Url, PartialClusterView),
        ExchangeHeartbeatsF: Fn(Url, HashMap<NodeId, u64>),
    {
        let mut cluster_view_exchanges = 0;
        let mut heartbeats_destinations = Vec::new();

        for url in &self.unknwon_peer_nodes {
            if cluster_view_exchanges >= GOSSIP_DESTINATIONS_SAMPLE_SIZE {
                break;
            }

            {
                log::debug!(
                    "[Node Id: {}] Exchanging cluster views with unknown peer node {}",
                    self.this_node_id,
                    url
                );
                let view = PartialClusterView {
                    this_node_id: self.this_node_id,
                    members: self.cluster_view.known_members.clone(),
                };
                cluster_view_exchanges += 1;
                exchange_cluster_views(url.clone(), view);
            }
        }

        for (_, n) in &self.cluster_view.known_members {
            if cluster_view_exchanges >= GOSSIP_DESTINATIONS_SAMPLE_SIZE {
                break;
            }
            if n.id == self.this_node_id {
                continue;
            }

            if let Some(partial_cluster_view_to_exchange) =
                self.collect_partial_cluster_view_of_newer_nodes(n.id)
            {
                log::debug!(
                    "[Node id: {}] Exchanging partial cluster view with known member {}",
                    self.this_node_id,
                    n.id
                );
                cluster_view_exchanges += 1;
                exchange_cluster_views(n.advertised_addr.clone(), partial_cluster_view_to_exchange);
            } else {
                heartbeats_destinations.push(n.advertised_addr.clone());
            }
        }

        let heartbeats_exchanges = GOSSIP_DESTINATIONS_SAMPLE_SIZE - cluster_view_exchanges;
        if heartbeats_destinations.len() > 0 && heartbeats_exchanges > 0 {
            log::debug!(
                "[Node Id: {}] Exchanging heartbeats with {} random members",
                self.this_node_id,
                heartbeats_exchanges
            );

            for dest in heartbeats_destinations
                .choose_multiple(&mut rand::thread_rng(), heartbeats_exchanges as usize)
            {
                exchange_heartbeats(dest.clone(), self.cluster_view.heartbeats.clone());
            }
        }
    }

    /// Given a node id, this returns a partial cluster view containing only the nodes
    /// that we think have been updated since the node corresponding to `node_id` has last seen them.
    ///
    /// This uses the convergence monitor to filter nodes that are lagging behind.
    pub(crate) fn collect_partial_cluster_view_of_newer_nodes(
        &self,
        node_id: NodeId,
    ) -> Option<PartialClusterView> {
        self.convergence_monitor
            .get(node_id)
            .map(|rhs| {
                let offset = VersionVectorOffset::of(&self.cluster_view.version_vector, rhs);
                let dest: HashMap<NodeId, MemberView> = offset
                    .behind_lhs
                    .iter()
                    .filter_map(|node_id| {
                        self.cluster_view
                            .known_members
                            .get(node_id)
                            .map(|n| (*node_id, n.clone()))
                    })
                    .collect();
                dest
            })
            .and_then(|destinations| {
                if destinations.is_empty() {
                    None
                } else {
                    Some(PartialClusterView {
                        this_node_id: self.this_node_id,
                        members: destinations,
                    })
                }
            })
    }
}
