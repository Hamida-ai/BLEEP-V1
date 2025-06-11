//! AI-Powered Decision Module for BLEEP Blockchain
//! Handles AI-driven automation, risk analysis, and governance optimizations.

pub mod decision_engine;
pub mod machine_learning;
#[cfg(test)]
mod tests;

// 🔍 AI Decision Engine: Core logic for AI-powered governance and security automation
pub use decision_engine::{AIDecisionEngine, DecisionOutcome};

// 🤖 Machine Learning Models: Neural networks, risk assessment, and automated blockchain optimizations
pub use machine_learning::{RiskAnalyzer, AIOptimizer};

/// AI Decision Module Struct
pub struct AIDecisionModule {
    pub engine: AIDecisionEngine,
    pub risk_analyzer: RiskAnalyzer,
    pub optimizer: AIOptimizer,
}

impl AIDecisionModule {
    /// Initializes the AI Decision Module with pre-trained models
    pub fn new() -> Self {
        Self {
            engine: AIDecisionEngine::new(),
            risk_analyzer: RiskAnalyzer::new(),
            optimizer: AIOptimizer::new(),
        }
    }

    /// Executes AI-driven governance decisions
    pub fn execute_decision(&self, input_data: &str) -> DecisionOutcome {
        self.engine.process_decision(input_data)
    }

    /// Runs AI-powered risk assessment for fraud detection and transaction security
    pub fn analyze_risk(&self, transaction_data: &str) -> f64 {
        self.risk_analyzer.assess_risk(transaction_data)
    }

    /// Optimizes blockchain parameters dynamically based on AI predictions
    pub fn optimize_network(&self) -> bool {
        self.optimizer.optimize()
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_ai_decision_execution() {
        let ai_module = AIDecisionModule::new();
        let result = ai_module.execute_decision("Governance Vote: Increase Staking Rewards");
        assert!(matches!(result, DecisionOutcome::Approved | DecisionOutcome::Rejected));
    }

    #[test]
    fn test_risk_analysis() {
        let ai_module = AIDecisionModule::new();
        let risk_score = ai_module.analyze_risk("Transaction: High-Value Cross-Chain Swap");
        assert!(risk_score >= 0.0 && risk_score <= 1.0);
    }

    #[test]
    fn test_network_optimization() {
        let ai_module = AIDecisionModule::new();
        assert!(ai_module.optimize_network());
    }
}
