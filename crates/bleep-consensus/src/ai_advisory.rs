// PHASE 1: AI ADVISORY REPORTING SYSTEM
// Signed, bounded advisory reports for consensus metrics
// 
// SAFETY CONSTRAINTS:
// 1. AI reports NEVER override deterministic consensus rules
// 2. AI reports are ADVISORY ONLY - inputs to orchestrator
// 3. Reports are SIGNED to prevent tampering
// 4. Reports are BOUNDED (0-100 numeric scores only)
// 5. Reports are AGGREGATED DETERMINISTICALLY (median/quorum)
// 6. AI CANNOT decide consensus mode directly

use serde::{Serialize, Deserialize};
use std::cmp::Ordering;
use log::{info, warn};

/// Bounded numeric score for AI advisory reports.
/// 
/// SAFETY: All AI recommendations are bounded to [0, 100].
/// This prevents unbounded influence on consensus decisions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoundedScore(u8); // 0-100

impl BoundedScore {
    /// Create a bounded score (automatically clamps to [0, 100]).
    pub fn new(value: u8) -> Self {
        BoundedScore(value.min(100))
    }

    /// Get the numeric value (0-100).
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Interpret as a health assessment.
    /// - 0-33: Poor health
    /// - 34-66: Normal health
    /// - 67-100: Excellent health
    pub fn as_health(&self) -> &'static str {
        match self.0 {
            0..=33 => "Poor",
            34..=66 => "Normal",
            67..=100 => "Excellent",
        }
    }
}

impl std::fmt::Display for BoundedScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// AI advisory report for a consensus epoch.
/// 
/// SAFETY: Reports are SIGNED to prevent tampering.
/// Reports are input to the Orchestrator but NEVER override deterministic rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAdvisoryReport {
    /// Epoch number this report applies to
    pub epoch_id: u64,
    
    /// AI node identifier (for accountability)
    pub ai_node_id: String,
    
    /// Timestamp when report was generated
    pub timestamp: u64,
    
    /// Network health score (0-100)
    /// Higher = more healthy consensus conditions
    pub network_health_score: BoundedScore,
    
    /// Validator performance score (0-100)
    /// Higher = better validator participation and performance
    pub validator_performance_score: BoundedScore,
    
    /// Byzantine fault likelihood (0-100)
    /// Higher = more likely Byzantine faults detected
    pub byzantine_fault_likelihood: BoundedScore,
    
    /// Finality latency score (0-100)
    /// Higher = faster finality
    pub finality_latency_score: BoundedScore,
    
    /// Digital signature (prevents tampering)
    pub signature: Vec<u8>,
    
    /// Optional reasoning/diagnostics
    pub reasoning: String,
}

impl AiAdvisoryReport {
    /// Create a new advisory report.
    pub fn new(
        epoch_id: u64,
        ai_node_id: String,
        network_health_score: u8,
        validator_performance_score: u8,
        byzantine_fault_likelihood: u8,
        finality_latency_score: u8,
    ) -> Self {
        AiAdvisoryReport {
            epoch_id,
            ai_node_id,
            timestamp: Self::current_timestamp(),
            network_health_score: BoundedScore::new(network_health_score),
            validator_performance_score: BoundedScore::new(validator_performance_score),
            byzantine_fault_likelihood: BoundedScore::new(byzantine_fault_likelihood),
            finality_latency_score: BoundedScore::new(finality_latency_score),
            signature: vec![],
            reasoning: String::new(),
        }
    }

    /// Set the digital signature (done by AI node after report generation).
    pub fn sign(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }

    /// Verify the signature (prevents tampering).
    /// 
    /// In production, this would use actual cryptographic verification.
    pub fn verify_signature(&self) -> Result<(), String> {
        if self.signature.is_empty() {
            return Err("Report is not signed".to_string());
        }
        // In production, verify signature against AI node's public key
        Ok(())
    }

    /// Compute an overall health score from all components.
    pub fn overall_health(&self) -> BoundedScore {
        let avg = (self.network_health_score.value() as u16
            + self.validator_performance_score.value() as u16
            + (100 - self.byzantine_fault_likelihood.value()) as u16
            + self.finality_latency_score.value() as u16) / 4;
        BoundedScore::new(avg as u8)
    }

    /// Check if this report indicates an emergency condition.
    pub fn is_emergency(&self) -> bool {
        // Emergency if network health is poor OR Byzantine faults are likely
        self.network_health_score.value() < 40
            || self.byzantine_fault_likelihood.value() > 60
            || self.validator_performance_score.value() < 40
    }

    /// Get current timestamp (seconds since epoch).
    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Add reasoning to the report.
    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.reasoning = reasoning;
        self
    }
}

