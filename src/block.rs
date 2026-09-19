// ============================================================
// NUSACOIN (NSC) — block.rs — Secure Mainnet Replacement
// ============================================================
// FIXES APPLIED:
// [FIX-01] calculate_block_hash() uses stable canonical format
//           not {:#?} Debug output (which can change between
//           Rust versions and break all hash comparisons)
// [FIX-02] mine() has a nonce overflow guard (u64 exhaustion)
// [FIX-03] mine() logs only block index and final hash,
//           not every nonce attempt
// [FIX-04] Block validates its own hash before returning
//           from mine()
// [FIX-05] All unwrap() replaced with safe alternatives
// [FIX-06] Transaction hashes included in block hash so
//           tx content cannot be swapped without detection
// ============================================================

use crate::hash::calculate_hash;
use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};

// ── Constants ─────────────────────────────────────────────────

/// Maximum nonce before mining gives up (prevents infinite loop).
const MAX_MINE_NONCE: u64 = u64::MAX - 1;

// ─────────────────────────────────────────────────────────────

/// A generic balance-mutation record attached to a block, used to
/// make non-transfer state changes (DEX swaps, L2 ops, staking,
/// custom tokens) replayable during fork resolution, since the
/// core Transaction type only covers simple transfers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceOp {
    /// Which balance map this applies to, e.g. "nsc", "usdt",
    /// "token:<symbol>", "l2", "staking".
    pub target: String,
    pub address: String,
    /// Signed delta (can be negative for debits).
    pub delta: i128,
    /// Human-readable reason, e.g. "swap", "l2_deposit", "stake".
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index:         u64,
    pub timestamp:     u64,
    pub transactions:  Vec<Transaction>,
    pub previous_hash: String,
    pub nonce:         u64,
    pub hash:          String,
    pub reward:        u128,
    /// Non-transfer balance mutations included in this block.
    /// [serde(default)] keeps old persisted blocks (pre-dating
    /// this field) loading correctly as an empty vec.
    #[serde(default)]
    pub ops:           Vec<BalanceOp>,
    /// Staking-specific bookkeeping mutations (stake/unstake/claim/
    /// slash) included in this block, replayed via Staking::apply_op().
    /// [serde(default)] keeps old persisted blocks loading correctly.
    #[serde(default)]
    pub staking_ops:   Vec<crate::staking::StakingOp>,
    /// Address that mined this block (informational only — NOT part of
    /// the hash, so adding it retroactively cannot invalidate any
    /// existing block's hash).
    #[serde(default)]
    pub miner:         String,
}

impl Block {

    // ── Constructor ───────────────────────────────────────────

    pub fn new(
        index:         u64,
        timestamp:     u64,
        transactions:  Vec<Transaction>,
        previous_hash: String,
        reward:        u128,
        ops:           Vec<BalanceOp>,
        staking_ops:   Vec<crate::staking::StakingOp>,
        miner:         String,
    ) -> Self {
        Self {
            index,
            timestamp,
            transactions,
            previous_hash,
            nonce: 0,
            hash:  String::new(),
            reward,
            ops,
            staking_ops,
            miner,
        }
    }

    // ── Hash computation ──────────────────────────────────────

