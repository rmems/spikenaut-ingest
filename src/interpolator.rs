//! State-Space Interpolator — upsamples slow blockchain signals to 10Hz.
//!
//! ## Background
//!
//! Blockchain data arrives at wildly different rates:
//! - Dynex miner stats:  ~1 Hz
//! - Qubic ticks:        ~0.2–0.5 Hz (2–5 second intervals)
//! - Quai blocks:        ~0.08 Hz (12-second block time)
//!
//! An SNN supervisor running at 10 Hz sees identical values for 20–120 consecutive
//! steps then a sharp discontinuity — creating phantom spikes that drown real signal.
//!
//! ## Solution
//!
//! First-order exponential state-space interpolation (Zero-Order Hold + EMA):
//!
//! ```text
//! x[k+1] = α · x[k] + (1 - α) · u[k]
//! α = exp(-Δt / τ),  Δt = 0.1s (10Hz),  τ = signal time constant
//! ```
//!
//! Properties:
//! - Converges to true value within ~3τ of a step change
//! - Zero heap allocation (all state on stack)
//! - Monotone: no overshoot or ringing (unlike Kalman or spline)
//! - Graceful degradation: if no new observation arrives, state
//!   exponentially decays toward the last known value

/// Time constant classes for different signal dynamics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalClass {
    /// Fast hardware signals (power, temp, hashrate): τ = 0.3s
    /// Converges in ~1 second. Tracks GPU transients without aliasing.
    Hardware,
    /// Medium blockchain signals (Qubic ticks, gas price): τ = 1.0s
    /// Converges in ~3 seconds. Smooths 2–5s tick jitter.
    Blockchain,
    /// Slow epoch signals (Quai blocks, epoch progress): τ = 3.0s
    /// Converges in ~9 seconds. Matches 12s block cadence.
    SlowChain,
}

impl SignalClass {
    /// Smoothing factor α = exp(-Δt / τ) for 10Hz (Δt = 0.1s).
    /// Pre-computed to avoid `exp()` in the hot path.
    pub fn alpha(self) -> f32 {
        match self {
            SignalClass::Hardware   => 0.7165, // exp(-0.1 / 0.3)
            SignalClass::Blockchain => 0.9048, // exp(-0.1 / 1.0)
            SignalClass::SlowChain  => 0.9672, // exp(-0.1 / 3.0)
        }
    }

    /// Time constant τ in seconds.
    pub fn tau_secs(self) -> f32 {
        match self {
            SignalClass::Hardware   => 0.3,
            SignalClass::Blockchain => 1.0,
            SignalClass::SlowChain  => 3.0,
        }
    }
}

/// Single-channel first-order state-space interpolator.
///
/// Zero-alloc, `Copy`-able, designed to live in a fixed-size array.
///
/// # Example
/// ```rust
/// use spikenaut_ingest::{ChannelInterpolator, SignalClass};
///
/// let mut ch = ChannelInterpolator::new(SignalClass::Blockchain);
/// ch.observe(100.0);          // new RPC reading
/// let v = ch.step();          // 10Hz tick
/// assert!(v > 0.0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ChannelInterpolator {
    state: f32,
    observation: f32,
    alpha: f32,
    initialized: bool,
}

impl ChannelInterpolator {
    /// Create a new interpolator for the given signal class.
    pub const fn new(class: SignalClass) -> Self {
        let alpha = match class {
            SignalClass::Hardware   => 0.7165,
            SignalClass::Blockchain => 0.9048,
            SignalClass::SlowChain  => 0.9672,
        };
        Self { state: 0.0, observation: 0.0, alpha, initialized: false }
    }

    /// Create an interpolator with a custom alpha (advanced use).
    pub fn with_alpha(alpha: f32) -> Self {
        Self { state: 0.0, observation: 0.0, alpha: alpha.clamp(0.0, 1.0), initialized: false }
    }

    /// Feed a new raw observation from an RPC response (irregular cadence).
    pub fn observe(&mut self, value: f32) {
        self.observation = value;
        if !self.initialized {
            self.state = value; // snap to avoid cold-start ramp
            self.initialized = true;
        }
    }

    /// Advance one 10Hz step. Returns the interpolated value.
    ///
    /// `x[k+1] = α · x[k] + (1 − α) · u[k]`
    #[inline(always)]
    pub fn step(&mut self) -> f32 {
        if !self.initialized { return 0.0; }
        self.state = self.alpha * self.state + (1.0 - self.alpha) * self.observation;
        self.state
    }

    /// Current interpolated value without advancing state.
    #[inline(always)]
    pub fn value(&self) -> f32 { self.state }

    /// True if at least one observation has been received.
    pub fn is_initialized(&self) -> bool { self.initialized }

    /// Reset state to zero (e.g. after a reconnection).
    pub fn reset(&mut self) {
        self.state = 0.0;
        self.observation = 0.0;
        self.initialized = false;
    }
}

// ── Multi-channel bank ────────────────────────────────────────────────────────

/// Number of interpolated channels in the default triple-bridge bank.
///
/// Channel layout:
/// ```text
///  0: Dynex hashrate (MH/s)         — Hardware
///  1: Dynex power (W)               — Hardware
///  2: Dynex GPU temp (°C)           — Hardware
///  3: Qubic tick rate (ticks/s)     — Blockchain
///  4: Qubic epoch progress [0,1]    — SlowChain
///  5: QU price (USD)                — Blockchain
///  6: Quai gas price (gwei)         — SlowChain
///  7: Quai tx count                 — SlowChain
///  8: Quai block utilization [0,1]  — SlowChain
///  9: Neuraxon dopamine [0,1]       — Blockchain
/// 10: Neuraxon serotonin [0,1]      — Blockchain
/// 11: Neuraxon ITS (normalized)     — Blockchain
/// ```
pub const NUM_BRIDGE_CHANNELS: usize = 12;

