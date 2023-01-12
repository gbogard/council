use tokio::sync::watch;

use self::{failure_detector::FailureDetector, views::ClusterView};

pub mod failure_detector;
pub mod version_vector;
pub mod views;

pub struct Cluster {
    state: watch::Sender<State>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
struct State {
    cluster_view: ClusterView,
    failure_detector: FailureDetector,
}
