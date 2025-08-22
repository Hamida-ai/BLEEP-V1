pub struct AIDetector;

impl AIDetector {
    pub fn new() -> Self { AIDetector }
    pub fn calculate_score(&self, _id: &str) -> f64 { 1.0 }
}
