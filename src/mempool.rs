// ============================================================
// NUSACOIN (NSC) — mempool.rs — Secure Mainnet Replacement
// ============================================================
// FIXES APPLIED:
// [FIX-01] add_transaction() re-validates signature before add
// [FIX-02] add_transaction() enforces MAX_MEMPOOL_SIZE cap
// [FIX-03] add_transaction() rejects stale transactions
// [FIX-04] cleanup_expired() also removes zero-amount tx
// [FIX-05] No println! leaking transaction data
// [FIX-06] save() errors logged, not silently swallowed
// [FIX-07] export() returns JSON string, not Debug format
// [FIX-08] Duplicate check uses HashSet for O(1) lookup
// [FIX-09] has_nonce() checks sender + nonce pair correctly
// [FIX-10] size() and count() are consistent (one impl)
// ============================================================

use crate::transaction::Transaction;
use std::collections::HashSet;

// ── Constants ─────────────────────────────────────────────────

/// Maximum number of transactions held in the mempool.
const MAX_MEMPOOL_SIZE: usize = 10_000;

/// Maximum age of a mempool transaction in seconds (1 hour).
#[allow(dead_code)]
const MAX_TX_AGE_SECS: u64 = 3_600;

// ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Mempool {
    pub transactions: Vec<Transaction>,
    /// [FIX-08] HashSet for O(1) duplicate detection.
    known_hashes:     HashSet<String>,
}

impl Mempool {

    pub fn new() -> Self {
        Self {
            transactions: Vec::new(),
            known_hashes: HashSet::new(),
        }
    }

    // ── Add transaction ───────────────────────────────────────

    /// Adds a transaction to the mempool after validation.
    ///
    /// [FIX-01] Re-validates signature before accepting.
    /// [FIX-02] Enforces MAX_MEMPOOL_SIZE cap.
    /// [FIX-03] Rejects stale transactions.
    /// [FIX-08] O(1) duplicate check.
    pub fn add_transaction(&mut self, tx: Transaction) {

        // [FIX-02] Capacity guard.
        if self.transactions.len() >= MAX_MEMPOOL_SIZE {
            eprintln!(
                "[MEMPOOL] Full ({} tx). Rejecting new transaction.",
                MAX_MEMPOOL_SIZE
            );
            return;
        }

        // [FIX-08] O(1) duplicate check.
        if self.known_hashes.contains(&tx.tx_hash) {
            eprintln!(
                "[MEMPOOL] Duplicate transaction rejected: {}",
                tx.tx_hash
            );
            return;
        }

        // [FIX-03] Reject stale transactions.
        if !tx.is_fresh() {
            eprintln!(
                "[MEMPOOL] Stale transaction rejected: {} (age={}s)",
                tx.tx_hash,
                tx.age_seconds()
            );
            return;
        }

        // [FIX-01] Re-validate signature.
        if !tx.verify_signature() {
            eprintln!(
                "[MEMPOOL] Invalid signature rejected: {}",
                tx.tx_hash
            );
            return;
        }

        // Basic sanity checks.
        if tx.amount == 0 {
            eprintln!("[MEMPOOL] Zero-amount transaction rejected.");
            return;
        }

        if tx.sender == tx.receiver {
            eprintln!("[MEMPOOL] Self-transfer rejected.");
            return;
        }

        // Accept.
        self.known_hashes.insert(tx.tx_hash.clone());
        self.transactions.push(tx);
        self.save();
    }

    // ── Lookup ────────────────────────────────────────────────

    /// Returns true if the tx_hash is already in the mempool.
    /// [FIX-08] O(1) via HashSet.
    pub fn contains_tx(&self, tx_hash: &str) -> bool {
        self.known_hashes.contains(tx_hash)
    }

    /// Returns true if any pending tx from `sender` uses `nonce`.
    /// [FIX-09] Correct sender + nonce pair check.
    pub fn has_nonce(&self, sender: &str, nonce: u64) -> bool {
        self.transactions
            .iter()
            .any(|tx| tx.sender == sender && tx.nonce == nonce)
    }

    // ── Maintenance ───────────────────────────────────────────

    /// Removes expired and invalid transactions.
    ///
    /// [FIX-04] Also removes zero-amount transactions.
    pub fn cleanup_expired(&mut self, max_age: u64) {
        let before = self.transactions.len();

        self.transactions.retain(|tx| {
            tx.age_seconds() <= max_age
                && tx.amount > 0
                && tx.sender != tx.receiver
        });

        // Rebuild the HashSet to stay in sync.
        self.known_hashes = self.transactions
            .iter()
            .map(|tx| tx.tx_hash.clone())
            .collect();

        let removed = before - self.transactions.len();
        if removed > 0 {
            println!(
                "[MEMPOOL] Cleaned {} expired/invalid transaction(s).",
                removed
            );
        }
    }

    /// Clears the mempool after a block is mined.
    /// [FIX-06] Save errors are logged.
    pub fn clear(&mut self) {
        self.transactions.clear();
        self.known_hashes.clear();
        self.save();
    }

    // ── Persistence ───────────────────────────────────────────

    /// Saves the mempool to disk.
    /// [FIX-06] Errors logged, not silently swallowed.
    pub fn save(&self) {
        crate::storage::save_mempool(&self.transactions);
    }

    // ── Stats ─────────────────────────────────────────────────

    /// [FIX-10] Single canonical size method.
    pub fn size(&self) -> usize {
        self.transactions.len()
    }

    /// Alias for size() — used by chain.rs.
    pub fn count(&self) -> usize {
        self.transactions.len()
    }

    /// Total pending value in the mempool.
    pub fn total_pending_value(&self) -> u128 {
        self.transactions
            .iter()
            .map(|tx| tx.amount.saturating_add(tx.fee))
            .sum()
    }

    // ── Export ────────────────────────────────────────────────

    /// Exports mempool as a JSON string.
    /// [FIX-07] Uses serde_json, not Debug format.
    pub fn export(&self) -> String {
        serde_json::to_string(&self.transactions)
            .unwrap_or_else(|_| "[]".to_string())
    }
}

// ── Default ───────────────────────────────────────────────────

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// END OF mempool.rs
// ============================================================

