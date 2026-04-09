/// Minimal local stub for zksnarks setup used by the root binary.

/// Generate a dummy proof system proving/verifying key pair for devnet startup.
pub fn devnet_setup() -> (Vec<u8>, Vec<u8>) {
    (vec![0u8; 64], vec![1u8; 64])
}

/// Generate a dummy batch circuit key pair for devnet startup.
pub fn devnet_batch_setup() -> (Vec<u8>, Vec<u8>) {
    (vec![2u8; 64], vec![3u8; 64])
}
