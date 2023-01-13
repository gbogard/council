use std::collections::HashSet;

use url::Url;

use self::{
    convergence_monitor::ConvergenceMonitor, failure_detector::FailureDetector, views::ClusterView,
};

pub mod convergence_monitor;
pub mod failure_detector;
pub mod version_vector;
pub mod views;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone)]
pub struct Cluster {
    pub cluster_view: ClusterView,
    pub peer_nodes: HashSet<Url>,
    pub failure_detector: FailureDetector,
    pub convergence_monitor: ConvergenceMonitor,
}
