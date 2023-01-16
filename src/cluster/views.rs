use std::collections::HashMap;

#[cfg(test)]
use quickcheck::Arbitrary;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use url::Url;

use super::version_vector::VersionVector;
use crate::node::{NodeId, NodeStatus};

/// A view of how the running node views the cluster, that is how it views itself and its peers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ClusterView {
    pub known_members: HashMap<NodeId, MemberView>,
    pub(crate) version_vector: VersionVector,
    pub(crate) heartbeats: HashMap<NodeId, u64>,
}

impl ClusterView {
    pub(crate) fn initial(this_node_id: NodeId, this_node_advertised_url: Url) -> Self {
        let mut known_members = HashMap::new();
        let mut heartbeats = HashMap::new();
        let mut version_vector = VersionVector::default();

        let this_node = MemberView::this_node_initial_view(this_node_id, this_node_advertised_url);
        version_vector.versions.insert(
            this_node_id,
            this_node.state.as_ref().map_or(0, |s| s.version),
        );
        heartbeats.insert(
            this_node.id,
            this_node.state.as_ref().map_or(0, |s| s.heartbeat),
        );
        known_members.insert(this_node_id, this_node);

        Self {
            known_members,
            version_vector,
            heartbeats,
        }
    }

    /// Merges a received view from another node into this view.
    /// This function makes [ClusterView] a Convergent Replicated Data Type (CvRDT)
    pub(crate) fn merge(&mut self, other_view: ClusterView) {
        for (_, other_node) in other_view.known_members {
            if let Some(existing_member_view) = self.known_members.get_mut(&other_node.id) {
                existing_member_view.merge(other_node);
                self.version_vector.versions.insert(
                    existing_member_view.id,
                    existing_member_view.state.as_ref().map_or(0, |s| s.version),
                );
                self.heartbeats.insert(
                    existing_member_view.id,
                    existing_member_view
                        .state
                        .as_ref()
                        .map_or(0, |s| s.heartbeat),
                );
            } else {
                self.version_vector.versions.insert(
                    other_node.id,
                    other_node.state.as_ref().map_or(0, |s| s.version),
                );
                self.heartbeats.insert(
                    other_node.id,
                    other_node.state.as_ref().map_or(0, |s| s.heartbeat),
                );
                self.known_members.insert(other_node.id, other_node);
            }
        }
    }
}

pub(crate) struct PartialClusterView {
    pub(crate) this_node_id: NodeId,
    pub(crate) members: HashMap<NodeId, MemberView>,
}

/// A view of how the running node views one of its peers
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MemberView {
    pub id: NodeId,
    pub advertised_addr: Url,
    pub state: Option<MemberViewState>,
}

impl MemberView {
    fn this_node_initial_view(id: NodeId, advertised_url: Url) -> Self {
        Self {
            id,
            advertised_addr: advertised_url,
            state: Some(MemberViewState {
                node_status: NodeStatus::Joining,
                heartbeat: 0,
                version: 1,
            }),
        }
    }

