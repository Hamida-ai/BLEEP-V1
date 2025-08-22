pub struct Kyber;
impl Kyber {
    pub fn encrypt(_data: &[u8]) -> Vec<u8> { Vec::new() }
    pub fn decrypt(_data: &[u8]) -> Option<Vec<u8>> { Some(Vec::new()) }
}

pub struct SphincsPlus;
impl SphincsPlus {
    pub fn sign(_data: &[u8]) -> Vec<u8> { Vec::new() }
}

pub struct Falcon;
impl Falcon {
    pub fn verify(_data: &[u8], _id: &str) -> bool { true }
}
