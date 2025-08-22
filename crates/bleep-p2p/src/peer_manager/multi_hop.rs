pub struct MultiHopRouter;

impl MultiHopRouter {
    pub fn new() -> Self { MultiHopRouter }
    pub fn route(&self, _data: &[u8], _destination: &str) -> bool { true }
}
