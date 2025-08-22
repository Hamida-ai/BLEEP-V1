pub struct EncryptedRoute;

impl EncryptedRoute {
    pub fn new() -> Self { EncryptedRoute }
    pub fn encrypt(&self, _data: &[u8]) -> Vec<u8> { Vec::new() }
}
