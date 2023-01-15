use std::collections::HashSet;

use url::Url;

pub use self::gossip_destinations::*;
use self::{
    convergence_monitor::ConvergenceMonitor, failure_detector::FailureDetector,
    version_vector::VersionVectorOffset, views::ClusterView,
};

pub mod convergence_monitor;
pub mod failure_detector;
pub mod version_vector;
pub mod views;

mod gossip_destinations;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone)]
pub struct Cluster {
    pub cluster_view: ClusterView,
    pub peer_nodes: HashSet<Url>,
    pub failure_detector: FailureDetector,
    pub convergence_monitor: ConvergenceMonitor,
}
