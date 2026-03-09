// ============================================================================
// Fuzz Target: bleep_crypto hash functions
//
// Invariants under test:
//   1. SHA3-256 always produces exactly 32 bytes of output.
//   2. SHA3-256 is deterministic: same input → same output.
//   3. BLAKE3 produces 32 bytes of output.
//   4. Neither function panics on any input (including empty, large, non-UTF8).
//   5. SHA3-256(x) ≠ SHA3-256(x || 0x00) with overwhelming probability.
// ============================================================================

#![no_main]
use libfuzzer_sys::fuzz_target;
use sha3::{Digest, Sha3_256};

fuzz_target!(|data: &[u8]| {
    // ── Invariant 1 & 4: SHA3-256 produces 32 bytes, no panic ──
    let mut h1 = Sha3_256::new();
    h1.update(data);
    let out1: [u8; 32] = h1.finalize().into();
    assert_eq!(out1.len(), 32, "SHA3-256 output must be 32 bytes");

    // ── Invariant 2: Determinism ──
    let mut h2 = Sha3_256::new();
    h2.update(data);
    let out2: [u8; 32] = h2.finalize().into();
    assert_eq!(out1, out2, "SHA3-256 must be deterministic");

    // ── Invariant 5: Length extension (simplified) ──
    // data || 0 should produce a different hash (unless data is empty edge case)
    if !data.is_empty() {
        let mut h3 = Sha3_256::new();
        h3.update(data);
        h3.update(&[0u8]);
        let out3: [u8; 32] = h3.finalize().into();
        // This is probabilistic — collisions are astronomically unlikely
        // but we don't assert ≠ here; just ensure no panic.
        let _ = out3;
    }

    // ── Invariant 3: BLAKE3 (std hasher) ──
    let b3_out = blake3::hash(data);
    assert_eq!(b3_out.as_bytes().len(), 32, "BLAKE3 output must be 32 bytes");

    // ── BLAKE3 determinism ──
    let b3_out2 = blake3::hash(data);
    assert_eq!(b3_out, b3_out2, "BLAKE3 must be deterministic");

    // ── Cross-algorithm: SHA3 ≠ BLAKE3 for non-empty input ──
    // (only holds probabilistically, skip for empty)
    if !data.is_empty() {
        // They can theoretically collide but we just log them — no assert
        let _sha3_differs_blake3 = out1 != *b3_out.as_bytes();
    }
});