    fn merge(&mut self, incoming: MemberView) {
        debug_assert_eq!(
            self.id, incoming.id,
            "Tried to merge unrelated member views"
        );

        if incoming.id.generation > self.id.generation {
            *self = incoming
        } else if incoming.id.generation == self.id.generation {
            match (&mut self.state, incoming.state) {
                // Whenever the incoming view carries data and ours doesn't, discard our view
                (None, Some(s)) => self.state = Some(s),
                // If the incoming version is stricly superior to our local view's version, accept the incoming node status and version
                (Some(self_state), Some(incoming_state))
                    if incoming_state.version > self_state.version =>
                {
                    self_state.node_status = incoming_state.node_status;
                    self_state.version = incoming_state.version;
                    self_state.heartbeat =
                        std::cmp::max(self_state.heartbeat, incoming_state.heartbeat);
                }
                // If our local view's version is strictly superior to the incoming view's version, keep our local node status and version
                (Some(self_state), Some(incoming_state))
                    if incoming_state.version < self_state.version =>
                {
                    self_state.heartbeat =
                        std::cmp::max(self_state.heartbeat, incoming_state.heartbeat);
                }
                // If our both view have the same version number but conflicting statuses, resolve the conflict
                // by applying status priority rules
                (Some(self_state), Some(incoming_state))
                    if self_state.node_status != incoming_state.node_status =>
                {
                    self_state.version += 1;
                    self_state.heartbeat =
                        std::cmp::max(self_state.heartbeat, incoming_state.heartbeat);
                    self_state.node_status =
                        std::cmp::max(self_state.node_status, incoming_state.node_status)
                }
                _ => (),
            }
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MemberViewState {
    pub node_status: NodeStatus,
    /// This increases every time the node's status changes (from Joining to Up, from Up to Leaving ...).
    /// The version number is used to reconcile gossips. When reconciling two views of the same node,
    /// the highest version value always wins.
    ///
    /// When two views have the same version number but conflicting statuses,
    /// we resolve the conflict by picking the highest-prority status:
    /// - [NodeStatus::Up] precedes [NodeStatus::Joining]
    /// - [NodeStatus::Leaving] precedes [NodeStatus::Up] and [NodeStatus::Joining]
    /// - [NodeStatus::Exiting] precedes [NodeStatus::Leaving], [NodeStatus::Up] and [NodeStatus::Joining]
    /// - [NodeStatus::Down] precedes all other statuses
    pub version: u16,
    /// The heartbeat is a monotonically increasing counter used to detect failure.
    /// Nodes will regularly increment their own counter and gossip the latest value to their peers.
    /// Nodes are not allowed to update other nodes' heartbeats counter.
    pub heartbeat: u64,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    /// Tests that for all states a, b and c, merging b into a and then c into a is equivalent to merging c into b and then b into a, i.e.
    /// that the merge function is associative. This is the first important property of state-based CRDT. Commutativity and idempotence are
    /// tested below.
    ///
    /// We could express associativity as "for all states a, b and c, merge(merge(a, b), c) = merge(a, merge(b, c))", except that here, we mutate
    /// state rather than returning an immutable state so "merge" is not quite exactly a function in the mathemtical sense.
    #[quickcheck]
    fn cluster_view_merge_is_associative(
        mut a: ClusterView,
        mut b: ClusterView,
        c: ClusterView,
    ) -> bool {
        let merged_a_and_b_first = {
            let mut res = a.clone();
            res.merge(b.clone());
            res.merge(c.clone());
            res
        };
        let merged_b_and_c_first = {
            b.merge(c);
            a.merge(b);
            a
        };
        merged_a_and_b_first == merged_b_and_c_first
    }

    /// Tests that for all states a and b, merging a into b is equivalent to merging b into a, i.e. that merging is commutative.
    /// Again, this could be expressed as "merge(a, b) = merge(b, a) for all states a and b", except that we use mutation rather then immutable states.
    #[quickcheck]
    fn cluster_view_merge_is_commutative(a: ClusterView, mut b: ClusterView) -> bool {
        let merged_a_b = {
            let mut a = a.clone();
            a.merge(b.clone());
            a
        };
        let merged_b_a = {
            b.merge(a);
            b
        };
        merged_a_b == merged_b_a
    }

    /// Tests that for all states a and b, once we have merged b into a, merging b again into the resulting state is a no-op,
    /// i.e. that merging is idempotent.
    #[quickcheck]
    fn cluster_view_merge_is_idempotent(mut a: ClusterView, b: ClusterView) -> bool {
        let merged_a_b = {
            a.merge(b.clone());
            a
        };
        let merged_a_b_b = {
            let mut merged_a_b = merged_a_b.clone();
            merged_a_b.merge(b);
            merged_a_b
        };
        merged_a_b == merged_a_b_b
    }

    #[quickcheck]
    fn member_view_merge_is_associative(
        mut a: MemberView,
        mut b: MemberView,
        c: MemberView,
    ) -> bool {
        // The merge function panics in debug mode if we attempt to merge unrelated nodes
        // so we should prepare our test data so it doesn't happen
        a.id = c.id;
        b.id = c.id;

        let merged_a_and_b_first = {
            let mut res = a.clone();
            res.merge(b.clone());
            res.merge(c.clone());
            res
        };
        let merged_b_and_c_first = {
            b.merge(c);
            a.merge(b);
            a
        };
        merged_a_and_b_first == merged_b_and_c_first
    }

    #[quickcheck]
    fn member_view_merge_is_commutative(a: MemberView, mut b: MemberView) -> bool {
        // The merge function panics in debug mode if we attempt to merge unrelated nodes
        // so we should prepare our test data so it doesn't happen
        b.id = a.id;

        let merged_a_b = {
            let mut a = a.clone();
            a.merge(b.clone());
            a
        };
        let merged_b_a = {
            b.merge(a);
            b
        };
        merged_a_b == merged_b_a
    }

    #[quickcheck]
    fn member_view_merge_is_idempotent(mut a: MemberView, b: MemberView) -> bool {
        // The merge function panics in debug mode if we attempt to merge unrelated nodes
        // so we should prepare our test data so it doesn't happen
        a.id = b.id;

        let merged_a_b = {
            a.merge(b.clone());
            a
        };
        let merged_a_b_b = {
            let mut merged_a_b = merged_a_b.clone();
            merged_a_b.merge(b);
            merged_a_b
        };
        merged_a_b == merged_a_b_b
    }

    impl Arbitrary for ClusterView {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut version_vector = VersionVector::default();
            let mut heartbeats = HashMap::new();
            // Generate members so that the ids field always matches the map key
            let members = Vec::<MemberView>::arbitrary(g)
                .into_iter()
                .map(|n| {
                    version_vector
                        .versions
                        .insert(n.id, n.state.as_ref().map_or(0, |s| s.version));
                    heartbeats.insert(n.id, n.state.as_ref().map_or(0, |s| s.heartbeat));
                    (n.id, n)
                })
                .collect();

            Self {
                known_members: members,
                version_vector,
                heartbeats,
            }
        }
    }

    impl Arbitrary for MemberView {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Self {
                id: NodeId::arbitrary(g),
                advertised_addr: Url::from_str("test:8080").unwrap(),
                state: Option::<MemberViewState>::arbitrary(g),
            }
        }
    }

    impl Arbitrary for MemberViewState {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Self {
                node_status: NodeStatus::arbitrary(g),
                version: u16::arbitrary(g),
                heartbeat: u64::arbitrary(g),
            }
        }
    }
}
