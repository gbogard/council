use std::collections::HashSet;

use url::Url;

pub use self::gossip_destinations::*;
use self::{
    convergence_monitor::ConvergenceMonitor, failure_detector::FailureDetector, views::ClusterView,
};
use crate::node::NodeId;

pub mod convergence_monitor;
pub mod failure_detector;
pub mod version_vector;
pub mod views;

mod gossip_destinations;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone)]
pub struct Cluster {
    pub this_node_id: NodeId,
    pub cluster_view: ClusterView,
    pub peer_nodes: HashSet<Url>,
    pub unknwon_peer_nodes: HashSet<Url>,
    pub failure_detector: FailureDetector,
    pub convergence_monitor: ConvergenceMonitor,
}

impl Cluster {
    /// Increments the heartbeat of the running node by one.
    pub(crate) fn increment_own_heartbeat(&mut self) {
        if let Some(heartbeat) = self.cluster_view.heartbeats.get_mut(&self.this_node_id) {
            *heartbeat += 1
        }
        if let Some(member) = self
            .cluster_view
            .known_members
            .get_mut(&self.this_node_id)
            .and_then(|m| m.state.as_mut())
        {
            member.heartbeat += 1
        }
    }
}
