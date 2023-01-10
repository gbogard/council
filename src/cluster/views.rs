use std::{collections::HashMap, hash::Hasher, str::FromStr};

#[cfg(test)]
use quickcheck::Arbitrary;
use url::Url;

use crate::node::{clock::Clock, id::NodeId, NodeStatus};

/// A view of how the running node views the cluster, that is how it views itself and its peers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterView {
    pub this_node: ThisNodeView,
    pub other_members: HashMap<NodeId, MemberView>,
}

#[cfg(test)]
impl Arbitrary for ClusterView {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            this_node: ThisNodeView::arbitrary(g),
            other_members: HashMap::<NodeId, MemberView>::arbitrary(g),
        }
    }
}

impl ClusterView {
    pub fn hash(&self) -> ClusterViewHash {
        let mut out = hash_node_id_clock_and_status(
            self.this_node.id,
            self.this_node.clock,
            self.this_node.status,
        );
        for (_, node) in self.other_members.iter() {
            if let Some(state) = &node.state {
                out = out ^ hash_node_id_clock_and_status(node.id, state.clock, state.node_status);
            } else {
                out = out ^ node.id.to_u64()
            }
        }
        ClusterViewHash(out)
    }
}

impl ClusterView {
    pub(crate) fn initial(this_node_advertised_url: Url, peer_members: &[Url]) -> Self {
        let mut other_members = HashMap::new();
        for url in peer_members {
            if url != &this_node_advertised_url {
                let member = MemberView::initial(&url);
                other_members.insert(member.id, member);
            }
        }
        let this_node = ThisNodeView {
            id: NodeId::from_url(&this_node_advertised_url),
            advertised_addr: this_node_advertised_url,
            clock: Clock::new(),
            status: NodeStatus::Joining,
        };
        Self {
            this_node,
            other_members,
        }
    }

    /// Merges a received view from another node into this view.
    /// This function makes [ClusterView] a Convergent Replicated Data Type (CvRDT)
    pub(crate) fn merge(&mut self, other_view: ClusterView) {
        let other_view_node = MemberView::from(&other_view);
        let other_members_iterator = other_view
            .other_members
            .into_iter()
            .map(|kv| kv.1)
            .chain(std::iter::once(other_view_node));

        for other_node in other_members_iterator {
            if other_node.id == self.this_node.id {
                if let Some(state) = other_node.state {
                    self.this_node.clock = std::cmp::max(self.this_node.clock, state.clock);
                    self.this_node.status = NodeStatus::preceding_status(
                        self.this_node.status,
                        self.this_node.clock,
                        state.node_status,
                        state.clock,
                    );
                }
            } else {
                if let Some(existing_member_view) = self.other_members.get_mut(&other_node.id) {
                    existing_member_view.merge(other_node);
                } else {
                    self.other_members.insert(other_node.id, other_node);
                }
            }
        }
    }
}

/// A view of how the running node views itself
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThisNodeView {
    pub id: NodeId,
    pub advertised_addr: Url,
    pub clock: Clock,
    pub status: NodeStatus,
}

#[cfg(test)]
impl Arbitrary for ThisNodeView {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            id: NodeId::arbitrary(g),
            advertised_addr: Url::from_str("test:8080").unwrap(),
            clock: Clock::arbitrary(g),
            status: NodeStatus::arbitrary(g),
        }
    }
}

/// A view of how the running node views one of its peers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberView {
    pub id: NodeId,
    pub advertised_addr: Url,
    pub state: Option<MemberViewState>,
}

impl From<&ClusterView> for MemberView {
    fn from(value: &ClusterView) -> Self {
        Self {
            id: value.this_node.id,
            advertised_addr: value.this_node.advertised_addr.clone(),
            state: Some(MemberViewState {
                node_status: value.this_node.status,
                clock: value.this_node.clock,
                last_cluster_view: value.hash(),
                last_cluster_view_stable_since: 0,
            }),
        }
    }
}

impl MemberView {
    pub(crate) fn initial(advertised_addr: &Url) -> Self {
        Self {
            id: NodeId::from_url(&advertised_addr),
            advertised_addr: advertised_addr.clone(),
            state: None,
        }
    }

    fn merge(&mut self, other: MemberView) {
        match (&mut self.state, other.state) {
            (None, Some(s)) => self.state = Some(s),
            (Some(self_state), Some(other_state)) => {
                self_state.node_status = NodeStatus::preceding_status(
                    self_state.node_status,
                    self_state.clock,
                    other_state.node_status,
                    other_state.clock,
                );

                if other_state.clock > self_state.clock {
                    self_state.clock = other_state.clock;
                    if self_state.last_cluster_view == other_state.last_cluster_view {
                        self_state.last_cluster_view_stable_since += 1;
                    } else {
                        self_state.last_cluster_view = other_state.last_cluster_view;
                        self_state.last_cluster_view_stable_since = 1;
                    }
                }
            }
            _ => (),
        };
        self.id = std::cmp::max(self.id, other.id)
    }
}

#[cfg(test)]
impl Arbitrary for MemberView {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            id: NodeId::arbitrary(g),
            advertised_addr: Url::from_str("test:8080").unwrap(),
            state: Option::<MemberViewState>::arbitrary(g),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberViewState {
    pub node_status: NodeStatus,
    pub clock: Clock,
    pub last_cluster_view: ClusterViewHash,
    pub last_cluster_view_stable_since: u64,
}

#[cfg(test)]
impl Arbitrary for MemberViewState {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let clock = Clock::arbitrary(g);
        /*
        Since the clock is monotonically increasing on each node, we will never have merge two views of the same node,
        with the same clock but different states associated with them. There exsits only one possible state for a given node id and clock.

        However if we generated each field randomly, it would possible for the test to generate two member views with the same id
        and the same clock but different states, which would in turn make the associativity and commutativity tests fail.
        To prevent that, instead of completely random fields, we generate the last_cluster_view so that a given clock is associated to exactly one cluster view.
        */
        let last_cluster_view = ClusterViewHash(clock.heartbeat);
        Self {
            node_status: NodeStatus::arbitrary(g),
            clock,
            last_cluster_view,
            last_cluster_view_stable_since: 1,
        }
    }
}

/// A [ClusterViewHash] is a deterministic hash of how the running node
/// views itself and its peers in the cluster. It is used to determine view convergence.
///
/// Each node keeps a hash of the last cluster view it has received from its peers, and a counter of
/// how many clock ticks this view has been stable for. When all nodes in the cluster have the same [ClusterViewHash]
/// for severtal ticks, we know that the clust has reached convergence.
#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct ClusterViewHash(pub(crate) u64);

/// Returns a hash from a triplet of [NodeId], [Clock] and [NodeStatus].
/// The resulting hash depends on the clock's started_at time, but not the precise heartbeat.
/// The hash is obtained by a bitwise XOR of the node_id (which is itself a URL hash), the clock's
/// UNIX timestamp, and the node_status.
fn hash_node_id_clock_and_status(node_id: NodeId, clock: Clock, status: NodeStatus) -> u64 {
    // the prime number 17 is used to "iron out" the distribution bias of status
    node_id.to_u64() ^ clock.started_at.unix_timestamp() as u64 ^ (status as u64 * 17)
}
