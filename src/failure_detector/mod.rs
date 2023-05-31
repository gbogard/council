use std::{collections::HashMap, time::Instant};

use crate::entities::node::NodeId;

use self::stats::Statistics;

mod stats;

struct FailureDetector {
    heartbeat_monitor: HashMap<NodeId, MonitoredNode>,
}

struct MonitoredNode {
    last_seen_at: Instant,
    first_heartbeat: u64,
    last_heartbeat: u64,
    // heabeat inter-arrival durations in seconds
    heartbeats_inter_arrival_durations: Statistics,
    variance: f32,
}

impl MonitoredNode {
    #[inline]
    fn record_heartbeat(&mut self, last_heartbeat: u64) {
        if last_heartbeat < self.last_heartbeat {
            panic!("[MonitoredNode::record_heartbeat] called record_heartbeat with a heartbeat strictyl inferior to the last recorded heartbeat for this node");
        }
        if last_heartbeat == self.last_heartbeat {
            return;
        }

        let elapsed_heartbeats = last_heartbeat - self.last_heartbeat;
        let elapsed_duration = self.last_seen_at.elapsed();
        let mean_duration_per_heartbeat = elapsed_duration / elapsed_heartbeats as u32;
        for _ in 0..elapsed_heartbeats {
            self.heartbeats_inter_arrival_durations
                .add_observation(mean_duration_per_heartbeat.as_secs_f32());
        }
        self.last_heartbeat = last_heartbeat;
        self.last_seen_at = Instant::now();
    }
}