/// Multi-channel interpolator bank for all triple-bridge signals.
///
/// Fits in ~192 bytes on the stack; suitable for supervisor loops.
pub struct InterpolatorBank {
    channels: [ChannelInterpolator; NUM_BRIDGE_CHANNELS],
}

impl Default for InterpolatorBank {
    fn default() -> Self { Self::new() }
}

impl InterpolatorBank {
    pub fn new() -> Self {
        Self {
            channels: [
                ChannelInterpolator::new(SignalClass::Hardware),    // 0 dynex hashrate
                ChannelInterpolator::new(SignalClass::Hardware),    // 1 dynex power
                ChannelInterpolator::new(SignalClass::Hardware),    // 2 dynex temp
                ChannelInterpolator::new(SignalClass::Blockchain),  // 3 qubic tick rate
                ChannelInterpolator::new(SignalClass::SlowChain),   // 4 qubic epoch
                ChannelInterpolator::new(SignalClass::Blockchain),  // 5 qu price
                ChannelInterpolator::new(SignalClass::SlowChain),   // 6 quai gas
                ChannelInterpolator::new(SignalClass::SlowChain),   // 7 quai tx count
                ChannelInterpolator::new(SignalClass::SlowChain),   // 8 quai block util
                ChannelInterpolator::new(SignalClass::Blockchain),  // 9 neuraxon dopamine
                ChannelInterpolator::new(SignalClass::Blockchain),  // 10 neuraxon serotonin
                ChannelInterpolator::new(SignalClass::Blockchain),  // 11 neuraxon its
            ],
        }
    }

    /// Feed raw observations from a [`TripleSnapshot`](crate::TripleSnapshot).
    pub fn observe(&mut self, snap: &crate::snapshot::TripleSnapshot) {
        self.channels[0].observe(snap.dynex_hashrate_mh);
        self.channels[1].observe(snap.dynex_power_w);
        self.channels[2].observe(snap.dynex_gpu_temp_c);
        self.channels[3].observe(snap.qubic_tick_rate);
        self.channels[4].observe(snap.qubic_epoch_progress);
        self.channels[5].observe(snap.qu_price_usd);
        self.channels[6].observe(snap.quai_gas_price);
        self.channels[7].observe(snap.quai_tx_count as f32);
        self.channels[8].observe(snap.quai_block_utilization);
        self.channels[9].observe(snap.neuraxon_dopamine.clamp(0.0, 1.0));
        self.channels[10].observe(snap.neuraxon_serotonin.clamp(0.0, 1.0));
        self.channels[11].observe((snap.neuraxon_its / 2000.0).clamp(0.0, 1.0));
    }

    /// Advance all channels one 10Hz step.
    pub fn step(&mut self) -> [f32; NUM_BRIDGE_CHANNELS] {
        let mut out = [0.0f32; NUM_BRIDGE_CHANNELS];
        for (i, ch) in self.channels.iter_mut().enumerate() {
            out[i] = ch.step();
        }
        out
    }

    /// Current values without advancing state.
    pub fn values(&self) -> [f32; NUM_BRIDGE_CHANNELS] {
        let mut out = [0.0f32; NUM_BRIDGE_CHANNELS];
        for (i, ch) in self.channels.iter().enumerate() {
            out[i] = ch.value();
        }
        out
    }

    /// Reset all channels (e.g. after reconnection).
    pub fn reset(&mut self) {
        for ch in &mut self.channels { ch.reset(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_on_first_observe() {
        let mut ch = ChannelInterpolator::new(SignalClass::Hardware);
        assert_eq!(ch.step(), 0.0);
        ch.observe(100.0);
        assert_eq!(ch.value(), 100.0);
    }

    #[test]
    fn test_convergence() {
        let mut ch = ChannelInterpolator::new(SignalClass::Hardware);
        ch.observe(0.0);
        ch.observe(1.0);
        for _ in 0..30 { ch.step(); }
        assert!((ch.value() - 1.0).abs() < 0.01, "got {}", ch.value());
    }

    #[test]
    fn test_monotone_no_overshoot() {
        let mut ch = ChannelInterpolator::new(SignalClass::Blockchain);
        ch.observe(0.0);
        ch.observe(1.0);
        let mut prev = 0.0;
        for _ in 0..50 {
            let v = ch.step();
            assert!(v >= prev);
            assert!(v <= 1.0 + 1e-6);
            prev = v;
        }
    }

    #[test]
    fn test_slow_chain_lags_at_1s() {
        let mut ch = ChannelInterpolator::new(SignalClass::SlowChain);
        ch.observe(0.0);
        ch.observe(1.0);
        for _ in 0..10 { ch.step(); }
        assert!(ch.value() < 0.5, "SlowChain lag at 1s: got {}", ch.value());
    }

    #[test]
    fn test_alpha_values() {
        let hw = (-0.1_f64 / 0.3).exp() as f32;
        assert!((SignalClass::Hardware.alpha() - hw).abs() < 0.001);
        let bc = (-0.1_f64 / 1.0).exp() as f32;
        assert!((SignalClass::Blockchain.alpha() - bc).abs() < 0.001);
        let sc = (-0.1_f64 / 3.0).exp() as f32;
        assert!((SignalClass::SlowChain.alpha() - sc).abs() < 0.001);
    }

    #[test]
    fn test_reset() {
        let mut ch = ChannelInterpolator::new(SignalClass::Hardware);
        ch.observe(50.0);
        ch.step();
        ch.reset();
        assert_eq!(ch.step(), 0.0);
        assert!(!ch.is_initialized());
    }
}
