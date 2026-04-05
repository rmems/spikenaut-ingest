//! Consensus Reward — converts blockchain events into dopamine spikes for SNN reward-modulated learning.
//!
//! ## Key Insight
//!
//! When a significant blockchain event occurs, it can serve as a strong reward signal for SNN learning, indicating an optimal state at that moment. This is more valuable than synthetic reward functions.
//!
//! ## Reward Hierarchy
//!
//! | Event               | Magnitude | Frequency  | Analogy        |
//! |---------------------|-----------|------------|----------------|
//! | Qubic event         | 1.0       | Rare       | Finding food   |
//! | Quai event          | 0.8       | Occasional | Successful hunt|
//! | Dynex event         | 0.3       | Frequent   | Foraging       |
//!
//! The dopamine spike decays with τ = 0.5s (5 steps at 10Hz),
//! so the E-prop eligibility trace captures a ~1.5s credit window around the event.
//!
//! ## Usage
//!
//! ```rust
//! use spikenaut_ingest::{ConsensusRewardTracker, TripleSnapshot};
//!
//! let mut tracker = ConsensusRewardTracker::new();
//! let mut snap = TripleSnapshot::default();
//!
//! // Simulate a Dynex event
//! snap.dynex_event = true;
//! let dopamine = tracker.update(&snap);
//! println!("Dopamine boost: {:.3}", dopamine);
//! ```

use crate::snapshot::TripleSnapshot;

/// Dopamine decay per 10Hz step: τ = 0.5s → α = exp(-0.1/0.5) = 0.8187
const DOPAMINE_DECAY: f32 = 0.8187;

/// Maximum combined reward (synthetic + dopamine).
/// Allows transient overshoot to make consensus events salient.
pub const REWARD_CEILING: f32 = 1.5;

const DYNEX_EVENT_REWARD: f32  = 0.3;
const QUAI_EVENT_REWARD: f32   = 0.8;
const QUBIC_EVENT_REWARD: f32 = 1.0;

/// Tracks dopamine level from consensus reward events.
///
/// Zero-alloc, stack-resident. Call `update()` every 10Hz step.
pub struct ConsensusRewardTracker {
    /// Current dopamine level [0.0, 1.0].
    dopamine: f32,
    /// Cumulative event counters for logging/diagnostics.
    pub dynex_events: u64,
    pub quai_events: u64,
    pub qubic_events: u64,
}

impl ConsensusRewardTracker {
    pub fn new() -> Self {
        Self { dopamine: 0.0, dynex_events: 0, quai_events: 0, qubic_events: 0 }
    }

    /// Process one 10Hz step: detect events and decay dopamine.
    ///
    /// Returns the current dopamine boost to add to `compute_reward()`.
    pub fn update(&mut self, snap: &TripleSnapshot) -> f32 {
        if snap.qubic_event {
            self.dopamine = QUBIC_EVENT_REWARD;
            self.qubic_events += 1;
        } else if snap.quai_event {
            self.dopamine = self.dopamine.max(QUAI_EVENT_REWARD);
            self.quai_events += 1;
        } else if snap.dynex_event {
            self.dopamine = self.dopamine.max(DYNEX_EVENT_REWARD);
            self.dynex_events += 1;
        }

        // Exponential decay
        self.dopamine *= DOPAMINE_DECAY;
        if self.dopamine < 1e-4 { self.dopamine = 0.0; } // avoid denormals

        self.dopamine
    }

    /// Current dopamine level without advancing state.
    pub fn dopamine(&self) -> f32 { self.dopamine }

    /// Inject dopamine boost directly (e.g. from external reward signal).
    pub fn inject(&mut self, amount: f32) {
        self.dopamine = (self.dopamine + amount).min(1.0);
    }

    /// Apply dopamine boost to a synthetic reward, clamped to `REWARD_CEILING`.
    ///
    /// `R_total = clamp(R_synthetic + dopamine, 0.0, REWARD_CEILING)`
    pub fn boost_reward(&self, synthetic_reward: f32) -> f32 {
        (synthetic_reward + self.dopamine).clamp(0.0, REWARD_CEILING)
    }

    /// One-line status for dashboard display.
    pub fn status_line(&self) -> String {
        format!(
            "DA:{:.2} dynex:{} quai:{} qubic:{}",
            self.dopamine, self.dynex_events, self.quai_events, self.qubic_events
        )
    }
}

impl Default for ConsensusRewardTracker {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dopamine_decay() {
        let mut tracker = ConsensusRewardTracker::new();
        let mut snap = TripleSnapshot::default();

        snap.dynex_event = true;
        let d = tracker.update(&snap);
        assert!((d - DYNEX_EVENT_REWARD * DOPAMINE_DECAY).abs() < 0.01);

        snap.dynex_event = false;
        for _ in 0..50 { tracker.update(&snap); }
        assert!(tracker.dopamine() < 0.001, "should decay, got {}", tracker.dopamine());
    }

    #[test]
    fn test_qubic_overrides_dynex() {
        let mut tracker = ConsensusRewardTracker::new();
        let mut snap = TripleSnapshot::default();

        snap.dynex_event = true;
        tracker.update(&snap);
        snap.dynex_event = false;

        snap.qubic_event = true;
        tracker.update(&snap);
        assert!(tracker.dopamine() > 0.8, "Qubic should dominate");
    }

    #[test]
    fn test_boost_reward_ceiling() {
        let mut tracker = ConsensusRewardTracker::new();
        let mut snap = TripleSnapshot::default();

        snap.qubic_event = true;
        tracker.update(&snap);

        let boosted = tracker.boost_reward(0.9);
        assert!(boosted <= REWARD_CEILING);
        assert!(boosted > 1.0);
    }

    #[test]
    fn test_event_counters() {
        let mut tracker = ConsensusRewardTracker::new();
        let mut snap = TripleSnapshot::default();

        snap.dynex_event = true;
        tracker.update(&snap);
        tracker.update(&snap);
        assert_eq!(tracker.dynex_events, 2);

        snap.dynex_event = false;
        snap.quai_event = true;
        tracker.update(&snap);
        assert_eq!(tracker.quai_events, 1);
    }
}