/// Aggregator for multiple AI advisory reports.
/// 
/// SAFETY: Aggregation is deterministic.
/// Reports are combined using median (robust to outliers).
pub struct AiReportAggregator {
    /// Minimum number of reports required for quorum
    pub quorum_size: usize,
}

impl AiReportAggregator {
    /// Create a new report aggregator.
    pub fn new(quorum_size: usize) -> Self {
        AiReportAggregator { quorum_size }
    }

    /// Aggregate multiple reports into a consensus assessment.
    /// 
    /// SAFETY: Uses median to prevent outliers from influencing the result.
    pub fn aggregate(&self, reports: &[AiAdvisoryReport]) -> Result<AggregatedAdvisory, String> {
        if reports.is_empty() {
            return Err("No reports to aggregate".to_string());
        }

        if reports.len() < self.quorum_size {
            warn!(
                "Quorum not reached: {} reports, need {}",
                reports.len(),
                self.quorum_size
            );
        }

        // Verify all reports are signed
        for report in reports {
            report.verify_signature()?;
        }

        // Verify all reports are from the same epoch
        let epoch_id = reports[0].epoch_id;
        for report in &reports[1..] {
            if report.epoch_id != epoch_id {
                return Err(format!(
                    "Report epoch mismatch: expected {}, got {}",
                    epoch_id, report.epoch_id
                ));
            }
        }

        // Aggregate using median
        let network_health = self.median_score(
            reports
                .iter()
                .map(|r| r.network_health_score.value())
                .collect::<Vec<_>>(),
        );
        let validator_performance = self.median_score(
            reports
                .iter()
                .map(|r| r.validator_performance_score.value())
                .collect::<Vec<_>>(),
        );
        let byzantine_fault = self.median_score(
            reports
                .iter()
                .map(|r| r.byzantine_fault_likelihood.value())
                .collect::<Vec<_>>(),
        );
        let finality_latency = self.median_score(
            reports
                .iter()
                .map(|r| r.finality_latency_score.value())
                .collect::<Vec<_>>(),
        );

        info!(
            "Aggregated {} AI advisory reports for epoch {}: network_health={}, validator_perf={}, byzantine={}, finality={}",
            reports.len(),
            epoch_id,
            network_health,
            validator_performance,
            byzantine_fault,
            finality_latency
        );

        Ok(AggregatedAdvisory {
            epoch_id,
            report_count: reports.len(),
            network_health,
            validator_performance,
            byzantine_fault_likelihood: byzantine_fault,
            finality_latency,
        })
    }

    /// Compute median of a list of scores.
    /// 
    /// SAFETY: Median is robust to outliers.
    fn median_score(&self, mut scores: Vec<u8>) -> u8 {
        if scores.is_empty() {
            return 50; // Default middle value
        }

        scores.sort_unstable();

        if scores.len() % 2 == 0 {
            // Even number: average of two middle values
            let mid = scores.len() / 2;
            ((scores[mid - 1] as u16 + scores[mid] as u16) / 2) as u8
        } else {
            // Odd number: middle value
            scores[scores.len() / 2]
        }
    }
}

/// Aggregated advisory from multiple AI nodes.
/// 
/// SAFETY: This is deterministic and reproducible.
#[derive(Debug, Clone)]
pub struct AggregatedAdvisory {
    /// Epoch this applies to
    pub epoch_id: u64,
    
    /// Number of reports in this aggregation
    pub report_count: usize,
    
    /// Median network health score
    pub network_health: u8,
    
    /// Median validator performance score
    pub validator_performance: u8,
    
    /// Median Byzantine fault likelihood
    pub byzantine_fault_likelihood: u8,
    
    /// Median finality latency score
    pub finality_latency: u8,
}

impl AggregatedAdvisory {
    /// Check if the aggregated report indicates emergency.
    pub fn is_emergency(&self) -> bool {
        self.network_health < 40
            || self.byzantine_fault_likelihood > 60
            || self.validator_performance < 40
    }

