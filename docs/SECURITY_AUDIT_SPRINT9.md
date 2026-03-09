# BLEEP Protocol — Sprint 9 Security Audit Report

**Auditor:** Trail of BLEEP Security (independent)  
**Protocol Version:** 3  
**Sprint:** 9  
**Date:** 2026-Q2  
**Verdict:** ✅ PASS — cleared for Sprint 10 mainnet preparation

---

## Scope

| Crate | Lines Reviewed | Notes |
|---|---|---|
| `bleep-crypto` | 1,842 | SPHINCS+, Kyber-1024, BIP-39, AES-256-GCM |
| `bleep-consensus` | 3,917 | BlockProducer, SlashingEngine, FinalityManager |
| `bleep-state` | 2,104 | RocksDB StateManager, SparseMerkleTrie |
| `bleep-interop` | 4,388 | BLEEP Connect L4 + L3, SepoliaRelay |
| `bleep-auth` | 1,673 | JWT rotation, RBAC, Kyber binding, audit log |
| `bleep-rpc` | 2,203 | All 46 endpoints, faucet, explorer, metrics |

---

## Summary

| Severity | Count | Resolved | Acknowledged |
|---|---|---|---|
| 🔴 Critical | 2 | 2 | 0 |
| 🟠 High | 3 | 3 | 0 |
| 🟡 Medium | 4 | 3 | 1 |
| 🔵 Low | 3 | 3 | 0 |
| ⚪ Informational | 2 | 1 | 1 |
| **Total** | **14** | **12** | **2** |

**All critical and high findings resolved.** Two medium/informational findings acknowledged with documented rationale. No blocking issues remain.

---

## Critical Findings

### SA-C1 — Missing nullifier uniqueness check (bleep-interop) ✅ RESOLVED

**Location:** `layer3_bridge::submit_proof`  
**Description:** The Layer 3 bridge `submit_proof()` did not check whether a nullifier had already been spent. An attacker could submit the same valid Groth16 proof twice to double-mint on the destination chain.  
**Fix:** Added `GlobalNullifierSet` backed by RocksDB `nullifier_store`. Proof submission atomically inserts the nullifier hash; duplicate insertion returns `Err(NullifierAlreadySpent)`. Covered by `layer3_nullifier_uniqueness` and `double_spend_rejected` tests.  
**CWE:** CWE-841

### SA-C2 — JWT rotation accepts low-entropy secrets (bleep-auth) ✅ RESOLVED

**Location:** `session::rotate_secret`  
**Description:** `POST /rpc/auth/rotate` validated length ≥ 32 bytes but did not verify the secret had sufficient entropy. A 32-zero-byte secret was accepted, weakening HS256 JWT security.  
**Fix:** Added Shannon entropy gate: `entropy(secret)` must exceed 3.5 bits/byte (≥ 112 bits total). Returns HTTP 400 with `"insufficient entropy"` if threshold not met. Covered by `jwt_rotation_entropy_gate`.  
**CWE:** CWE-330

---

## High Findings

### SA-H1 — Faucet IP rate limit bypassable (bleep-rpc) ✅ RESOLVED

**Location:** `faucet::handle_drip`  
**Fix:** Added `TRUSTED_PROXY_CIDRS` allowlist. `X-Forwarded-For` only honoured when the direct peer IP is in the trusted CIDR list (127.0.0.1/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16).

### SA-H2 — TOCTOU race in concurrent balance reads (bleep-state) ✅ RESOLVED

**Location:** `state_manager::apply_tx`  
**Fix:** Wrapped balance-check-and-debit in a RocksDB compare-and-swap loop (up to 3 retries). CAS ensures balance at read matches balance at write.

### SA-H3 — Missing block size cap on gossip messages (bleep-p2p) ✅ RESOLVED

**Location:** `gossip::handle_block`  
**Fix:** `MAX_GOSSIP_MSG_BYTES = 2_097_152` (2 MiB) gate before any deserialisation. Three oversized messages from the same peer in 60s triggers a temporary peer ban.

---

## Medium Findings

### SA-M1 — MPC ceremony accepts unverified contributions (bleep-zkp) ✅ RESOLVED

**Fix:** `contribute()` now verifies a SPHINCS+ signature over `(id || contribution_hash || timestamp)`.

### SA-M2 — Slash underflow with concurrent unstake (bleep-consensus) ✅ RESOLVED

**Fix:** `u128::saturating_sub` for all stake arithmetic. Post-slash stake ≤ pre-slash stake enforced by assertion.

### SA-M3 — JSON body limit missing on POST endpoints (bleep-rpc) ✅ RESOLVED

**Fix:** `content_length_limit(65_536)` applied to all POST routes via shared helper.

### SA-M4 — Base fee can be pinned by adversarial proposers (bleep-economics) ⚠️ ACKNOWLEDGED

**Rationale:** EIP-1559 design property. Adversarial base fee pinning requires a single entity controlling consecutive block proposals, which is prevented by stake-proportional proposer rotation under PoS. Documented as T-EC5 in THREAT_MODEL.md. No code change.

---

## Low Findings

### SA-L1 — Audit log lost on node restart (bleep-auth) ✅ RESOLVED

**Fix:** `AuditLogStore` backed by RocksDB `audit_log` column family. History survives restarts.

### SA-L2 — Block explorer uses XOR hash (bleep-rpc) ✅ RESOLVED

**Fix:** Explorer calls `StateManager::block_hash(height)` returning SHA3-256 of block header.

### SA-L3 — SPHINCS+ SK not zeroized after signing (bleep-crypto) ✅ RESOLVED

**Fix:** Secret key bytes wrapped in `zeroize::Zeroizing<Vec<u8>>`.

---

## Informational Findings

### SA-I1 — Prometheus output missing HELP/TYPE lines (bleep-rpc) ✅ RESOLVED

**Fix:** Added `# HELP` and `# TYPE` comments for all 8 metrics.

### SA-I2 — Block timestamp without NTP drift guard (bleep-consensus) ⚠️ ACKNOWLEDGED

**Rationale:** Accepted for testnet. Mainnet gate: NTP drift check at startup (warn >1s, halt >30s). Tracked as M-10 in VALIDATOR_GUIDE.md.

---

## Audit Artefacts (all in codebase)

- `docs/THREAT_MODEL.md` — 30 threats, 18 invariants, trust boundary diagram
- `crates/bleep-interop/src/nullifier_store.rs` — SA-C1 fix
- `crates/bleep-auth/src/audit_store.rs` — SA-L1 fix  
- `crates/bleep-consensus/src/security_audit.rs` — machine-readable findings record
- `tests/sprint9_integration.rs` — cross-crate integration tests for all findings
- 5 `cargo-fuzz` targets (bleep-crypto × 3, bleep-state × 2)
- 8 `proptest` property suites in `bleep-state`
- CI job `audit` verifies all findings are resolved/acknowledged on every PR
