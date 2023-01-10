use tokio::sync::watch;

use self::{failure_detector::FailureDetector, views::ClusterView};

pub mod failure_detector;
pub mod views;

#[cfg(test)]
mod views_tests;

pub struct Cluster {
    state: watch::Sender<State>,
}

struct State {
    cluster_view: ClusterView,
    failure_detector: FailureDetector,
}
