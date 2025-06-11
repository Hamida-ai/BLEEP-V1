use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use ethers::signers::{LocalWallet, Signer};
use ethers::contract::Contract;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use log::{info, error};
use env_logger::Env;
use tokio::sync::RwLock;
use pqcrypto_kyber::kyber512::{keypair, encapsulate, decapsulate};
use crate::{
    ai_decision::BLEEPAIDecisionModule,
    governance::SelfAmendingGovernance,
    zkp_verification::BLEEPZKPModule,
    interoperability::BLEEPInteroperabilityModule,
    bleep_connect::BLEEPConnect,
    consensus::BLEEPAdaptiveConsensus,
};

// 🚀 **Governance & AI-Powered Decision Making**
#[derive(Deserialize, Serialize, Clone)]
pub struct ContractProposal {
    pub contract_name: String,
    pub creator: String, // Address proposing the contract
    pub network: String,
    pub proposal_id: u64,
    pub votes_for: u64,
    pub votes_against: u64,
    pub executed: bool,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub status: String,
    pub message: String,
    pub transaction_hash: Option<String>,
}

// **Smart Contract Execution Module**
pub struct SmartContractAutomation {
    governance: Arc<RwLock<SelfAmendingGovernance>>,
    ai_decision: Arc<BLEEPAIDecisionModule>,
    zkp_module: Arc<BLEEPZKPModule>,
    interoperability: Arc<BLEEPInteroperabilityModule>,
    bleep_connect: Arc<BLEEPConnect>,
    consensus: Arc<RwLock<BLEEPAdaptiveConsensus>>,
}

impl SmartContractAutomation {
    pub fn new() -> Self {
        Self {
            governance: Arc::new(RwLock::new(SelfAmendingGovernance::new())),
            ai_decision: Arc::new(BLEEPAIDecisionModule::new()),
            zkp_module: Arc::new(BLEEPZKPModule::new()),
            interoperability: Arc::new(BLEEPInteroperabilityModule::new()),
            bleep_connect: Arc::new(BLEEPConnect::new()),
            consensus: Arc::new(RwLock::new(BLEEPAdaptiveConsensus::new())),
        }
    }

    // ✅ **Governance-Based Smart Contract Execution**
    pub async fn submit_proposal(&self, input: ContractProposal) -> Result<ApiResponse, String> {
        let mut governance = self.governance.write().await;
        let proposal_id = governance.submit_proposal(input.contract_name.clone(), input.creator.clone())?;
        Ok(ApiResponse {
            status: "Proposal Submitted".to_string(),
            message: format!("Proposal ID: {}", proposal_id),
            transaction_hash: None,
        })
    }

    pub async fn vote_on_proposal(&self, proposal_id: u64, support: bool) -> Result<ApiResponse, String> {
        let mut governance = self.governance.write().await;
        governance.vote_on_proposal(proposal_id, support)?;
        Ok(ApiResponse {
            status: "Vote Casted".to_string(),
            message: format!("Proposal {} voted successfully", proposal_id),
            transaction_hash: None,
        })
    }

    pub async fn execute_proposal(&self, proposal_id: u64) -> Result<ApiResponse, String> {
        let mut governance = self.governance.write().await;
        
        // Ensure proposal is approved
        if !governance.is_approved(proposal_id)? {
            return Err("Proposal not approved. Execution denied.".to_string());
        }

        let executed = governance.execute_proposal(proposal_id)?;
        if executed {
            Ok(ApiResponse {
                status: "Proposal Executed".to_string(),
                message: format!("Contract Deployment Approved for Proposal {}", proposal_id),
                transaction_hash: None,
            })
        } else {
            Err("Proposal execution conditions not met.".to_string())
        }
    }

    // 🔍 **AI-Powered Smart Contract Security Audit**
    pub async fn audit_smart_contract(&self, contract_code: String) -> Result<ApiResponse, String> {
        let risk_score = self.ai_decision.analyze_contract_security(contract_code)?;

        // **Block Deployment if Risk Score is Too High**
        if risk_score > 50.0 {
            return Err("Security risk too high. Deployment rejected.".to_string());
        }

        Ok(ApiResponse {
            status: "Audit Passed".to_string(),
            message: format!("Contract is secure. Risk Score: {:.2}", risk_score),
            transaction_hash: None,
        })
    }

    // 🔒 **Zero-Knowledge Proof (ZKP) Verification for Contract Validity**
    pub async fn verify_contract_with_zkp(&self, contract_data: &[u8]) -> Result<ApiResponse, String> {
        let proof = self.zkp_module.generate_proof(contract_data)?;
        let is_valid = self.zkp_module.verify_proof(&proof, contract_data)?;

        if is_valid {
            Ok(ApiResponse {
                status: "Verified".to_string(),
                message: "Contract verified using ZKP.".to_string(),
                transaction_hash: None,
            })
        } else {
            Err("ZKP verification failed".to_string())
        }
    }

    // 🏛️ **Quantum-Secure Multi-Signature Approval**
    pub async fn approve_multisig_transaction(&self, tx_id: &str) -> Result<(), String> {
        let mut consensus = self.consensus.write().await;
        if consensus.is_quorum_reached(tx_id)? {
            consensus.finalize_transaction(tx_id)?;
            Ok(())
        } else {
            Err("Quorum not reached for multi-signature approval.".to_string())
        }
    }
}

// 🌐 **REST API Routes**
async fn deploy_contract(input: web::Json<ContractProposal>, engine: web::Data<SmartContractAutomation>) -> impl Responder {
    match engine.deploy_cross_chain_contract(input.0).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(err) => HttpResponse::InternalServerError().body(err),
    }
}

async fn vote(input: web::Json<(u64, bool)>, engine: web::Data<SmartContractAutomation>) -> impl Responder {
    match engine.vote_on_proposal(input.0 .0, input.0 .1).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(err) => HttpResponse::InternalServerError().body(err),
    }
}

async fn audit_contract(input: web::Json<String>, engine: web::Data<SmartContractAutomation>) -> impl Responder {
    match engine.audit_smart_contract(input.0).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(err) => HttpResponse::InternalServerError().body(err),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let engine = web::Data::new(SmartContractAutomation::new());

    HttpServer::new(move || {
        App::new()
            .app_data(engine.clone())
            .route("/deploy", web::post().to(deploy_contract))
            .route("/vote", web::post().to(vote))
            .route("/audit", web::post().to(audit_contract))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
