use std::collections::HashMap;

use url::Url;

use crate::node::{id::NodeId, NodeStatus};

/// A view of how the running node views the cluster, that is how it views itself and its peers.
pub struct ClusterView {
    pub this_node: ThisNodeView,
    pub other_members: HashMap<NodeId, MemberView>,
}

impl ClusterView {
    pub(crate) fn initial(this_node_advertised_url: Url, peer_members: &[Url]) -> Self {
        let this_node = ThisNodeView {
            id: NodeId::from_url(&this_node_advertised_url),
            advertised_addr: this_node_advertised_url,
            heartbeat: 0,
        };
        let mut other_members = HashMap::new();
        for url in peer_members {
            let member = MemberView::initial(&url);
            other_members.insert(member.id, member);
        }
        Self {
            this_node,
            other_members,
        }
    }
}

/// A view of how the running node views itself
pub struct ThisNodeView {
    pub id: NodeId,
    pub advertised_addr: Url,
    pub heartbeat: u64,
}

/// A view of how the running node views one of its peers
pub struct MemberView {
    pub id: NodeId,
    pub advertised_addr: Url,
    pub state: Option<MemberViewState>,
}

impl MemberView {
    pub(crate) fn initial(advertised_addr: &Url) -> Self {
        Self {
            id: NodeId::from_url(&advertised_addr),
            advertised_addr: advertised_addr.clone(),
            state: None,
        }
    }
}

pub struct MemberViewState {
    pub node_status: NodeStatus,
    pub last_heartbeat: u64,
    pub last_cluster_view: ClusterViewHash,
    pub last_cluster_view_stable_since: u64,
}

/// A [ClusterViewHash] is a deterministic hash of how the running node
/// views itself and its peers in the cluster. It is used to determine view convergence.
///
/// Each node keeps a hash of the last cluster view it has received from its peers, and a counter of
/// how many clock ticks this view has been stable for. When all nodes in the cluster have the same [ClusterViewHash]
/// for severtal ticks, we know that the clust has reached convergence.
#[repr(transparent)]
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct ClusterViewHash(pub(crate) u64);
