use std::collections::{HashMap, HashSet};

#[cfg(test)]
use quickcheck::Arbitrary;

use crate::node::NodeId;

/// A [VersionVector] stores associates [node ids](NodeId) with a last seen version.
///
/// Each runnning node keeps [its own local view of the cluster](crate::cluster::views), and with that view, a version vector,
/// that tells us, for each of the running node's peers, what is the last version of that peer, as seen by the running node.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct VersionVector {
    pub versions: HashMap<NodeId, u16>,
}

impl VersionVector {
    pub(crate) fn record_version(&mut self, node_id: NodeId, version: u16) {
        if let Some(existing_version) = self.versions.get_mut(&node_id) {
            *existing_version = std::cmp::max(*existing_version, version);
        } else {
            self.versions.insert(node_id, version);
        }
    }
}

/// A [VersionVectorOffset] lets us compare two [version vectors](VersionVector) and tells us
/// if the right-hand side version vector is lagging behing the left-hand side, and if so, which nodes
/// exactly have newer version that have not yet been tracked by the RHS vector.
///
/// Implementation-wise: the comparison function goes trough every node/version pair
/// in the LHS vector and comapres it with its RHS counterpart. If the RHS version for a node is strictly inferior to
/// its LHS counterpart, or if it is absent altogether from the RHS, we mark it as "behind" the LHS
pub(crate) struct VersionVectorOffset<'lhs, 'rhs> {
    lhs: &'lhs VersionVector,
    rhs: &'rhs VersionVector,
    /// A set of nodes whose latest version has not yet been observed by the RHS
    /// A non-empty set indicates that the RHS version vector is lagging behind.
    pub(crate) behind_lhs: HashSet<NodeId>,
}

impl<'lhs, 'rhs> VersionVectorOffset<'lhs, 'rhs> {
    pub(crate) fn of(lhs: &'lhs VersionVector, rhs: &'rhs VersionVector) -> Self {
        let mut behind_lhs = HashSet::new();
        for (node_id, lhs_version) in &lhs.versions {
            match rhs.versions.get(node_id) {
                Some(rhs_version) if rhs_version < lhs_version => {
                    behind_lhs.insert(*node_id);
                }
                None => {
                    behind_lhs.insert(*node_id);
                }
                _ => (),
            }
        }
        Self {
            lhs,
            rhs,
            behind_lhs,
        }
    }
}

#[cfg(test)]
impl Arbitrary for VersionVector {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        VersionVector {
            versions: HashMap::<NodeId, u16>::arbitrary(g),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use url::Url;

    use crate::{
        cluster::version_vector::{VersionVector, VersionVectorOffset},
        node::NodeId,
    };

    #[quickcheck]
    pub fn test_version_vector_record_version(mut version_vector: VersionVector) {
        let node_id = NodeId::from_url(&Url::parse("http://test.com").unwrap(), SystemTime::now());
        assert_eq!(version_vector.versions.get(&node_id), None);
        version_vector.record_version(node_id, 1);
        assert_eq!(version_vector.versions.get(&node_id), Some(&1));
        version_vector.record_version(node_id, 2);
        assert_eq!(version_vector.versions.get(&node_id), Some(&2));
        version_vector.record_version(node_id, 1);
        assert_eq!(version_vector.versions.get(&node_id), Some(&2));
    }

    #[quickcheck]
    pub fn test_version_vector_offset_identical(version_vector: VersionVector) {
        assert!(VersionVectorOffset::of(&version_vector, &version_vector)
            .behind_lhs
            .is_empty());
    }

    #[test]
    pub fn test_version_vector_offset() {
        let now = SystemTime::now();
        let node_id_a = NodeId::from_url(&Url::parse("http://a.com").unwrap(), now);
        let node_id_b = NodeId::from_url(&Url::parse("http://b.com").unwrap(), now);
        let node_id_c = NodeId::from_url(&Url::parse("http://c.com").unwrap(), now);
        let node_id_d = NodeId::from_url(&Url::parse("http://d.com").unwrap(), now);

        let a = VersionVector {
            versions: vec![(node_id_a, 1), (node_id_b, 3), (node_id_c, 3)]
                .into_iter()
                .collect(),
        };
        let b = VersionVector {
            versions: vec![
                (node_id_a, 1),
                (node_id_b, 2),
                (node_id_c, 4),
                (node_id_d, 1),
            ]
            .into_iter()
            .collect(),
        };
        let offset = VersionVectorOffset::of(&a, &b);
        assert_eq!(offset.behind_lhs, vec![node_id_b].into_iter().collect());

        let offset = VersionVectorOffset::of(&b, &a);
        assert_eq!(
            offset.behind_lhs,
            vec![node_id_c, node_id_d].into_iter().collect()
        );
    }
}
