use std::{
    collections::{HashMap, LinkedList},
    time::{Duration, Instant},
};

use crate::node::NodeId;

const HEARTBEAT_INTERVALS_WINDOW_SIZE: u32 = 255;

/// The [FailureDetector] estimates the probability of a node being unreachable.
///
/// Every time a node gossips with the running node, its last heartbeat value is recorded, and
/// the last intervals between each hearteat are recorded in a bounded window.
/// This in turn lets us implement a Phi-accrual failure detector (Hayashibara, N., Défago, X., Yared, R., & Katayama, T. (2004)),
/// a failure detector popularized by Akka on the JVM.
///
/// To quote Akka's documentation:
/// > The suspicion level of failure is represented by a value called phi.
/// > The basic idea of the phi failure detector is to express the value of phi on a scale that is dynamically adjusted to reflect current network conditions.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone)]
pub struct FailureDetector {
    members: HashMap<NodeId, FailureDetectorMember>,
    pub phi_treshold: f64,
}

impl FailureDetector {
    pub(crate) fn new() -> Self {
        Self {
            members: HashMap::new(),
            phi_treshold: 8.0,
        }
    }

    pub(crate) fn record_heartbeat(&mut self, node_id: NodeId, last_heartbeat: u64) {
        let last_heartbeat_received_at = Instant::now();
        match self.members.get_mut(&node_id) {
            Some(member) if member.last_heartbeat < last_heartbeat => {
                let elapsed_time = member.last_heartbeat_received_at.elapsed();
                let received_heartbeats_since_last_record: u32 =
                    (last_heartbeat - member.last_heartbeat) as u32;
                let mean_heartbeat_time = elapsed_time / received_heartbeats_since_last_record;
                for _ in 0..received_heartbeats_since_last_record {
                    member.insert_interval(mean_heartbeat_time);
                }
                member.refresh_stats();
            }
            None => {
                let member = FailureDetectorMember {
                    last_heartbeat,
                    last_heartbeat_received_at,
                    heartbeats_intervals: LinkedList::new(),
                    hearbeats_interval_std_dev: None,
                    heartbeats_intervals_mean: None,
                };
                self.members.insert(node_id, member);
            }
            _ => (),
        }
    }

    pub fn is_live(&self, node_id: NodeId, now: Instant) -> bool {
        self.members
            .get(&node_id)
            .and_then(|n| n.phi(now))
            .map(|phi| phi < self.phi_treshold)
            .unwrap_or(false)
    }

    pub fn live_members<'a>(&'a self, now: Instant) -> impl Iterator<Item = NodeId> + 'a {
        self.members.iter().filter_map(move |(node_id, node)| {
            if node.phi(now).unwrap_or(0.0) < self.phi_treshold {
                Some(*node_id)
            } else {
                None
            }
        })
    }

    pub fn unreachable_members<'a>(&'a self, now: Instant) -> impl Iterator<Item = NodeId> + 'a {
        self.members.iter().filter_map(move |(node_id, node)| {
            if node.phi(now).unwrap_or(0.0) >= self.phi_treshold {
                Some(*node_id)
            } else {
                None
            }
        })
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
struct FailureDetectorMember {
    last_heartbeat: u64,
    #[cfg_attr(feature = "serde", serde(skip))]
    last_heartbeat_received_at: Instant,
    heartbeats_intervals: LinkedList<Duration>,
    heartbeats_intervals_mean: Option<Duration>,
    hearbeats_interval_std_dev: Option<Duration>,
}

impl FailureDetectorMember {
    fn refresh_stats(&mut self) {
        let count = self.heartbeats_intervals.len();
        if count > 0 {
            let sum: Duration = self.heartbeats_intervals.iter().sum();
            let mean_heartbeat_duration = sum / count as u32;
            let mean_heartbeat_duration_f64 = mean_heartbeat_duration.as_secs_f64();
            let variance_f64: f64 = self
                .heartbeats_intervals
                .iter()
                .map(|interval| {
                    let diff = mean_heartbeat_duration_f64 - interval.as_secs_f64();
                    diff * diff
                })
                .sum::<f64>()
                / count as f64;
            let std_dev = Duration::from_secs_f64(variance_f64.sqrt());

            self.heartbeats_intervals_mean = Some(mean_heartbeat_duration);
            self.hearbeats_interval_std_dev = Some(std_dev);
        } else {
            self.heartbeats_intervals_mean = None;
            self.hearbeats_interval_std_dev = None;
        }
    }

    fn phi(&self, now: Instant) -> Option<f64> {
        match (
            self.heartbeats_intervals_mean,
            self.hearbeats_interval_std_dev,
        ) {
            (Some(mean), Some(std_dev)) => {
                let x = (now - self.last_heartbeat_received_at).as_secs_f64();
                let mean = mean.as_secs_f64();
                let std_dev = std_dev.as_secs_f64();
                let cdf_at_x = 0.5 * (mean - x) / (std_dev * std::f64::consts::SQRT_2);
                let phi = 1.0 - cdf_at_x.log10();
                Some(phi)
            }
            _ => None,
        }
    }

    fn insert_interval(&mut self, interval: Duration) {
        if self.heartbeats_intervals.len() == HEARTBEAT_INTERVALS_WINDOW_SIZE as usize {
            self.heartbeats_intervals.pop_front();
        }
        self.heartbeats_intervals.push_back(interval);
    }
}
