use std::iter::Sum;

use ringbuffer::{
    ConstGenericRingBuffer, RingBuffer, RingBufferExt, RingBufferRead, RingBufferWrite,
};

/// A circular buffer of observations (i.e. numbers) that computes
/// the mean, sum and variance of the last observed values (bounded by CAP, which defaults to 98)
pub struct Statistics<const CAP: usize = 98> {
    observations: ConstGenericRingBuffer<f32, CAP>,
    pub sum: f32,
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            observations: ConstGenericRingBuffer::new(),
            sum: 0.0,
        }
    }

    pub fn add_observation(&mut self, observation: f32) {
        if let Some(oldest_observation) = self.observations.dequeue() {
            self.sum -= oldest_observation;
        }
        self.observations.push(observation);
        self.sum += observation;
    }

    pub fn mean(&self) -> f32 {
        self.sum as f32 / self.observations.len() as f32
    }

    pub fn variance(&self, mean: f32) -> f32 {
        let count = self.observations.len();
        self.observations
            .iter()
            .map(|o| {
                let diff = mean - *o;
                diff * diff
            })
            .sum::<f32>()
            / count as f32
    }
}
