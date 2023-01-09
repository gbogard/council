use tokio::sync::watch;

use self::{failure_detector::FailureDetector, views::ClusterView};

pub mod failure_detector;
pub mod views;

pub struct Cluster {
    state: watch::Sender<State>,
}

struct State {
    cluster_view: ClusterView,
    failure_detector: FailureDetector,
}
