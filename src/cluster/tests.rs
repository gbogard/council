use std::{collections::HashSet, iter::once};

use quickcheck::Arbitrary;

use super::{
    failure_detector::FailureDetector,
    views::{ClusterView, MemberView},
    Cluster,
};

/// Generates an arbitrary cluster with at least one member
impl Arbitrary for Cluster {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let this_node = MemberView::arbitrary(g);
        let this_node_id = this_node.id;
        let this_advertised_url = this_node.advertised_addr.clone();
        let mut failure_detector = FailureDetector::new(this_node_id);
        let mut peer_nodes = HashSet::new();

        // Generate a cluster view with at least one member
        let cluster_view = ClusterView::from_members(
            Vec::<MemberView>::arbitrary(g)
                .into_iter()
                .chain(once(this_node)),
        );
        for (id, member) in cluster_view.known_members.iter() {
            if *id != this_node_id && peer_nodes.len() < 2 {
                peer_nodes.insert(member.advertised_addr.clone());
            }
            if *id != this_node_id {
                if let Some(state) = &member.state {
                    failure_detector.record_heartbeat(*id, state.heartbeat);
                }
            }
        }

        Cluster {
            this_node_id,
            this_advertised_url,
            failure_detector,
            cluster_view,
            unknwon_peer_nodes: peer_nodes.clone(),
            peer_nodes,
        }
    }
}

#[quickcheck]
fn arbitrary_cluster_test(cluster: Cluster) {
    assert!(cluster.cluster_view.known_members.len() > 0);
    for (id, member) in cluster
        .cluster_view
        .known_members
        .iter()
        .filter(|(id, _)| **id != cluster.this_node_id)
    {
        if let Some(state) = &member.state {
            assert_eq!(
                cluster
                    .failure_detector
                    .members
                    .get(&id)
                    .map(|m| m.last_heartbeat),
                Some(state.heartbeat)
            );
        }
    }
}
