use std::{collections::HashSet, time::Instant};

use url::Url;

use self::{failure_detector::FailureDetector, views::ClusterView};
use crate::node::NodeId;

pub mod failure_detector;
pub mod version_vector;
pub mod views;

mod gossip_destinations;

/// Represents the state of the cluster
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone, Debug)]
pub struct Cluster {
    pub this_node_id: NodeId,
    pub this_advertised_url: Url,
    pub cluster_view: ClusterView,
    pub peer_nodes: HashSet<Url>,
    pub unknwon_peer_nodes: HashSet<Url>,
    pub failure_detector: FailureDetector,
}

impl Cluster {
    pub fn has_converged(&self) -> bool {
        let now = Instant::now();
        let all_members_ids: HashSet<NodeId> =
            self.cluster_view.known_members.keys().cloned().collect();

        let all_members_are_live_and_converged =
            self.cluster_view.known_members.iter().all(|(_, member)| {
                let member_is_live =
                    member.id == self.this_node_id || self.failure_detector.is_live(member.id, now);
                let member_observed_all_states = member
                    .state
                    .as_ref()
                    .map_or(false, |state| state.observed_by == all_members_ids);
                member_is_live && member_observed_all_states
            });

        self.unknwon_peer_nodes.is_empty() && all_members_are_live_and_converged
    }

    /// Increments the heartbeat of the running node by one and returns the new value
    pub(crate) fn increment_own_heartbeat(&mut self) -> u64 {
        if let Some(heartbeat) = self.cluster_view.heartbeats.get_mut(&self.this_node_id) {
            *heartbeat += 1;
        }
        if let Some(member) = self
            .cluster_view
            .known_members
            .get_mut(&self.this_node_id)
            .and_then(|m| m.state.as_mut())
        {
            member.heartbeat += 1;
            member.heartbeat
        } else {
            panic!("Failed to increment heartbeat")
        }
    }
}

#[cfg(test)]
mod tests;