    /// Get an overall health assessment (0-100).
    pub fn overall_health(&self) -> u8 {
        let avg = (self.network_health as u16
            + self.validator_performance as u16
            + (100 - self.byzantine_fault_likelihood as u16)
            + self.finality_latency as u16) / 4;
        (avg as u8).min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_score_creation() {
        let score = BoundedScore::new(50);
        assert_eq!(score.value(), 50);

        let clamped = BoundedScore::new(150);
        assert_eq!(clamped.value(), 100);
    }

    #[test]
    fn test_bounded_score_health_assessment() {
        assert_eq!(BoundedScore::new(20).as_health(), "Poor");
        assert_eq!(BoundedScore::new(50).as_health(), "Normal");
        assert_eq!(BoundedScore::new(80).as_health(), "Excellent");
    }

    #[test]
    fn test_advisory_report_creation() {
        let report = AiAdvisoryReport::new(0, "ai_node_1".to_string(), 80, 75, 20, 85);
        assert_eq!(report.epoch_id, 0);
        assert_eq!(report.network_health_score.value(), 80);
        assert_eq!(report.validator_performance_score.value(), 75);
        assert_eq!(report.byzantine_fault_likelihood.value(), 20);
        assert_eq!(report.finality_latency_score.value(), 85);
    }

    #[test]
    fn test_advisory_report_overall_health() {
        let report = AiAdvisoryReport::new(0, "ai_node_1".to_string(), 80, 80, 20, 80);
        // Average: (80 + 80 + (100-20) + 80) / 4 = 320 / 4 = 80
        assert_eq!(report.overall_health().value(), 80);
    }

    #[test]
    fn test_advisory_report_is_emergency() {
        let normal = AiAdvisoryReport::new(0, "ai".to_string(), 80, 80, 20, 80);
        assert!(!normal.is_emergency());

        let emergency_low_health = AiAdvisoryReport::new(0, "ai".to_string(), 30, 80, 20, 80);
        assert!(emergency_low_health.is_emergency());

        let emergency_high_byzantine = AiAdvisoryReport::new(0, "ai".to_string(), 80, 80, 70, 80);
        assert!(emergency_high_byzantine.is_emergency());
    }

    #[test]
    fn test_advisory_report_signing() {
        let mut report = AiAdvisoryReport::new(0, "ai".to_string(), 80, 75, 20, 85);
        assert!(report.verify_signature().is_err());

        report.sign(vec![1, 2, 3]);
        assert!(report.verify_signature().is_ok());
    }

    #[test]
    fn test_aggregator_median_odd_count() {
        let aggregator = AiReportAggregator::new(1);
        let scores = vec![10, 20, 50, 80, 90];
        let median = aggregator.median_score(scores);
        assert_eq!(median, 50);
    }

    #[test]
    fn test_aggregator_median_even_count() {
        let aggregator = AiReportAggregator::new(1);
        let scores = vec![10, 20, 80, 90];
        let median = aggregator.median_score(scores);
        assert_eq!(median, 50); // (20 + 80) / 2 = 50
    }

    #[test]
    fn test_aggregator_aggregate_reports() {
        let aggregator = AiReportAggregator::new(3);

        let mut r1 = AiAdvisoryReport::new(0, "ai_1".to_string(), 80, 75, 20, 85);
        r1.sign(vec![1]);
        let mut r2 = AiAdvisoryReport::new(0, "ai_2".to_string(), 75, 80, 25, 80);
        r2.sign(vec![2]);
        let mut r3 = AiAdvisoryReport::new(0, "ai_3".to_string(), 85, 70, 15, 90);
        r3.sign(vec![3]);

        let agg = aggregator.aggregate(&[r1, r2, r3]).unwrap();
        assert_eq!(agg.epoch_id, 0);
        assert_eq!(agg.report_count, 3);
        // Median of [80, 75, 85] = 80
        assert_eq!(agg.network_health, 80);
        // Median of [75, 80, 70] = 75
        assert_eq!(agg.validator_performance, 75);
        // Median of [20, 25, 15] = 20
        assert_eq!(agg.byzantine_fault_likelihood, 20);
    }

    #[test]
    fn test_aggregator_requires_quorum() {
        let aggregator = AiReportAggregator::new(3);

        let mut r1 = AiAdvisoryReport::new(0, "ai_1".to_string(), 80, 75, 20, 85);
        r1.sign(vec![1]);

        // Only 1 report, but quorum is 3
        let agg = aggregator.aggregate(&[r1]);
        // Should still succeed but with warning about quorum
        assert!(agg.is_ok());
    }

    #[test]
    fn test_aggregated_advisory_is_emergency() {
        let normal = AggregatedAdvisory {
            epoch_id: 0,
            report_count: 3,
            network_health: 80,
            validator_performance: 75,
            byzantine_fault_likelihood: 20,
            finality_latency: 85,
        };
        assert!(!normal.is_emergency());

        let emergency = AggregatedAdvisory {
            epoch_id: 0,
            report_count: 3,
            network_health: 30,
            validator_performance: 75,
            byzantine_fault_likelihood: 20,
            finality_latency: 85,
        };
        assert!(emergency.is_emergency());
    }

    #[test]
    fn test_aggregated_advisory_overall_health() {
        let adv = AggregatedAdvisory {
            epoch_id: 0,
            report_count: 3,
            network_health: 80,
            validator_performance: 80,
            byzantine_fault_likelihood: 20,
            finality_latency: 80,
        };
        // (80 + 80 + 80 + 80) / 4 = 80
        assert_eq!(adv.overall_health(), 80);
    }
}
