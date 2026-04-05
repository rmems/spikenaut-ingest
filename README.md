<p align="center">
  <img src="docs/logo.png" width="220" alt="Spikenaut">
</p>

<h1 align="center">spikenaut-ingest</h1>
<p align="center">Multi-chain blockchain data ingest with state-space interpolation for SNN supervisors</p>

<p align="center">
  <a href="https://crates.io/crates/spikenaut-ingest"><img src="https://img.shields.io/crates/v/spikenaut-ingest" alt="crates.io"></a>
  <a href="https://docs.rs/spikenaut-ingest"><img src="https://docs.rs/spikenaut-ingest/badge.svg" alt="docs.rs"></a>
  <img src="https://img.shields.io/badge/license-GPL--3.0-orange" alt="GPL-3.0">
</p>

---

Blockchain data arrives at heterogeneous rates — Dynex ~1 Hz, Qubic ~0.2 Hz, Quai
~0.08 Hz. Without interpolation an SNN running at 10 Hz sees step discontinuities
that create phantom spikes drowning real signal. This crate fixes that with
first-order state-space interpolation: `x[k+1] = α·x[k] + (1−α)·u[k]`.

## Features

- `ChannelInterpolator` — single-channel IIR smoother, α tuned per `SignalClass`
- `InterpolatorBank` — 12-channel bank (~192 bytes on stack, zero allocation)
- `SignalClass` — `Hardware` (α=0.72), `Blockchain` (α=0.90), `SlowChain` (α=0.97)
- `ConsensusRewardTracker` — EMA-smoothed composite reward from multi-chain activity
- `TripleSnapshot` — unified Dynex/Qubic/Quai observation struct

## Core Responsibilities

- **Acquire raw inputs** from sensors, system telemetry, and optional external feeds.
- **Normalize & sanitize** values into stable numeric ranges.
- **Map named fields** into a deterministic channel vector (`[f32; N]` ABI).
- **Buffer & align** samples in time (timestamps, sample rates, windows).
- **Export snapshots** in a compact, testable format for encoders & simulators.
- **Provide utilities** such as migration helpers, golden fixtures, and CI smoke examples.

## Why It Matters for SNN / LLM Fusion

- **Deterministic inputs** let encoders pre-allocate and map channels to neurons without runtime guessing.
- **Normalization** guarantees consistent value ranges across machines and datasets.
- **Fixed ordering** (the 12-channel layout) acts as a contract every repo can rely on, simplifying fusion and hardware export.
- **Separation of concerns** keeps ingestion pure; hardware/ML logic lives in other crates, making each repo easier to review and license-clean.


- `ChannelInterpolator` — single-channel IIR smoother, α tuned per `SignalClass`
- `InterpolatorBank` — 12-channel bank (~192 bytes on stack, zero allocation)
- `SignalClass` — `Hardware` (α=0.72), `Blockchain` (α=0.90), `SlowChain` (α=0.97)
- `ConsensusRewardTracker` — EMA-smoothed composite reward from multi-chain activity
- `TripleSnapshot` — unified Dynex/Quai/Qubic observation struct

## Installation

```toml
spikenaut-ingest = "0.1"

# With async HTTP ingest (tokio + reqwest)
spikenaut-ingest = { version = "0.1", features = ["async"] }
```

## Quick Start

```rust
use spikenaut_ingest::{ChannelInterpolator, SignalClass};

let mut interp = ChannelInterpolator::new(SignalClass::Blockchain);

// Feed observations as they arrive (irregular cadence, ~0.2 Hz)
interp.observe(42.5);

// Step at 10 Hz — returns smoothed value without discontinuities
for _ in 0..50 {
    let smooth = interp.step();
    println!("{:.4}", smooth);
}
```

## 12-Channel Bank

```rust
use spikenaut_ingest::InterpolatorBank;

let mut bank = InterpolatorBank::default();
// Channels 0–3: Hardware (Dynex data, α=0.72)
// Channels 4–7: Blockchain (Qubic data, α=0.90)
// Channels 8–11: SlowChain (Quai data, α=0.97)
bank.channels[4].observe(95.0);   // Qubic data rate
let smooth = bank.channels[4].step();
```

## Background

The `α` values are derived from Zero-Order Hold discretization:
`α = exp(−Δt/τ)` where `Δt = 0.1s` (10 Hz step) and `τ` is the natural
time constant of each signal class (Franklin et al. 2019; Kálmán 1960).

## Part of the Spikenaut Ecosystem

| Library | Purpose |
|---------|---------|
| [spikenaut-spine](https://github.com/rmems/spikenaut-spine) | ZMQ wire protocol to Julia brain |
| [spikenaut-telemetry](https://github.com/rmems/spikenaut-telemetry) | GPU/CPU hardware telemetry |
| [SpikenautSignals.jl](https://github.com/rmems/SpikenautSignals.jl) | Time-series feature extraction |

## Provenance

Extracted from Eagle-Lander, the author's own private neuromorphic GPU supervisor
repository (closed-source). Source: `ingest/` Rust modules feeding 12-channel
blockchain telemetry into a 65,536-neuron LSM at 10 Hz in production.

## License

GPL-3.0-or-later
