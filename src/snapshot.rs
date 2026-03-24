//! Unified snapshot from all three blockchain nodes.

use serde::{Deserialize, Serialize};

/// Unified snapshot from Dynex, Qubic, and Quai nodes.
///
/// All fields default to 0.0/false so missing data is benign.
/// The `InterpolatorBank` smooths these to 10Hz for SNN consumption.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TripleSnapshot {
    // ── Dynex ──────────────────────────────────────────────────────────────
    pub dynex_hashrate_mh: f32,
    pub dynex_power_w: f32,
    pub dynex_gpu_temp_c: f32,
    /// True when a Dynex share was accepted this polling cycle.
    pub dynex_share_found: bool,

    // ── Qubic ──────────────────────────────────────────────────────────────
    pub qubic_tick_number: u64,
    pub qubic_epoch: u32,
    pub qubic_tick_rate: f32,
    pub qubic_epoch_progress: f32,
    pub qu_price_usd: f32,
    /// True when a Qubic computation solution was validated this cycle.
    pub qubic_solution_found: bool,

    // ── Quai ───────────────────────────────────────────────────────────────
    pub quai_gas_price: f32,
    pub quai_tx_count: u32,
    pub quai_block_utilization: f32,
    pub quai_staking_ratio: f32,
    /// True when a Quai block was mined this polling cycle.
    pub quai_block_mined: bool,

    // ── Neuraxon (optional neuromodulator telemetry) ────────────────────────
    pub neuraxon_dopamine: f32,
    pub neuraxon_serotonin: f32,
    pub neuraxon_its: f32,
}