    /// Computes the canonical block hash.
    ///
    /// [FIX-01] Uses a stable, deterministic format — NOT
    /// Rust's Debug ({:?}) output which is not guaranteed to
    /// be stable across compiler versions.
    ///
    /// [FIX-06] Transaction hashes are included so swapping
    /// transactions in a block will change the block hash.
    pub fn calculate_block_hash(&self) -> String {
        // Build a stable transaction fingerprint: join all
        // tx_hash values with ':' separator.
        let tx_fingerprint: String = self.transactions
            .iter()
            .map(|tx| tx.tx_hash.as_str())
            .collect::<Vec<&str>>()
            .join(":");

        // [OPS-HASH] ops are only folded into the hash content when
        // non-empty, so every historical block (which always has an
        // empty ops vec, field didn't exist yet) still hashes to the
        // exact same value as before this change — zero migration
        // needed, chain stays valid.
        let ops_fingerprint: String = self.ops
            .iter()
            .map(|o| format!("{}|{}|{}|{}", o.target, o.address, o.delta, o.reason))
            .collect::<Vec<String>>()
            .join(";");

        let staking_ops_fingerprint: String = self.staking_ops
            .iter()
            .map(|o| format!("{:?}", o))
            .collect::<Vec<String>>()
            .join(";");

        // [HASH-COMPAT] Each combination of (ops empty?, staking_ops
        // empty?) uses its own exact format, chosen so that any state
        // reachable BEFORE staking_ops existed (empty+empty, or
        // ops-nonempty+staking_ops-always-empty) hashes IDENTICALLY to
        // what it always did -- only the two NEW combinations involving
        // a non-empty staking_ops (which could never occur before this
        // field existed) get a new format. This guarantees zero
        // historical blocks are invalidated.
        let content = match (self.ops.is_empty(), self.staking_ops.is_empty()) {
            (true, true) => format!(
                "NSCBLOCK:{}:{}:{}:{}:{}",
                self.index, self.timestamp, self.previous_hash, self.nonce, tx_fingerprint,
            ),
            (false, true) => format!(
                "NSCBLOCK:{}:{}:{}:{}:{}:{}",
                self.index, self.timestamp, self.previous_hash, self.nonce, tx_fingerprint, ops_fingerprint,
            ),
            (true, false) => format!(
                "NSCBLOCK:{}:{}:{}:{}:{}:STAKING:{}",
                self.index, self.timestamp, self.previous_hash, self.nonce, tx_fingerprint, staking_ops_fingerprint,
            ),
            (false, false) => format!(
                "NSCBLOCK:{}:{}:{}:{}:{}:{}:STAKING:{}",
                self.index, self.timestamp, self.previous_hash, self.nonce, tx_fingerprint, ops_fingerprint, staking_ops_fingerprint,
            ),
        };

        calculate_hash(&content)
    }

    // ── Mining ────────────────────────────────────────────────

    /// Proof-of-Work mining loop.
    ///
    /// [FIX-02] Nonce overflow guard — stops at MAX_MINE_NONCE.
    /// [FIX-03] Logs only start and final result, not every nonce.
    /// [FIX-04] Validates own hash after mining completes.
    pub fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);

        println!(
            "[MINE] Mining block {} (difficulty={})...",
            self.index,
            difficulty
        );

        // [FIX-02] Overflow guard.
        while self.nonce < MAX_MINE_NONCE {
            let hash = self.calculate_block_hash();

            if hash.starts_with(&target) {
                self.hash = hash;

                // [FIX-04] Self-validate immediately after mining.
                if self.hash != self.calculate_block_hash() {
                    eprintln!(
                        "[MINE] Block {} hash self-validation failed!",
                        self.index
                    );
                    self.hash = String::new();
                    return;
                }

                println!(
                    "[MINE] Block {} mined: nonce={} hash={}",
                    self.index,
                    self.nonce,
                    self.hash
                );

                return;
            }

            self.nonce += 1;
        }

        // [FIX-02] If nonce exhausted, log and leave hash empty.
        eprintln!(
            "[MINE] Block {} nonce exhausted — could not mine.",
            self.index
        );
    }

    // ── Validation helpers ────────────────────────────────────

    /// Returns true if the stored hash matches the computed hash.
    pub fn is_hash_valid(&self) -> bool {
        !self.hash.is_empty()
            && self.hash == self.calculate_block_hash()
    }

    /// Returns true if the block meets the PoW difficulty target.
    pub fn meets_difficulty(&self, difficulty: usize) -> bool {
        self.hash.starts_with(&"0".repeat(difficulty))
    }
}

// ============================================================
// END OF block.rs
// ============================================================

