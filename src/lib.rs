//! # spikenaut-ingest
//!
//! Multi-chain blockchain ingest with state-space interpolation for SNN supervisors.
//!
//! ## The Problem
//!
//! Blockchain data arrives at wildly different rates:
//! - Dynex miner stats:  ~1 Hz
//! - Qubic ticks:        ~0.2–0.5 Hz (2–5 second intervals)
//! - Quai blocks:        ~0.08 Hz (12-second block time)
//!
//! An SNN supervisor running at 10 Hz sees identical values for 20–120 steps,
//! then a sharp discontinuity — creating phantom spikes that drown real signal.
//!
//! ## The Solution
//!
//! First-order exponential state-space interpolation:
//! ```text
//! x[k+1] = α · x[k] + (1 - α) · u[k]
//! ```
//! where `α = exp(-Δt / τ)` and `τ` is tuned per signal class.
//!
//! ## Usage
//!
//! ```rust
//! use spikenaut_ingest::{ChannelInterpolator, SignalClass};
//!
//! let mut interp = ChannelInterpolator::new(SignalClass::Blockchain);
//!
//! // Feed a new observation from the RPC (irregular cadence)
//! interp.observe(42.0);
//!
//! // Step at 10 Hz to get smooth output
//! let smooth = interp.step();
//! println!("Smoothed value: {}", smooth);
//! ```

pub mod interpolator;
pub mod consensus_reward;
pub mod snapshot;

#[cfg(feature = "async")]
pub mod triple_bridge;

pub use interpolator::{ChannelInterpolator, InterpolatorBank, SignalClass};
pub use consensus_reward::{ConsensusRewardTracker, REWARD_CEILING};
pub use snapshot::TripleSnapshot;
