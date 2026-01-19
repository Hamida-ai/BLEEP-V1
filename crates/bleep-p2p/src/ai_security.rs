use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct PeerScoring {
    scores: Mutex<HashMap<String, f64>>,
}

impl PeerScoring {
    pub fn new() -> Self {
        Self {
            scores: Mutex::new(HashMap::new()),
        }
    }

    pub fn calculate_score(&self, id: &str) -> f64 {
        let scores = self.scores.lock().unwrap();
        *scores.get(id).unwrap_or(&1.0)
    }

    pub fn update_score(&self, id: &str, score: f64) {
        let mut scores = self.scores.lock().unwrap();
        scores.insert(id.to_string(), score);
    }
}

#[derive(Debug)]
pub struct SybilDetector {
    suspicious: Mutex<HashMap<String, bool>>,
}

impl SybilDetector {
    pub fn new() -> Self {
        Self {
            suspicious: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_suspicious(&self, peer_id: &str) -> bool {
        let suspicious = self.suspicious.lock().unwrap();
        *suspicious.get(peer_id).unwrap_or(&false)
    }

    pub fn mark_suspicious(&self, peer_id: &str) {
        let mut suspicious = self.suspicious.lock().unwrap();
        suspicious.insert(peer_id.to_string(), true);
    }
}
