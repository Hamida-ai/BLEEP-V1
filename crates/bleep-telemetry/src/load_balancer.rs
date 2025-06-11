use log::info;
use crate::modules::energy::energy_module::EnergyMonitor;
use crate::p2p::P2PNetwork;

/// A load balancer that uses energy efficiency metrics to trigger balancing actions
pub struct LoadBalancer {
    p2p_network: P2PNetwork,
    /// The load threshold is the minimum acceptable energy efficiency score (0-100)
    load_threshold: f64,
}

impl LoadBalancer {
    /// Initializes a new LoadBalancer with the given P2P network interface and threshold
    pub fn new(p2p_network: P2PNetwork, load_threshold: f64) -> Self {
        Self {
            p2p_network,
            load_threshold,
        }
    }

    /// Evaluates the energy efficiency score and triggers load balancing actions if needed
    pub fn balance_load(&self, energy_monitor: &EnergyMonitor) {
        if energy_monitor.energy_efficiency_score < self.load_threshold {
            info!(
                "LoadBalancer: Energy efficiency score ({:.2}) is below threshold ({:.2}). Initiating load balancing.",
                energy_monitor.energy_efficiency_score, self.load_threshold
            );
            // Broadcast a load balancing signal to peers.
            // This assumes that the P2PNetwork has a method `broadcast_load_balance_signal`
            // which would be implemented as part of the overall system.
            self.p2p_network.broadcast_load_balance_signal(energy_monitor.energy_usage);
            info!("LoadBalancer: Load balancing action executed.");
        } else {
            info!(
                "LoadBalancer: Energy efficiency score ({:.2}) is optimal. No load balancing required.",
                energy_monitor.energy_efficiency_score
            );
        }
    }
}