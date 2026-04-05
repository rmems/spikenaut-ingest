use spikenaut_ingest::{ChannelInterpolator, InterpolatorBank, SignalClass, TripleSnapshot};

fn main() {
    // Example 1: Single Channel Interpolation
    let mut interp = ChannelInterpolator::new(SignalClass::Blockchain);
    interp.observe(42.0); // Simulate a blockchain data point
    for _ in 0..10 {
        let smoothed = interp.step();
        println!("Smoothed value (single channel): {:.2}", smoothed);
    }

    // Example 2: Multi-Channel Interpolation with InterpolatorBank
    let mut bank = InterpolatorBank::new();
    let mut snapshot = TripleSnapshot::default();
    snapshot.dynex_hashrate_mh = 100.0; // Simulate Dynex data
    snapshot.qubic_tick_rate = 0.3;     // Simulate Qubic data
    snapshot.quai_gas_price = 25.0;     // Simulate Quai data
    bank.observe(&snapshot);
    for _ in 0..10 {
        let values = bank.step();
        println!("Smoothed values (multi-channel): {:?}", values);
    }
}
