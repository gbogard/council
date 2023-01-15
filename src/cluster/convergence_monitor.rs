use std::collections::HashMap;

use super::version_vector::VersionVector;
use crate::node::NodeId;

/// The [convergence monitor](ConvergenceMonitor) associates [node ids](NodeId) with their
/// last observed version vector.
///
/// By building a [VersionVectorOffset](super::version_vector::VersionVectorOffset) with the running node's [Cluster view](super::views::ClusterView)
/// as the left-hand side, and an other node's version vector (as recorded by this component) as the right-hand side, we know which nodes
/// are lagging behind the running node's cluster view; and if so, exactly which [member views](super::views::MemberView) need to be gossipped about.
///
/// This lets us save network bandwidth when gossipping: when we know that no other node is lagging behind us, we can just gossip about heartbeats
/// instead of a full cluster view. When we know that nodes are lagging behind the running node, we can gossip only about specific nodes whose states
/// have been updated, instead of gossipping our entire cluster view.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone)]
pub struct ConvergenceMonitor {
    this_node_id: NodeId,
    observed_states_per_node: HashMap<NodeId, VersionVector>,
}

impl ConvergenceMonitor {
    pub(crate) fn new(this_node_id: NodeId) -> Self {
        Self {
            this_node_id,
            observed_states_per_node: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, node_id: NodeId) -> Option<&VersionVector> {
        self.observed_states_per_node.get(&node_id)
    }

    pub(crate) fn record_version_vector(
        &mut self,
        node_id: NodeId,
        version_vector: &VersionVector,
    ) {
        debug_assert_ne!(node_id, self.this_node_id);

        if let Some(existing) = self.observed_states_per_node.get_mut(&node_id) {
            existing.merge(version_vector)
        } else {
            self.observed_states_per_node
                .insert(node_id, version_vector.clone());
        }
    }

    pub(crate) fn has_converged(&self) -> bool {
        let mut iter = self.observed_states_per_node.values();
        if let Some(first) = iter.next() {
            iter.all(|n| n == first)
        } else {
            false
        }
    }
}
