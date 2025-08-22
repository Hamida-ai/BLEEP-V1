use warp::Filter;
use tracing::{info, error};
use bleep-core::blockchain;
use bleep-wallet_core::wallet_core;
use bleep-ai::ai_assistant;
use bleep-governance::governance_engine;
use bleep-state::state_manager;
use bleep-pat::pat_engine;
use bleep-telemetry::telemetry;
use bleep-crypto::zkp_verification;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Deserialize)]
struct TxRequest {
    to: String,
    amount: f64,
}

#[derive(Deserialize)]
struct AiRequest {
    prompt: String,
}

#[derive(Deserialize)]
struct ZkpRequest {
    proof: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let health = warp::path("health")
        .map(|| warp::reply::json(&"BLEEP RPC is running"));

    let latest_block = warp::path!("block" / "latest")
        .and(warp::get())
        .and_then(|| async move {
            match blockchain::latest_block() {
                Ok(block) => Ok(warp::reply::json(&block)),
                Err(e) => {
                    error!("Error fetching latest block: {}", e);
                    Err(warp::reject::custom(e))
                },
            }
        });

    let send_tx = warp::path!("tx" / "send")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(|req: TxRequest| async move {
            match wallet_core::send_transaction(&req.to, req.amount) {
                Ok(tx_hash) => Ok(warp::reply::json(&tx_hash)),
                Err(e) => {
                    error!("Transaction failed: {}", e);
                    Err(warp::reject::custom(e))
                }
            }
        });

    let ai_ask = warp::path!("ai" / "ask")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(|req: AiRequest| async move {
            match ai_assistant::run_prompt(&req.prompt).await {
                Ok(response) => Ok(warp::reply::json(&response)),
                Err(e) => {
                    error!("AI request failed: {}", e);
                    Err(warp::reject::custom(e))
                }
            }
        });

    let zkp_verify = warp::path!("zkp" / "verify")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(|req: ZkpRequest| async move {
            match zkp_verification::verify_proof(&req.proof) {
                Ok(valid) => Ok(warp::reply::json(&valid)),
                Err(e) => {
                    error!("ZKP verification failed: {}", e);
                    Err(warp::reject::custom(e))
                }
            }
        });

    let pat_predict = warp::path!("pat" / "predict")
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(|params| async move {
            if let Some(input) = params.get("input") {
                match pat_engine::predict(input) {
                    Ok(result) => Ok(warp::reply::json(&result)),
                    Err(e) => {
                        error!("PAT prediction failed: {}", e);
                        Err(warp::reject::custom(e))
                    }
                }
            } else {
                Err(warp::reject::not_found())
            }
        });

    let telemetry_report = warp::path!("telemetry" / "report")
        .and(warp::get())
        .and_then(|| async move {
            match telemetry::report_metrics() {
                Ok(_) => Ok(warp::reply::json(&"Telemetry reported")),
                Err(e) => {
                    error!("Telemetry failed: {}", e);
                    Err(warp::reject::custom(e))
                }
            }
        });

    let routes = health
        .or(latest_block)
        .or(send_tx)
        .or(ai_ask)
        .or(zkp_verify)
        .or(pat_predict)
        .or(telemetry_report)
        .with(warp::log("bleep_rpc"));

    info!("🚀 BLEEP RPC server running at http://127.0.0.1:8080");
    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}            } else {
                Err(warp::reject::not_found())
            }
        });

    let telemetry_report = warp::path!("telemetry" / "report")
        .and(warp::get())
        .and_then(|| async move {
            match telemetry::report_metrics() {
                Ok(_) => Ok(warp::reply::json(&"Telemetry reported")),
                Err(e) => {
                    error!("Telemetry failed: {}", e);
                    Err(warp::reject::custom(e))
                }
            }
        });

    let routes = health
        .or(latest_block)
        .or(send_tx)
        .or(ai_ask)
        .or(zkp_verify)
        .or(pat_predict)
        .or(telemetry_report)
        .with(warp::log("bleep_rpc"));

    info!("🚀 BLEEP RPC server running at http://127.0.0.1:8080");
    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
