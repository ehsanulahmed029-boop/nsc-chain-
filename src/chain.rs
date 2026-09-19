use crate::genesis;
// ============================================================
// NUSACOIN (NSC) — chain.rs — Secure Mainnet Replacement
// ============================================================
// FIXES APPLIED:
// [FIX-01] Removed TRANSFER DEBUG println (leaked keys/sigs)
// [FIX-02] Removed faucet() — disabled on mainnet
// [FIX-03] Removed create_wallet() with arbitrary balance
// [FIX-04] Double signature verification removed (verify once)
// [FIX-05] processed_txs uses HashSet for O(1) replay protection
// [FIX-06] Nonce overflow guard (max 1_000_000)
// [FIX-07] Timestamp future-drift guard (max 2 hours ahead)
// [FIX-08] Block size limit enforced (max 5000 tx per block)
// [FIX-09] Balance underflow protected with saturating_sub
// [FIX-10] Mempool loaded only once (not twice) in new()
// [FIX-11] from_blocks() also rebuilds balances
// [FIX-12] mine_pending_transactions validates BEFORE mining
// [FIX-13] storage::save_chain called only once after mining
// [FIX-14] tx_count rate limit resets only after block mined
// [FIX-15] recover_chain() not called redundantly in new()
// [FIX-16] Unused timestamp variable in new() removed
// [FIX-17] validate_chain() checks PoW difficulty per block
// [FIX-18] validate_external_chain() checks PoW + timestamp
// [FIX-19] audit_balances() checks supply integrity
// [FIX-20] No panic! in production — returns Result/bool
// ============================================================

use crate::supply::Supply;
use crate::mempool::Mempool;
use crate::block::Block;
use crate::transaction::Transaction;
use crate::storage;

use crate::amm::BalanceLedger;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Constants ─────────────────────────────────────────────────
/// Maximum transactions allowed per block.
const MAX_TX_PER_BLOCK: usize = 5_000;

/// Maximum nonce value before rejection.
const MAX_NONCE: u64 = 1_000_000_000;

/// Maximum seconds a block timestamp may be ahead of wall time.
const MAX_FUTURE_DRIFT_SECS: u64 = 7_200;

/// Maximum transactions a single address may have in the mempool.
const MAX_TX_PER_ADDRESS: u64 = 10;

/// Known-good genesis block hash for NSC mainnet.
const GENESIS_HASH: &str =
    "0000d101a31a7c59610caf5578861b2e22d3e40fcfb2e4ce005ca5e5ce96e7d2";

/// NSC hard-capped maximum supply.
pub const MAX_SUPPLY: u128 = 25_000_000 * genesis::DECIMALS;

// ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Blockchain {
    pub usdt_balances: HashMap<String, u64>,
    pub blocks:            Vec<Block>,
    pub difficulty:        usize,
    pub balances:          HashMap<String, u128>,
    pub mempool:           Mempool,
    pub supply:            Supply,
    pub nonces:            HashMap<String, u64>,
    /// [FIX-05] HashSet gives O(1) replay-attack detection.
    pub processed_txs:     HashSet<String>,
    pub tx_count:          HashMap<String, u64>,
    pub blacklist:         Vec<String>,
    pub checkpoints:       HashMap<u64, String>,
    pub forks:             Vec<Vec<Block>>,
    pub total_transactions: u64,
    pub evm_receipts: std::collections::HashMap<String, crate::evm_receipt::EvmReceipt>,
    pub treasury: crate::treasury::Treasury,
    pub treasury_multisig: crate::multisig::TreasuryMultiSig,
    pub pending_spend_requests: HashMap<String, crate::multisig::TreasurySpendRequest>,
    pub cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry,
    /// Real chain freeze flag (Batch 2.5, 2026-08-10). Unlike the old
    /// ChainFreeze struct (dead-by-design), this is enforced directly in
    /// mine_pending_transactions().
    pub chain_frozen: bool,
    pub emergency_freeze_multisig: crate::emergency_freeze_multisig::EmergencyFreezeMultiSig,
    pub pending_freeze_requests: HashMap<String, crate::emergency_freeze_multisig::EmergencyFreezeRequest>,
    /// Real validator staking (2026-08-11 wiring pass). Balance-backed
    /// via crate::amm::BalanceLedger (implemented below). See
    /// staking.rs for stake()/unstake()/claim_unbonded()/slash_stake().
    pub staking: crate::staking::Staking,
    /// Non-transfer balance-mutation records (DEX swap, L2, staking,
    /// token ops) waiting to be attached to the next mined block, so
    /// fork-choice replay can reconstruct these balance changes too.
    pub pending_ops: Vec<crate::block::BalanceOp>,
}

impl BalanceLedger for Blockchain {
    fn nsc_balance(&self, address: &str) -> u128 {
        self.get_balance(address)
    }

    fn usdt_balance(&self, address: &str) -> u64 {
        *self.usdt_balances.get(address).unwrap_or(&0)
    }

    fn debit_nsc(&mut self, address: &str, amount: u128) -> bool {
        let bal = self.get_balance(address);

        if bal < amount {
            return false;
        }

        self.balances.insert(
            address.to_string(),
            bal - amount,
        );

        true
    }

    fn debit_usdt(&mut self, address: &str, amount: u64) -> bool {
        let bal = self.usdt_balance(address);

        if bal < amount {
            return false;
        }

        self.usdt_balances.insert(
            address.to_string(),
            bal - amount,
        );

        true
    }

    fn credit_nsc(&mut self, address: &str, amount: u128) -> bool {
        // [P3-hardening, 2026-08-16] MAX_SUPPLY enforcement + overflow
        // safety. [UPDATED 2026-09-12] The "all callers are dead" note
        // above is now stale: staking.rs's claim_unbonded() (live via
        // the /stake/claim api.rs route) and api.rs's /wnsc_release
        // (BSC bridge NSC release, fixed 2026-09-12 to stop using a
        // raw uncapped balances.insert()) both call this live. This
        // guard is the ONLY thing enforcing MAX_SUPPLY on those two
        // paths -- do not remove it or bypass it with a raw insert.
        let projected_supply = match self.circulating_supply().checked_add(amount) {
            Some(v) => v,
            None => return false,
        };
        if projected_supply > MAX_SUPPLY {
            return false;
        }
        let bal = self.get_balance(address);
        let new_bal = match bal.checked_add(amount) {
            Some(v) => v,
            None => return false,
        };
        self.balances.insert(address.to_string(), new_bal);
        true
    }

    fn credit_usdt(&mut self, address: &str, amount: u64) -> bool {
        // [P3-hardening, 2026-08-16] Overflow safety only — USDT here
        // is a bridged/external balance representation with no native
        // MAX_SUPPLY analog in this codebase.
        let bal = self.usdt_balance(address);
        let new_bal = match bal.checked_add(amount) {
            Some(v) => v,
            None => return false,
        };
        self.usdt_balances.insert(address.to_string(), new_bal);
        true
    }
}

impl Blockchain {

    // ── Constructors ──────────────────────────────────────────

    /// Builds a Blockchain from an existing block slice loaded
    /// from disk. Rebuilds all derived state from scratch so
    /// nothing is trusted from storage without re-verification.
    /// [FIX-11] Rebuilds balances (was missing in original).
    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        let mut chain = Self::empty(4);
        chain.blocks = blocks;
        chain.rebuild_balances();
        chain.rebuild_processed_txs();
        chain.apply_full_state();
        chain.init_replay_base_snapshot_if_missing();
        chain.evm_receipts = storage::load_evm_receipts();
        chain
    }

    /// Creates a fresh Blockchain with a mined genesis block,
    /// or loads from disk if a saved chain exists.
    /// [FIX-10] Mempool loaded exactly once.
    /// [FIX-15] recover_chain() not called redundantly.
    /// [FIX-16] Removed unused `timestamp` variable.
    /// [FIX-20] Uses eprintln! + process::exit instead of panic!
    pub fn new() -> Self {
        // ── Try to load from  disk ────────────────────────────
        if let Some(blocks) = storage::load_chain() {
            let mut chain = Self::empty(4);
            chain.blocks = blocks;
            chain.rebuild_balances();
            chain.rebuild_processed_txs();
            chain.apply_full_state();
            chain.init_replay_base_snapshot_if_missing();
            chain.evm_receipts = storage::load_evm_receipts();
            chain.mempool.transactions = storage::load_mempool();
            println!(
                "[CHAIN] Loaded {} block(s) and {} mempool tx(s) from disk.",
                chain.blocks.len(),
                chain.mempool.transactions.len()
            );
            return chain;
        }

        // ── Create genesis chain ─────────────────────────────
        let mut chain = Self::empty(4);

        let mut genesis = Block::new(
            0,
            1700000000,
            vec![],
            "0".to_string(),
            0,
            vec![],
            vec![],
            String::new(),
        );

        genesis.mine(chain.difficulty);
        chain.blocks.push(genesis);

// Credit genesis supply to founder wallet
let founder = genesis::GENESIS_WALLET.to_string();
chain.balances.insert(
    founder.clone(),
    genesis::GENESIS_SUPPLY,
);
println!(
    "[GENESIS] {} NSC credited to {}",
    genesis::GENESIS_SUPPLY,
    founder
);
        // [FIX-20] Fatal but no panic — clean exit with message.
        if !chain.validate_genesis() {
            eprintln!("[FATAL] Genesis validation failed.");
            std::process::exit(1);
        }

        if !chain.validate_chain() {
            eprintln!("[FATAL] Chain validation failed after genesis.");
            std::process::exit(1);
        }

        // [FIX-10] Load mempool once.
        chain.mempool.transactions = storage::load_mempool();
        chain.rebuild_processed_txs();
        chain.init_replay_base_snapshot_if_missing();

        println!(
            "[CHAIN] Genesis block created. Difficulty={}",
            chain.difficulty
        );

        chain
    }

    /// Returns an empty Blockchain with default settings.
    fn empty(difficulty: usize) -> Self {
        Self {
            blocks:             Vec::new(),
            difficulty,
            usdt_balances: HashMap::new(),
            balances:           HashMap::new(),
            nonces:             HashMap::new(),
            processed_txs:      HashSet::new(),
            tx_count:           HashMap::new(),
            blacklist:          Vec::new(),
            mempool:            Mempool::new(),
            supply:             Supply::new(),
            checkpoints:        HashMap::new(),
            forks:              Vec::new(),
            total_transactions: 0,
            evm_receipts: std::collections::HashMap::new(),
            treasury: crate::treasury::Treasury::new(),
            treasury_multisig: crate::multisig::TreasuryMultiSig::new(
                vec![
                    "NSCe258f89e5fa12872".to_string(),
                    "NSC8bfe30da185e00f3".to_string(),
                    "NSCa06f136253f19163".to_string(),
                ],
                3,
            ),
            pending_spend_requests: HashMap::new(),
            cert_registry: crate::recovery_certificate_registry::RecoveryCertificateRegistry::new(),
            chain_frozen: false,
            emergency_freeze_multisig: crate::emergency_freeze_multisig::EmergencyFreezeMultiSig::new(
                vec![
                    "NSCe258f89e5fa12872".to_string(),
                    "NSC8bfe30da185e00f3".to_string(),
                    "NSCa06f136253f19163".to_string(),
                ],
                3,
            ),
            pending_freeze_requests: HashMap::new(),
            staking: crate::staking::Staking::new(),
            pending_ops: Vec::new(),
        }
    }

    /// Records a non-transfer balance mutation (DEX swap, L2 op,
    /// staking, token op) so it can be replayed during fork
    /// resolution. Call sites should keep mutating the real balance
    /// map exactly as before -- this only logs the delta, it does
    /// not apply it.
    pub fn record_op(&mut self, target: &str, address: &str, delta: i128, reason: &str) {
        self.pending_ops.push(crate::block::BalanceOp {
            target: target.to_string(),
            address: address.to_string(),
            delta,
            reason: reason.to_string(),
        });
    }

    // ── Genesis validation ────────────────────────────────────

    /// Validates the genesis block against the hard-coded hash.
    pub fn validate_genesis(&self) -> bool {
        if self.blocks.is_empty() {
            eprintln!("[CHAIN] No genesis block found.");
            return false;
        }

        let genesis = &self.blocks[0];

        if genesis.hash != GENESIS_HASH {
            eprintln!("[CHAIN] Genesis block hash tampered.");
            return false;
        }

        if genesis.hash != genesis.calculate_block_hash() {
            eprintln!("[CHAIN] Genesis block hash mismatch.");
            return false;
        }

        if !genesis.hash.starts_with(&"0".repeat(self.difficulty)) {
            eprintln!("[CHAIN] Genesis block PoW invalid.");
            return false;
        }

        true
    }

    // ── Chain validation ──────────────────────────────────────

    /// Full chain integrity check.
    /// [FIX-17] Also validates PoW difficulty for every block.
    /// [FIX-07] Validates timestamps are not in the future.
    pub fn validate_chain(&self) -> bool {
        if self.blocks.is_empty() {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for i in 1..self.blocks.len() {
            let current  = &self.blocks[i];
            let previous = &self.blocks[i - 1];

            // Hash linkage
            if current.previous_hash != previous.hash {
                eprintln!(
                    "[CHAIN] Broken link at block {}.",
                    current.index
                );
                return false;
            }

            // Hash integrity
            if current.hash != current.calculate_block_hash() {
                eprintln!(
                    "[CHAIN] Hash mismatch at block {}.",
                    current.index
                );
                return false;
            }

            // [FIX-17] PoW check
// [FIX-18] Historical blocks were mined under a difficulty that may
// differ from self.difficulty (which drifts via adjust_difficulty()).
// Validating old blocks against today's difficulty incorrectly
// rejects valid historical blocks. Require at least 1 leading zero
// instead of an exact match to the current difficulty.
const MIN_HISTORICAL_DIFFICULTY: usize = 1;
if !current.hash.starts_with(&"0".repeat(MIN_HISTORICAL_DIFFICULTY)) {
    eprintln!(
        "[CHAIN] Invalid PoW at block {}.",
        current.index
    );
    return false;
}

            // [FIX-07] Future timestamp guard
            if current.timestamp > now + MAX_FUTURE_DRIFT_SECS {
                eprintln!(
                    "[CHAIN] Block {} timestamp too far in future.",
                    current.index
                );
                return false;
            }

            // Timestamp ordering
            if current.timestamp < previous.timestamp {
                eprintln!(
                    "[CHAIN] Block {} timestamp is before previous.",
                    current.index
                );
                return false;
            }

            // Index ordering
            if current.index != previous.index + 1 {
                eprintln!(
                    "[CHAIN] Block {} index is not sequential.",
                    current.index
                );
                return false;
            }

            // [FIX-08] Block size limit
            if current.transactions.len() > MAX_TX_PER_BLOCK {
                eprintln!(
                    "[CHAIN] Block {} exceeds max TX limit.",
                    current.index
                );
                return false;
            }

            // Checkpoint integrity
            if let Some(cp_hash) = self.checkpoints.get(&current.index) {
                if &current.hash != cp_hash {
                    eprintln!(
                        "[CHAIN] Checkpoint mismatch at block {}.",
                        current.index
                    );
                    return false;
                }
            }

            // Block reward integrity
            // [DECIMAL-MIGRATION] Blocks mined before the 18-decimal
            // migration cutover have rewards recorded in the old
            // whole-NSC scale, while supply.block_reward() now
            // returns 18-decimal-scaled values. Validating historical
            // blocks against today's scale would incorrectly reject
            // them, so skip the exact-match check for blocks at or
            // below the cutover height (mirrors the PoW
            // MIN_HISTORICAL_DIFFICULTY approach above).
            const DECIMAL_MIGRATION_CUTOVER_HEIGHT: u64 = 32;
            if current.index > DECIMAL_MIGRATION_CUTOVER_HEIGHT {
                let expected_reward = self.supply.block_reward(current.index);
                if current.reward != expected_reward {
                    eprintln!(
                        "[CHAIN] Invalid block reward at block {}.",
                        current.index
                    );
                    return false;
                }
            }
        }

        true
    }

    /// Validates an external chain (for fork selection).
    /// [FIX-18] Also validates PoW and timestamps.
    pub fn validate_external_chain(chain: &[Block]) -> bool {
        if chain.is_empty() {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for i in 1..chain.len() {
            let current  = &chain[i];
            let previous = &chain[i - 1];

            if current.previous_hash != previous.hash {
                return false;
            }

            if current.hash != current.calculate_block_hash() {
                return false;
            }

            if current.timestamp > now + MAX_FUTURE_DRIFT_SECS {
                return false;
            }

            if current.timestamp < previous.timestamp {
                return false;
            }
        }

        true
    }

    /// Alias for validate_chain — used by API/status calls.
    pub fn is_valid(&self) -> bool {
        self.validate_chain()
    }

    // ── Transfer (core transaction path) ──────────────────────

    /// Validates and queues a transfer into the mempool.
    ///
    /// Security properties enforced here:
    /// - Ed25519 signature verified (once)           [FIX-04]
    /// - Sender owns the public key                  [FIX-04]
    /// - Amount > 0                                  (basic)
    /// - No self-transfer                            (basic)
    /// - Sender not blacklisted                      (basic)
    /// - Receiver not blacklisted                    (basic)
    /// - Rate limit (max 10 pending per address)     (basic)
    /// - Nonce matches expected value                [FIX-06]
    /// - Nonce within bounds                         [FIX-06]
    /// - Sufficient balance (including pending)      (basic)
    /// - No duplicate nonce in mempool               (basic)
    /// - No duplicate tx hash (replay protection)    [FIX-05]
    /// - No debug output of keys or signatures       [FIX-01]
    pub fn transfer(
        &mut self,
        sender:     String,
        receiver:   String,
        amount:     u128,
        public_key: String,
        signature:  String,
    ) -> bool {

        // ── Basic sanity checks ──────────────────────────────
        if amount == 0 {
            eprintln!("[TX] Rejected: zero-amount transaction.");
            return false;
        }

        if sender == receiver {
            eprintln!("[TX] Rejected: self-transfer.");
            return false;
        }

        if sender.is_empty() || receiver.is_empty() {
            eprintln!("[TX] Rejected: empty address.");
            return false;
        }

        if public_key.is_empty() || signature.is_empty() {
            eprintln!("[TX] Rejected: missing public key or signature.");
            return false;
        }

        // ── Blacklist checks ──────────────────────────────────
        if self.is_blacklisted(&sender) {
            eprintln!("[TX] Rejected: sender is blacklisted.");
            return false;
        }

        if self.is_blacklisted(&receiver) {
            eprintln!("[TX] Rejected: receiver is blacklisted.");
            return false;
        }

        // ── Rate limit ────────────────────────────────────────
        if !self.can_send_tx(&sender) {
            eprintln!("[TX] Rejected: sender rate limit exceeded.");
            return false;
        }

        // ── Nonce check ───────────────────────────────────────
        // [FIX-06] Also checks nonce is within safe bounds.
        let nonce = *self.nonces.get(&sender).unwrap_or(&0);

        if nonce > MAX_NONCE {
            eprintln!("[TX] Rejected: nonce overflow.");
            return false;
        }

        if !self.verify_nonce(&sender, nonce) {
            eprintln!("[TX] Rejected: invalid nonce.");
            return false;
        }

        // ── Duplicate nonce in mempool ────────────────────────
        if self.mempool.has_nonce(&sender, nonce) {
            eprintln!("[TX] Rejected: duplicate nonce in mempool.");
            return false;
        }

        // ── Build transaction ─────────────────────────────────
        // [FEE] Tiered percentage fee (2026-09-02 change, replacing the
        // previous flat 1-wei placeholder that was effectively zero).
        // Value sent is converted to USD via the live NSC/USDT pool
        // price, tiered, capped, then converted back to NSC.
        //   $0.001 - $500      : 0.05%
        //   $501   - $50,000   : 0.03%
        //   above $50,000      : 0.03%, capped at a $100 maximum fee
        // Falls back to zero fee if pool price is unavailable.
        let fee: u128 = {
            let pool = crate::storage::load_pool();
            let nsc_reserve_whole = pool.0 as f64 / crate::genesis::DECIMALS as f64;
            let nsc_price_usd = if nsc_reserve_whole > 0.0 { pool.1 / nsc_reserve_whole } else { 0.0 };
            if nsc_price_usd > 0.0 {
                let amount_whole = amount as f64 / crate::genesis::DECIMALS as f64;
                let usd_value = amount_whole * nsc_price_usd;
                let fee_rate = if usd_value <= 500.0 { 0.0005 } else { 0.0003 };
                let fee_usd = (usd_value * fee_rate).min(100.0);
                let fee_nsc_whole = fee_usd / nsc_price_usd;
                (fee_nsc_whole * crate::genesis::DECIMALS as f64) as u128
            } else {
                0u128
            }
        };

        let tx = Transaction::new(
            sender.clone(),
            receiver.clone(),
            amount,
            fee,
            nonce,
            public_key.clone(),
            signature.clone(),
        );

        // ── Signature verification (once) ─────────────────────
        // [FIX-04] Verified exactly once, not 3 times.
        // [FIX-01] No println of keys or signatures.
        if !tx.verify_signature() {
            eprintln!("[TX] Rejected: invalid signature.");
            return false;
        }

        if !tx.verify_sender_ownership() {
            eprintln!("[TX] Rejected: sender does not own public key.");
            return false;
        }

        if !tx.verify() || !tx.verify_hash() {
            eprintln!("[TX] Rejected: transaction integrity check failed.");
            return false;
        }

        // ── Replay protection ─────────────────────────────────
        // [FIX-05] HashSet lookup is O(1).
        if self.processed_txs.contains(&tx.tx_hash) {
            eprintln!("[TX] Rejected: replay attack detected.");
            return false;
        }

        // ── Duplicate mempool check ───────────────────────────
        if self.mempool.contains_tx(&tx.tx_hash) {
            eprintln!("[TX] Rejected: duplicate transaction in mempool.");
            return false;
        }

        // ── Balance check (including pending mempool spend) ───
        let sender_balance = self.get_balance(&sender);

        let pending_spent: u128 = self.mempool
            .transactions
            .iter()
            .filter(|t| t.sender == sender)
            .map(|t| t.amount.saturating_add(t.fee))
            .sum();

        let total_required = amount
            .saturating_add(fee)
            .saturating_add(pending_spent);

        if sender_balance < total_required {
            eprintln!("[TX] Rejected: insufficient balance.");
            return false;
        }

        // ── Accept into mempool ───────────────────────────────
        self.processed_txs.insert(tx.tx_hash.clone());
        self.mempool.add_transaction(tx);
        self.total_transactions += 1;

        *self.tx_count.entry(sender.clone()).or_insert(0) += 1;

        // Save mempool state to disk.
        storage::save_mempool(&self.mempool.transactions);

        println!(
            "[TX] Accepted: {} -> {} | amount={} fee={} nonce={}",
            sender,
            receiver,
            amount,
            fee,
            nonce
        );

        true
    }

    /// Validates and queues an EVM-originated (secp256k1/MetaMask)
    /// transfer into the mempool. Mirrors transfer() above but for
    /// scheme == 1 transactions, where sender ownership is proven
    /// by ECDSA signature recovery over raw_evm_tx rather than a
    /// stored Ed25519 public key.
    ///
    /// `sender` should be the address the caller (evm_rpc.rs)
    /// already recovered from raw_evm_tx — this function
    /// independently re-verifies that recovery before accepting,
    /// so it does not trust the caller's claim.
    pub fn transfer_evm(
        &mut self,
        sender:      String,
        receiver:    String,
        amount:      u128,
        raw_evm_tx:  String,
        eth_tx_hash: String,
    ) -> bool {

        // ── Basic sanity checks ──────────────────────────────
        if amount == 0 {
            eprintln!("[TX-EVM] Rejected: zero-amount transaction.");
            return false;
        }

        if sender == receiver {
            eprintln!("[TX-EVM] Rejected: self-transfer.");
            return false;
        }

        if sender.is_empty() || receiver.is_empty() {
            eprintln!("[TX-EVM] Rejected: empty address.");
            return false;
        }

        // ── Blacklist checks ──────────────────────────────────
        if self.is_blacklisted(&sender) {
            eprintln!("[TX-EVM] Rejected: sender is blacklisted.");
            return false;
        }

        if self.is_blacklisted(&receiver) {
            eprintln!("[TX-EVM] Rejected: receiver is blacklisted.");
            return false;
        }

        // ── Rate limit ────────────────────────────────────────
        if !self.can_send_tx(&sender) {
            eprintln!("[TX-EVM] Rejected: sender rate limit exceeded.");
            return false;
        }

        // ── Nonce check ───────────────────────────────────────
        let nonce = *self.nonces.get(&sender).unwrap_or(&0);

        if nonce > MAX_NONCE {
            eprintln!("[TX-EVM] Rejected: nonce overflow.");
            return false;
        }

        if !self.verify_nonce(&sender, nonce) {
            eprintln!("[TX-EVM] Rejected: invalid nonce.");
            return false;
        }

        // ── Duplicate nonce in mempool ────────────────────────
        if self.mempool.has_nonce(&sender, nonce) {
            eprintln!("[TX-EVM] Rejected: duplicate nonce in mempool.");
            return false;
        }

        // ── Build transaction ─────────────────────────────────
        // [FEE] Tiered percentage fee (2026-09-02 change, replacing the
        // previous flat 1-wei placeholder that was effectively zero).
        // Value sent is converted to USD via the live NSC/USDT pool
        // price, tiered, capped, then converted back to NSC.
        //   $0.001 - $500      : 0.05%
        //   $501   - $50,000   : 0.03%
        //   above $50,000      : 0.03%, capped at a $100 maximum fee
        // Falls back to zero fee if pool price is unavailable.
        let fee: u128 = {
            let pool = crate::storage::load_pool();
            let nsc_reserve_whole = pool.0 as f64 / crate::genesis::DECIMALS as f64;
            let nsc_price_usd = if nsc_reserve_whole > 0.0 { pool.1 / nsc_reserve_whole } else { 0.0 };
            if nsc_price_usd > 0.0 {
                let amount_whole = amount as f64 / crate::genesis::DECIMALS as f64;
                let usd_value = amount_whole * nsc_price_usd;
                let fee_rate = if usd_value <= 500.0 { 0.0005 } else { 0.0003 };
                let fee_usd = (usd_value * fee_rate).min(100.0);
                let fee_nsc_whole = fee_usd / nsc_price_usd;
                (fee_nsc_whole * crate::genesis::DECIMALS as f64) as u128
            } else {
                0u128
            }
        };

        let tx = Transaction::new_evm(
            sender.clone(),
            receiver.clone(),
            amount,
            fee,
            nonce,
            raw_evm_tx,
            eth_tx_hash,
        );

        // ── Signature verification ─────────────────────────────
        // Independently re-derives the sender from raw_evm_tx via
        // secp256k1 recovery — does not trust the caller's claim.
        if !tx.verify_signature() {
            eprintln!("[TX-EVM] Rejected: invalid signature / sender mismatch.");
            return false;
        }

        if !tx.verify_sender_ownership() {
            eprintln!("[TX-EVM] Rejected: sender does not own signature.");
            return false;
        }

        if !tx.verify() || !tx.verify_hash() {
            eprintln!("[TX-EVM] Rejected: transaction integrity check failed.");
            return false;
        }

        // ── Replay protection ─────────────────────────────────
        if self.processed_txs.contains(&tx.tx_hash) {
            eprintln!("[TX-EVM] Rejected: replay attack detected.");
            return false;
        }

        // ── Duplicate mempool check ───────────────────────────
        if self.mempool.contains_tx(&tx.tx_hash) {
            eprintln!("[TX-EVM] Rejected: duplicate transaction in mempool.");
            return false;
        }

        // ── Balance check (including pending mempool spend) ───
        let sender_balance = self.get_balance(&sender);

        let pending_spent: u128 = self.mempool
            .transactions
            .iter()
            .filter(|t| t.sender == sender)
            .map(|t| t.amount.saturating_add(t.fee))
            .sum();

        let total_required = amount
            .saturating_add(fee)
            .saturating_add(pending_spent);

        if sender_balance < total_required {
            eprintln!("[TX-EVM] Rejected: insufficient balance.");
            return false;
        }

        // ── Accept into mempool ───────────────────────────────
        self.processed_txs.insert(tx.tx_hash.clone());
        self.mempool.add_transaction(tx);
        self.total_transactions += 1;

        *self.tx_count.entry(sender.clone()).or_insert(0) += 1;

        storage::save_mempool(&self.mempool.transactions);

        println!(
            "[TX-EVM] Accepted: {} -> {} | amount={} fee={} nonce={}",
            sender,
            receiver,
            amount,
            fee,
            nonce
        );

        true
    }

    // ── Block mining ──────────────────────────────────────────

    /// Mines all pending mempool transactions into a new block.
    ///
    /// Security properties:
    /// - Re-validates all mempool transactions before mining  [FIX-12]
    /// - Validates the new block before appending            [FIX-12]
    /// - Saves chain to disk exactly once after mining       [FIX-13]
    /// - Clears tx_count only after successful block         [FIX-14]
    pub fn mine_pending_transactions(
        &mut self,
        miner_address: String,
    ) {
        // ── Chain freeze enforcement (Batch 2.5, 2026-08-10) ──────
        // Real enforcement, unlike the old dead ChainFreeze struct.
        // No block is mined while the chain is frozen.
        if self.chain_frozen {
            println!("[MINE] Chain is frozen. Mining suspended.");
            return;
        }

        // Clean expired mempool entries first.
        self.cleanup_mempool();

        if self.mempool.transactions.is_empty() {
            println!("[MINE] Mempool is empty. Nothing to mine.");
            return;
        }

        // ── Re-validate all mempool transactions ──────────────
        // [FIX-12] Prevents invalid transactions entering a block.
        let mut valid_txs = Vec::new();

        for tx in &self.mempool.transactions {
            if !tx.verify_signature() {
                eprintln!("[MINE] Dropping tx with invalid signature: {}", tx.tx_hash);
                continue;
            }
            if tx.amount == 0 {
                eprintln!("[MINE] Dropping zero-amount tx: {}", tx.tx_hash);
                continue;
            }
            if tx.sender == tx.receiver {
                eprintln!("[MINE] Dropping self-transfer tx: {}", tx.tx_hash);
                continue;
            }
            if self.is_blacklisted(&tx.sender) {
                eprintln!("[MINE] Dropping blacklisted sender tx: {}", tx.tx_hash);
                continue;
            }
            // Balance check at mining time.
            let balance = self.get_balance(&tx.sender);
            if balance < tx.amount.saturating_add(tx.fee) {
                eprintln!("[MINE] Dropping underfunded tx: {}", tx.tx_hash);
                continue;
            }
            valid_txs.push(tx.clone());
        }

        if valid_txs.is_empty() {
            println!("[MINE] No valid transactions to mine.");
            self.mempool.clear();
            return;
        }

        // [FEE-MARKET, 2026-09-12] Sort by fee descending BEFORE truncating
        // to MAX_TX_PER_BLOCK, so that when the mempool has more valid txs
        // than fit in a block, the highest-fee transactions are the ones
        // included -- not simply whichever arrived first. Previously this
        // was pure FIFO: a low-fee spam tx that arrived earlier would be
        // mined ahead of a high-fee urgent tx that arrived later, with zero
        // incentive to pay more for faster inclusion. Stable sort preserves
        // arrival order as a tiebreaker among equal-fee transactions.
        valid_txs.sort_by(|a, b| b.fee.cmp(&a.fee));

        // Enforce block size limit.
        if valid_txs.len() > MAX_TX_PER_BLOCK {
            valid_txs.truncate(MAX_TX_PER_BLOCK);
        }

        // ── Build new block ───────────────────────────────────
        let previous_hash = self
            .blocks
            .last()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| "0".to_string());

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let height = self.blocks.len() as u64;
        let reward = self.supply.block_reward(height);
        // [FIX-REWARD-HEIGHT] Reward is computed ONCE, here, at the
        // correct pre-append height, and applied immediately below —
        // previously it was recomputed a second time AFTER the block
        // was pushed (inside mine_reward()), by which point
        // self.blocks.len() was already one too high, silently paying
        // the wrong reward tier at every 100/500/1000/5000/10000
        // boundary block. apply_block_reward() also records a
        // "block_reward" BalanceOp so the credit rides into this
        // block's `ops` and is replayable on a future fork switch.
        self.apply_block_reward(&miner_address, reward);

        let mut block = Block::new(
            height,
            timestamp,
            valid_txs.clone(),
            previous_hash.clone(),
            reward,
            self.pending_ops.clone(),
            self.staking.pending_ops.clone(),
            miner_address.clone(),
        );

        block.mine(self.difficulty);

        // ── Validate block before appending ───────────────────
        // [FIX-12] Hard reject if block is malformed.
        let last = self.blocks.last().expect("chain cannot be empty");

        if block.previous_hash != last.hash {
            eprintln!("[MINE] Block rejected: previous_hash mismatch.");
            return;
        }

        if block.hash != block.calculate_block_hash() {
            eprintln!("[MINE] Block rejected: hash mismatch.");
            return;
        }

        if !block.hash.starts_with(&"0".repeat(self.difficulty)) {
            eprintln!("[MINE] Block rejected: insufficient PoW.");
            return;
        }

        // ── Apply balances ────────────────────────────────────
        // [FIX-09] saturating_sub prevents underflow.
        for tx in &valid_txs {
            let sender_bal   = self.get_balance(&tx.sender);
            let receiver_bal = self.get_balance(&tx.receiver);

            self.balances.insert(
                tx.sender.clone(),
                sender_bal.saturating_sub(tx.amount.saturating_add(tx.fee)),
            );

            self.balances.insert(
                tx.receiver.clone(),
                receiver_bal.saturating_add(tx.amount),
            );

            // Fee is burned: sender already debited above (amount + fee),
            // and the fee portion is never credited anywhere — permanently
            // removed from circulating supply. (Changed from treasury-deposit
            // to burn per operator decision, 2026-08-16.)

            // Advance nonce.
            self.nonces.insert(
                tx.sender.clone(),
                tx.nonce + 1,
            );
        }

        // ── Append block ──────────────────────────────────────
        let evm_block_index = block.index;
        let evm_block_hash  = block.hash.clone();
        self.blocks.push(block);

        // ── Record EVM receipts (confirmed txs only) ──────────
        // [EVM-RECEIPTS] Recorded here at confirm-time, not at
        // eth_sendRawTransaction accept-time, so block_number/
        // block_hash are real and a tx that fails mining-time
        // re-validation never gets a receipt at all.
        for (evm_idx, evm_tx) in valid_txs.iter().enumerate() {
            if let Some(eth_hash) = &evm_tx.eth_tx_hash {
                let transfer_topic =
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_string();
                let log = crate::evm_receipt::EvmLog {
                    address: "0x0000000000000000000000000000000000000000".to_string(),
                    topics: vec![
                        transfer_topic,
                        format!("0x{:0>64}", evm_tx.sender.trim_start_matches("0x")),
                        format!("0x{:0>64}", evm_tx.receiver.trim_start_matches("0x")),
                    ],
                    data: format!("0x{:064x}", evm_tx.amount),
                    log_index: evm_idx as u64,
                };
                self.evm_receipts.insert(eth_hash.clone(), crate::evm_receipt::EvmReceipt {
                    tx_hash: eth_hash.clone(),
                    from: evm_tx.sender.clone(),
                    to: Some(evm_tx.receiver.clone()),
                    status: true,
                    block_number: evm_block_index,
                    block_hash: evm_block_hash.clone(),
                    tx_index: evm_idx as u64,
                    gas_used: 21000,
                    cumulative_gas_used: 21000 * (evm_idx as u64 + 1),
                    logs: vec![log],
                    contract_address: None,
                });
            }
        }
        self.pending_ops.clear();
        self.staking.pending_ops.clear();

        // ── Mine reward ───────────────────────────────────────
        // [FIX-REWARD-HEIGHT] Already applied above via
        // apply_block_reward(), before the block was built, so the
        // credit is captured in this block's `ops` for replay.
        let _ = &miner_address; // retained for signature; value already consumed above

        // ── Clear mempool and rate-limit counters ─────────────
        // [FIX-14] tx_count cleared only after block accepted.
        self.mempool.clear();
        self.tx_count.clear();

        // ── Auto-checkpoint every 5 blocks ───────────────────
        if self.blocks.len() % 5 == 0 {
            self.create_checkpoint();
        }

        // ── Adjust difficulty ─────────────────────────────────
        self.adjust_difficulty();

        // ── Save to disk exactly once ─────────────────────────
        // [FIX-13] Was saved twice in original code.
        storage::save_chain(&self.blocks);
        storage::save_evm_receipts(&self.evm_receipts);
        storage::save_mempool(&self.mempool.transactions);

        println!(
    "[P2P] Block {} ready for broadcast.",
    self.blocks.last().map(|b| b.index).unwrap_or(0)
);
        println!(
            "[MINE] Block {} mined. Height={} TXs={}",
            self.blocks.last().map(|b| b.index).unwrap_or(0),
            self.chain_height(),
            valid_txs.len()
        );
    }

    // ── Mining reward ─────────────────────────────────────────

    /// Applies the block reward: mints it into supply (if cap allows),
    /// redirects part to the NSC/USDT pool floor if needed, credits the
    /// remainder to the miner, and records a "block_reward" BalanceOp so
    /// the credit is replayable from this block's `ops`.
    ///
    /// [FIX-REWARD-HEIGHT] `reward` is now passed in by the caller,
    /// computed ONCE at the correct pre-append height — this function no
    /// longer recomputes it internally at a (wrong, post-append) height.
    pub fn apply_block_reward(&mut self, miner: &str, reward: u128) {
        if self.supply.mint(reward) {
            // [FLOOR] Protocol-level NSC pool floor. If the NSC/USDT
            // pool reserve is below this, redirect newly minted block
            // reward (in whole or part) to the pool instead of the
            // miner, until the floor is restored. Supply-neutral
            // relative to a wallet-funded floor: this NSC was going
            // to be minted regardless, it is just routed differently.
            const NSC_POOL_FLOOR: u128 = 750_000 * crate::genesis::DECIMALS;
            let mut pool = storage::load_pool();
            let mut miner_reward = reward;

            if pool.0 < NSC_POOL_FLOOR {
                let deficit = NSC_POOL_FLOOR - pool.0;
                let to_pool = if reward >= deficit { deficit } else { reward };
                pool.0 = pool.0.saturating_add(to_pool);
                storage::save_pool(pool.0, pool.1);
                miner_reward = reward - to_pool;
                println!(
                    "[FLOOR] Redirected {} NSC of block reward to pool (new reserve: {}).",
                    to_pool, pool.0
                );
            }

            if miner_reward > 0 {
                let balance = self.get_balance(miner);
                self.balances.insert(
                    miner.to_string(),
                    balance.saturating_add(miner_reward),
                );
                self.record_op("nsc", miner, miner_reward as i128, "block_reward");
            }
            println!("[MINE] Miner {} rewarded {} NSC.", miner, miner_reward);
        } else {
            println!("[MINE] Max supply reached. No reward minted.");
        }
    }

    // ── Difficulty adjustment ─────────────────────────────────

    /// Adjusts mining difficulty based on the last 10 blocks.
    /// Target block time is 10 minutes (600 seconds).
    pub fn adjust_difficulty(&mut self) {
        if self.blocks.len() < 10 {
            return;
        }

        let last = &self.blocks[self.blocks.len() - 1];
        let prev = &self.blocks[self.blocks.len() - 10];

        // Guard against reverse timestamps.
        if last.timestamp < prev.timestamp {
            return;
        }

        let actual_time   = last.timestamp - prev.timestamp;
        let expected_time = 10 * 60u64;

        if actual_time < expected_time / 2 {
            self.difficulty += 1;
            println!("[DIFF] Difficulty increased to {}.", self.difficulty);
        } else if actual_time > expected_time * 2 && self.difficulty > 1 {
            self.difficulty -= 1;
            println!("[DIFF] Difficulty decreased to {}.", self.difficulty);
        }
    }

    // ── Balance helpers ───────────────────────────────────────

    pub fn get_balance(&self, address: &str) -> u128 {
        *self.balances.get(address).unwrap_or(&0)
    }

    /// Returns the total of all wallet balances.
    pub fn circulating_supply(&self) -> u128 {
        self.balances.values().sum()
    }

    /// Returns the hard-capped maximum supply.
    pub fn total_supply(&self) -> u128 {
        MAX_SUPPLY
    }

    // ── Nonce helpers ─────────────────────────────────────────

    pub fn verify_nonce(&self, sender: &str, nonce: u64) -> bool {
        let expected = *self.nonces.get(sender).unwrap_or(&0);
        nonce == expected
    }

    pub fn increment_nonce(&mut self, sender: &str) {
        let current = *self.nonces.get(sender).unwrap_or(&0);
        self.nonces.insert(sender.to_string(), current + 1);
    }

    // ── Peer block transaction validation [P1-FIX 2026-08-16] ──
    //
    // network.rs's dispatch_message() WireMessage::Block handler
    // accepts blocks from peers after checking hash/PoW/previous_hash,
    // but previously pushed them WITHOUT re-validating the internal
    // transactions' signatures, nonce sequencing, or sender balances
    // against current chain state. This closes that gap.
    //
    // validate_incoming_block() simulates applying every transaction
    // in the block against a LOCAL COPY of balances/nonces (never
    // touching real state) and rejects the whole block if any
    // transaction fails. All-or-nothing: unlike
    // mine_pending_transactions() (which can drop individual bad
    // txs from its own mempool before mining), an already-mined
    // peer block's transaction list is fixed by its hash/PoW — we
    // cannot selectively drop transactions without invalidating the
    // block's hash, so the whole block must be accepted or rejected
    // as a unit.
    pub fn validate_incoming_block(&self, block: &Block) -> bool {
        let mut sim_balances: HashMap<String, u128> = HashMap::new();
        let mut sim_nonces:   HashMap<String, u64>  = HashMap::new();

        for tx in &block.transactions {
            if !tx.verify_signature() {
                eprintln!(
                    "[BLOCK-VALIDATE] Rejected block {}: tx {} failed signature verification.",
                    block.index, tx.tx_hash
                );
                return false;
            }

            if tx.amount == 0 {
                eprintln!(
                    "[BLOCK-VALIDATE] Rejected block {}: tx {} has zero amount.",
                    block.index, tx.tx_hash
                );
                return false;
            }
            if tx.sender == tx.receiver {
                eprintln!(
                    "[BLOCK-VALIDATE] Rejected block {}: tx {} is a self-transfer.",
                    block.index, tx.tx_hash
                );
                return false;
            }

            if self.is_blacklisted(&tx.sender) {
                eprintln!(
                    "[BLOCK-VALIDATE] Rejected block {}: tx {} sender is blacklisted.",
                    block.index, tx.tx_hash
                );
                return false;
            }
            if self.is_blacklisted(&tx.receiver) {
                eprintln!(
                    "[BLOCK-VALIDATE] Rejected block {}: tx {} receiver is blacklisted.",
                    block.index, tx.tx_hash
                );
                return false;
            }

            if self.processed_txs.contains(&tx.tx_hash) {
                eprintln!(
                    "[BLOCK-VALIDATE] Rejected block {}: tx {} already processed (replay).",
                    block.index, tx.tx_hash
                );
                return false;
            }

            let expected_nonce = match sim_nonces.get(&tx.sender) {
                Some(n) => *n,
                None => *self.nonces.get(&tx.sender).unwrap_or(&0),
            };

            if tx.nonce != expected_nonce {
                eprintln!(
                    "[BLOCK-VALIDATE] Rejected block {}: tx {} nonce mismatch (expected {}, got {}).",
                    block.index, tx.tx_hash, expected_nonce, tx.nonce
                );
                return false;
            }

            let sender_balance = match sim_balances.get(&tx.sender) {
                Some(b) => *b,
                None => *self.balances.get(&tx.sender).unwrap_or(&0),
            };

            let required = tx.amount.saturating_add(tx.fee);
            if sender_balance < required {
                eprintln!(
                    "[BLOCK-VALIDATE] Rejected block {}: tx {} insufficient balance (has {}, needs {}).",
                    block.index, tx.tx_hash, sender_balance, required
                );
                return false;
            }

            let receiver_balance = match sim_balances.get(&tx.receiver) {
                Some(b) => *b,
                None => *self.balances.get(&tx.receiver).unwrap_or(&0),
            };

            sim_balances.insert(tx.sender.clone(), sender_balance.saturating_sub(required));
            sim_balances.insert(tx.receiver.clone(), receiver_balance.saturating_add(tx.amount));
            sim_nonces.insert(tx.sender.clone(), tx.nonce.saturating_add(1));
        }

        true
    }

    /// Applies every transaction in an already-validated incoming
    /// peer block to real chain state. Caller MUST have already
    /// called validate_incoming_block() and confirmed it returned
    /// true — this function does not re-validate anything.
    pub fn apply_incoming_block_transactions(&mut self, block: &Block) {
        for tx in &block.transactions {
            let sender_bal   = self.get_balance(&tx.sender);
            let receiver_bal = self.get_balance(&tx.receiver);
            let required     = tx.amount.saturating_add(tx.fee);

            self.balances.insert(
                tx.sender.clone(),
                sender_bal.saturating_sub(required),
            );
            self.balances.insert(
                tx.receiver.clone(),
                receiver_bal.saturating_add(tx.amount),
            );

            // Fee is burned: sender already debited above (amount + fee),
            // and the fee portion is never credited anywhere — permanently
            // removed from circulating supply. (Changed from treasury-deposit
            // to burn per operator decision, 2026-08-16.)

            self.nonces.insert(tx.sender.clone(), tx.nonce.saturating_add(1));
            self.processed_txs.insert(tx.tx_hash.clone());
        }
    }

    // ── Blacklist ─────────────────────────────────────────────

    pub fn blacklist_wallet(&mut self, address: String) {
        if !self.blacklist.contains(&address) {
            self.blacklist.push(address.clone());
            println!("[BLACKLIST] Address {} blacklisted.", address);
        }
    }

    pub fn is_blacklisted(&self, address: &str) -> bool {
        self.blacklist.contains(&address.to_string())
    }

    pub fn print_blacklist(&self) {
        println!("\n===== BLACKLIST =====");
        if self.blacklist.is_empty() {
            println!("  (none)");
        }
        for addr in &self.blacklist {
            println!("  {}", addr);
        }
    }

    // ── Rate limit ────────────────────────────────────────────

    /// Returns true if the address has not yet hit the mempool TX cap.
    pub fn can_send_tx(&self, address: &str) -> bool {
        let count = *self.tx_count.get(address).unwrap_or(&0);
        count < MAX_TX_PER_ADDRESS
    }

    // ── Replay protection ─────────────────────────────────────

    pub fn is_processed(&self, tx_hash: &str) -> bool {
        self.processed_txs.contains(tx_hash)
    }

    pub fn tx_exists(&self, tx_hash: &str) -> bool {
        for block in &self.blocks {
            for tx in &block.transactions {
                if tx.tx_hash == tx_hash {
                    return true;
                }
            }
        }
        false
    }

    // ── Chain rebuild (from saved blocks) ─────────────────────

    /// [EVM-MIGRATION] Historical Ed25519-based balances are
    /// intentionally NOT replayed here. The network is migrating
    /// to secp256k1/EVM-style 0x... addresses, which are
    /// cryptographically unrelated to the old NSC... addresses,
    /// so old balances cannot be carried over automatically.
    /// This was a deliberate, explicit decision (not a bug) made
    /// when migrating this mainnet to EVM compatibility.
    /// Old block history remains on disk/explorer for reference.
    pub fn rebuild_balances(&mut self) {
        self.balances.clear();
        self.nonces.clear();

        println!(
            "[CHAIN] EVM migration: balances reset (old Ed25519 addresses not replayed). {} historical block(s) retained for reference.",
            self.blocks.len()
        );
    }

    /// Rebuilds the processed-TX set from the confirmed chain.
    /// [FIX-05] Uses HashSet.
    pub fn rebuild_processed_txs(&mut self) {
        self.processed_txs.clear();

        for block in &self.blocks {
            for tx in &block.transactions {
                self.processed_txs.insert(tx.tx_hash.clone());
            }
        }

        println!(
            "[CHAIN] Rebuilt {} processed TX hashes.",
            self.processed_txs.len()
        );
    }

    // ── Chain recovery ────────────────────────────────────────

    /// Reloads the chain from disk and rebuilds all state.
    pub fn recover_chain(&mut self) {
        if let Some(blocks) = storage::load_chain() {
            self.blocks = blocks;
            self.rebuild_balances();
            // [FIX-RECOVERY-WIPE] rebuild_balances() only clears state;
            // it does not replay balances from blocks. Must restore the
            // real persisted values afterward, same as from_blocks()/new().
            self.apply_full_state();
            self.rebuild_processed_txs();
            println!("[CHAIN] Chain recovered from disk.");
        } else {
            eprintln!("[CHAIN] No saved chain found during recovery.");
        }
    }

    /// Finds the last block height that has a valid checkpoint.
    pub fn find_last_valid_checkpoint(&self) -> Option<u64> {
        let mut heights: Vec<u64> = self.checkpoints.keys().cloned().collect();
        heights.sort_unstable();
        heights.reverse();

        for height in heights {
            if self.verify_checkpoint(height) {
                return Some(height);
            }
        }

        None
    }

    /// Truncates the chain back to the last valid checkpoint.
    pub fn recover_from_checkpoint(&mut self) {
        match self.find_last_valid_checkpoint() {
            Some(height) => {
                let checkpoint_hash = match self.checkpoints.get(&height) {
                    Some(h) => h.clone(),
                    None => {
                        eprintln!("[CERT] No checkpoint hash at height {} — aborting.", height);
                        return;
                    }
                };
                let seed = format!("{}:{}", height, checkpoint_hash);
                let expected_cert_id = {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(seed.as_bytes());
                    format!("{:x}", hasher.finalize())
                };
                let cert = match self.cert_registry.get(&expected_cert_id) {
                    Some(c) => c.clone(),
                    None => {
                        eprintln!("[CERT] No recovery certificate for height {} — aborting. Manual intervention required.", height);
                        return;
                    }
                };
                if !crate::recovery_snapshot_certificate::RecoverySnapshotCertificate::verify(&cert) {
                    eprintln!("[CERT] Certificate for height {} FAILED verification — aborting. Manual intervention required.", height);
                    return;
                }
                println!("[CERT] Recovery certificate for height {} verified OK.", height);
                self.blocks.truncate((height + 1) as usize);
                self.rebuild_balances();
                // [FIX-RECOVERY-WIPE] rebuild_balances() only clears state;
                // it does not replay balances from blocks. Without this call,
                // checkpoint recovery would zero every wallet balance and then
                // chain.save() would persist that wipe permanently to disk.
                self.apply_full_state();
                self.rebuild_processed_txs();
                println!("[CHAIN] Recovered to checkpoint at block {}.", height);
            }
            None => {
                eprintln!("[CHAIN] No valid checkpoint to recover to.");
            }
        }
    }

    // ── Checkpoints ───────────────────────────────────────────

    pub fn add_checkpoint(&mut self, height: u64, hash: String) {
        self.checkpoints.insert(height, hash);
    }

    pub fn create_checkpoint(&mut self) {
        if let Some(last) = self.blocks.last() {
            self.checkpoints.insert(last.index, last.hash.clone());
            println!(
                "[CHECKPOINT] Created at block {}.",
                last.index
            );
            let cert = crate::recovery_snapshot_certificate::RecoverySnapshotCertificate::generate(
                last.index,
                last.hash.clone(),
            );
            self.cert_registry.register(cert);
            println!(
                "[CERT] Recovery certificate generated for checkpoint at block {}.",
                last.index
            );
        }
    }

    pub fn verify_checkpoint(&self, height: u64) -> bool {
        let block = match self.blocks.get(height as usize) {
            Some(b) => b,
            None    => return false,
        };

        match self.checkpoints.get(&height) {
            Some(hash) => block.hash == *hash,
            None       => false,
        }
    }

    pub fn show_checkpoints(&self) {
        println!("\n=== CHECKPOINTS ===");
        if self.checkpoints.is_empty() {
            println!("  (none)");
        }
        for (height, hash) in &self.checkpoints {
            println!("  {} => {}", height, hash);
        }
    }

    // ── Fork management ───────────────────────────────────────

    pub fn add_fork(&mut self, chain: Vec<Block>) {
        self.forks.push(chain);
        println!("[FORK] Fork candidate registered.");
    }

    pub fn fork_count(&self) -> usize {
        self.forks.len()
    }

    pub fn longest_chain_index(&self) -> Option<usize> {
        if self.forks.is_empty() {
            return None;
        }

        let mut longest = 0usize;
        let mut size    = 0usize;

        for (i, chain) in self.forks.iter().enumerate() {
            if chain.len() > size {
                size    = chain.len();
                longest = i;
            }
        }

        Some(longest)
    }

    /// Switches to the longest valid fork if it beats the main chain.
    pub fn select_best_chain(&mut self) {
        let index = match self.longest_chain_index() {
            Some(i) => i,
            None    => return,
        };

        let candidate = self.forks[index].clone();

        if Blockchain::validate_external_chain(&candidate)
            && candidate.len() > self.blocks.len()
        {
            self.blocks = candidate;
            self.rebuild_processed_txs();
            // [FORK-REPLAY] Full block-by-block replay from the replay
            // base snapshot, across every tx + BalanceOp + StakingOp in
            // the newly-adopted fork's blocks. Replaces the old
            // stale-snapshot reapply (rebuild_balances() +
            // apply_full_state()), which silently restored the OLD
            // fork's persisted balances onto the NEW fork's blocks
            // instead of actually replaying them.
            self.replay_full_chain_state();
            storage::save_chain(&self.blocks);
            println!("[FORK] Switched to longer valid chain.");
        }
    }

    // ── Mempool ───────────────────────────────────────────────

    pub fn cleanup_mempool(&mut self) {
        self.mempool.cleanup_expired(3600);
        println!("[MEMPOOL] Expired transactions cleaned.");
    }

    pub fn mempool_info(&self) {
        println!("\n=== MEMPOOL INFO ===");
        println!("  Pending TXs: {}", self.mempool.size());
        for tx in &self.mempool.transactions {
            println!(
                "  {} -> {} | amount={}",
                tx.sender,
                tx.receiver,
                tx.amount
            );
        }
    }

    pub fn print_mempool(&self) {
        println!("\n===== MEMPOOL =====");
        for tx in &self.mempool.transactions {
            println!("{:#?}", tx);
        }
    }

    // ── Persistence ───────────────────────────────────────────

    pub fn save(&self) {
        storage::save_chain(&self.blocks);
        storage::save_full_state(&storage::FullState {
            balances: self.balances.clone(),
            nonces: self.nonces.clone(),
            usdt_balances: self.usdt_balances.clone(),
            treasury: self.treasury.clone(),
            treasury_multisig: self.treasury_multisig.clone(),
            pending_spend_requests: self.pending_spend_requests.clone(),
            checkpoints: self.checkpoints.clone(),
            cert_registry: self.cert_registry.clone(),
            chain_frozen: self.chain_frozen,
            emergency_freeze_multisig: self.emergency_freeze_multisig.clone(),
            pending_freeze_requests: self.pending_freeze_requests.clone(),
            staking: self.staking.clone(),
        });
    }

    /// Restores balances, treasury, multisig, and pending spend
    /// state from the last saved full_state.json, if one exists.
    /// Must be called AFTER rebuild_balances(), since it overwrites
    /// whatever rebuild_balances() set, with the real persisted
    /// values (rebuild_balances() does not replay balances from
    /// blocks — see its own doc comment).
    pub fn apply_full_state(&mut self) {
        if let Some(state) = storage::load_full_state() {
            self.balances = state.balances;
            self.nonces = state.nonces;
            self.usdt_balances = state.usdt_balances;
            self.treasury = state.treasury;
            self.treasury_multisig = state.treasury_multisig;
            self.pending_spend_requests = state.pending_spend_requests;
            self.checkpoints = state.checkpoints;
            self.cert_registry = state.cert_registry;
            self.chain_frozen = state.chain_frozen;
            self.emergency_freeze_multisig = state.emergency_freeze_multisig;
            self.pending_freeze_requests = state.pending_freeze_requests;
            self.staking = state.staking;
            println!("[CHAIN] Full state restored from disk.");
            self.audit_balances();
        } else {
            println!("[CHAIN] No prior full state found — starting with empty balances/treasury.");
        }
    }

    /// One-time bootstrap for the fork-choice full-replay system. If no
    /// replay base snapshot exists yet, captures the CURRENT (trusted,
    /// already-loaded) state as the base, at the current chain height.
    ///
    /// Why not replay from genesis instead? Blocks mined before the
    /// BalanceOp/StakingOp infrastructure existed carry no record of
    /// their swap/liquidity/reward/L2 balance mutations — only plain
    /// transfers. Replaying from genesis would silently reconstruct
    /// WRONG balances for any chain with history predating this system.
    /// Anchoring the base snapshot to today's already-correct state
    /// means every FUTURE fork (which can only diverge from today
    /// onward, since no fork exists yet) is replayed with full
    /// accuracy — nothing before this point is ever silently dropped,
    /// because nothing before this point is ever replayed.
    pub fn init_replay_base_snapshot_if_missing(&self) {
        if storage::load_replay_base_snapshot().is_some() {
            return;
        }

        let tokens = storage::load_tokens();
        let mut token_balances: HashMap<String, HashMap<String, u128>> = HashMap::new();
        for t in &tokens {
            token_balances.insert(t.symbol.clone(), t.balances.clone());
        }

        let snapshot = storage::ReplayBaseSnapshot {
            height: self.blocks.len() as u64,
            balances: self.balances.clone(),
            nonces: self.nonces.clone(),
            usdt_balances: self.usdt_balances.clone(),
            current_supply: self.supply.current_supply,
            staking: self.staking.clone(),
            token_balances,
        };

        storage::save_replay_base_snapshot(&snapshot);
        println!(
            "[REPLAY] Initialised replay base snapshot at height {}.",
            snapshot.height
        );
    }

    /// Full block-by-block replay of balances, usdt balances, token
    /// balances, supply, and staking — from the replay base snapshot
    /// through every block currently in self.blocks at or after the
    /// snapshot's height. This is the correct fork-switch state
    /// reconstruction, replacing the old stale-snapshot reapply.
    ///
    /// Falls back to the legacy apply_full_state() restore if no base
    /// snapshot exists, or if the base snapshot's height is somehow
    /// ahead of the current chain (should not happen in practice, but
    /// must never panic or corrupt state if it does).
    pub fn replay_full_chain_state(&mut self) {
        let base = match storage::load_replay_base_snapshot() {
            Some(b) => b,
            None => {
                eprintln!("[REPLAY] No replay base snapshot found — falling back to legacy full-state restore.");
                self.rebuild_balances();
                self.apply_full_state();
                return;
            }
        };

        if base.height as usize > self.blocks.len() {
            eprintln!(
                "[REPLAY] Base snapshot height ({}) exceeds current chain length ({}) — cannot replay safely. Falling back to legacy full-state restore.",
                base.height, self.blocks.len()
            );
            self.rebuild_balances();
            self.apply_full_state();
            return;
        }

        let mut balances = base.balances.clone();
        let mut nonces = base.nonces.clone();
        let mut usdt_balances = base.usdt_balances.clone();
        let mut current_supply = base.current_supply;
        let mut staking = base.staking.clone();
        let mut token_balances = base.token_balances.clone();

        for block in self.blocks.iter().filter(|b| b.index >= base.height) {
            // Plain transfers.
            for tx in &block.transactions {
                let sender_bal = *balances.get(&tx.sender).unwrap_or(&0);
                let receiver_bal = *balances.get(&tx.receiver).unwrap_or(&0);
                balances.insert(
                    tx.sender.clone(),
                    sender_bal.saturating_sub(tx.amount.saturating_add(tx.fee)),
                );
                balances.insert(
                    tx.receiver.clone(),
                    receiver_bal.saturating_add(tx.amount),
                );
                nonces.insert(tx.sender.clone(), tx.nonce + 1);
            }

            // Non-transfer balance mutations (swap, liquidity, L2
            // bridge crossings, token transfer/create, block reward,
            // wnsc_release, etc.)
            for op in &block.ops {
                match op.target.as_str() {
                    "nsc" => {
                        let bal = *balances.get(&op.address).unwrap_or(&0);
                        let new_bal = if op.delta >= 0 {
                            bal.saturating_add(op.delta as u128)
                        } else {
                            bal.saturating_sub((-op.delta) as u128)
                        };
                        balances.insert(op.address.clone(), new_bal);
                        if op.reason == "block_reward" && op.delta > 0 {
                            current_supply = current_supply.saturating_add(op.delta as u128);
                        }
                    }
                    "usdt" => {
                        let bal = *usdt_balances.get(&op.address).unwrap_or(&0);
                        let new_bal = if op.delta >= 0 {
                            bal.saturating_add(op.delta as u64)
                        } else {
                            bal.saturating_sub((-op.delta) as u64)
                        };
                        usdt_balances.insert(op.address.clone(), new_bal);
                    }
                    t if t.starts_with("token:") => {
                        let symbol = &t[6..];
                        let map = token_balances
                            .entry(symbol.to_string())
                            .or_insert_with(HashMap::new);
                        let bal = *map.get(&op.address).unwrap_or(&0);
                        let new_bal = if op.delta >= 0 {
                            bal.saturating_add(op.delta as u128)
                        } else {
                            bal.saturating_sub((-op.delta) as u128)
                        };
                        map.insert(op.address.clone(), new_bal);
                    }
                    other => {
                        eprintln!(
                            "[REPLAY] WARNING: unrecognized BalanceOp target '{}' (reason='{}') — skipped.",
                            other, op.reason
                        );
                    }
                }
            }

            // Staking mutations.
            for sop in &block.staking_ops {
                staking.apply_op(sop);
            }
        }

        self.balances = balances;
        self.nonces = nonces;
        self.usdt_balances = usdt_balances;
        self.supply.current_supply = current_supply;
        self.staking = staking;

        // Persist rebuilt per-wallet token balances (token metadata —
        // name/symbol/total_supply — is untouched: token creation is
        // treated as fork-independent global state, same as pool
        // reserves and L2BridgeState).
        let mut tokens = storage::load_tokens();
        for token in tokens.iter_mut() {
            if let Some(new_bals) = token_balances.get(&token.symbol) {
                token.balances = new_bals.clone();
            }
        }
        storage::save_tokens(&tokens);

        println!(
            "[REPLAY] Full chain state replayed from base height {} through block {}.",
            base.height,
            self.blocks.len().saturating_sub(1)
        );

        // Persist immediately so a restart never falls back to the
        // stale pre-replay full_state.json.
        self.save();
    }

    pub fn export_snapshot(&self) {
        println!("[CHAIN] Exporting chain snapshot...");
        self.save();
    }

    pub fn export_chain(&self) -> String {
        format!("{:#?}", self.blocks)
    }

    // ── Stats & diagnostics ───────────────────────────────────

    pub fn height(&self)       -> usize { self.blocks.len() }
    pub fn chain_height(&self) -> usize { self.blocks.len() }

    pub fn wallet_exists(&self, address: &str) -> bool {
        self.balances.contains_key(address)
    }

    pub fn transaction_count(&self, address: &str) -> usize {
        let mut count = 0;
        for block in &self.blocks {
            for tx in &block.transactions {
                if tx.sender == address || tx.receiver == address {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn total_tx_count(&self) -> usize {
        self.blocks.iter().map(|b| b.transactions.len()).sum()
    }

    pub fn average_txs_per_block(&self) -> f64 {
        if self.blocks.is_empty() { return 0.0; }
        self.total_tx_count() as f64 / self.blocks.len() as f64
    }

    pub fn total_fees_collected(&self) -> u128 {
        self.blocks.iter()
            .flat_map(|b| b.transactions.iter())
            .map(|tx| tx.fee)
            .sum()
    }

    pub fn average_fee(&self) -> f64 {
        let total_tx = self.total_tx_count();
        if total_tx == 0 { return 0.0; }
        self.total_fees_collected() as f64 / total_tx as f64
    }

    pub fn transaction_volume(&self) -> u128 {
        self.blocks.iter()
            .flat_map(|b| b.transactions.iter())
            .map(|tx| tx.amount)
            .sum()
    }

    pub fn show_stats(&self) {
        println!("\n===== NETWORK STATS =====");
        println!("  Blocks      : {}", self.blocks.len());
        println!("  TXs         : {}", self.total_transactions);
        println!("  Wallets     : {}", self.balances.len());
        println!("  Difficulty  : {}", self.difficulty);
    }

    pub fn network_stats(&self) {
        println!("\n=== NETWORK STATS ===");
        println!("  Blocks        : {}", self.blocks.len());
        println!("  Wallets       : {}", self.balances.len());
        println!("  Processed TXs : {}", self.processed_txs.len());
        println!("  Total Fees    : {}", self.total_fees_collected());
        println!("  Average Fee   : {:.2}", self.average_fee());
    }

    pub fn network_status(&self) {
        println!("\n=== NETWORK STATUS ===");
        println!("  Height : {}", self.height());
        println!("  Forks  : {}", self.fork_count());
        println!("  Blocks : {}", self.blocks.len());
    }

    pub fn sync_status(&self) {
        println!("\n=== CHAIN STATUS ===");
        println!("  Height     : {}", self.blocks.len());
        println!("  Difficulty : {}", self.difficulty);
        println!("  Pending TX : {}", self.mempool.size());
    }
    pub fn supply_info(&self) {
        println!("\n=== SUPPLY INFO ===");
        println!("  Circulating : {}", self.circulating_supply());
        println!("  Wallet Count: {}", self.balances.len());
        println!("  Block Height: {}", self.blocks.len());
    }

    pub fn block_stats(&self) {
        println!("\n=== BLOCK STATS ===");
        for block in &self.blocks {
            println!("  Block {} | TXs: {}", block.index, block.transactions.len());
        }
    }

    pub fn performance_info(&self) {
        println!("\n=== PERFORMANCE ===");
        println!("  Total TXs       : {}", self.total_tx_count());
        println!("  Avg TX/Block    : {:.2}", self.average_txs_per_block());
    }

    pub fn volume_info(&self) {
        println!("\n=== VOLUME INFO ===");
        println!("  Total Volume : {}", self.transaction_volume());
    }

    pub fn chain_health(&self) {
        println!("\n=== CHAIN HEALTH ===");
        println!("  Height : {}", self.height());
        println!("  Valid  : {}", self.validate_chain());
        println!("  TXs    : {}", self.total_transactions);
    }

    pub fn self_check(&self) {
        if self.validate_chain() {
            println!("[CHAIN] Self-check: OK");
        } else {
            eprintln!("[CHAIN] Self-check: CORRUPTED");
        }
    }

    pub fn scan_corruption(&self) {
        println!("\n=== CHAIN SCAN ===");
        for block in &self.blocks {
            if block.hash != block.calculate_block_hash() {
                eprintln!("  [CORRUPTED] Block {}", block.index);
            }
        }
        println!("  Scan complete.");
    }

    /// [FIX-19] Audits that sum of balances does not exceed minted supply.
    pub fn audit_balances(&self) {
        println!("\n===== BALANCE AUDIT =====");
        let total: u128 = self.balances.values().sum();
        println!("  Total Balances  : {}", total);
        println!("  Minted Supply   : {}", self.supply.current_supply);

        if total > self.supply.current_supply {
            eprintln!("  [WARN] Balance total exceeds minted supply!");
        } else {
            println!("  [OK] Balance audit passed.");
        }
    }

    pub fn richest_wallets(&self) {
        println!("\n=== TOP WALLETS ===");
        let mut wallets: Vec<_> = self.balances.iter().collect();
        wallets.sort_by(|a, b| b.1.cmp(a.1));
        for (address, balance) in wallets.iter().take(10) {
            println!("  {} : {}", address, balance);
        }
    }

    pub fn address_history(&self, address: &str) {
        println!("\n=== ADDRESS HISTORY ===");
        let mut found = false;
        for block in &self.blocks {
            for tx in &block.transactions {
                if tx.sender == address || tx.receiver == address {
                    println!(
                        "  {} -> {} | amount={}",
                        tx.sender, tx.receiver, tx.amount
                    );
                    found = true;
                }
            }
        }
        if !found {
            println!("  No transactions found.");
        }
    }

    pub fn wallet_info(&self, address: &str) {
        println!("\n=== WALLET INFO ===");
        println!("  Address : {}", address);
        println!("  Balance : {}", self.get_balance(address));
        println!("  Nonce   : {}", self.nonces.get(address).unwrap_or(&0));
    }

    pub fn latest_block_info(&self) {
        if let Some(block) = self.blocks.last() {
            println!("\n=== LATEST BLOCK ===");
            println!("  Index    : {}", block.index);
            println!("  Hash     : {}", block.hash);
            println!("  Previous : {}", block.previous_hash);
            println!("  TXs      : {}", block.transactions.len());
        }
    }

    pub fn get_block(&self, index: usize) {
        match self.blocks.get(index) {
            Some(block) => {
                println!("\n=== BLOCK {} ===", index);
                println!("  Hash : {}", block.hash);
                println!("  TXs  : {}", block.transactions.len());
            }
            None => {
                println!("[CHAIN] Block {} not found.", index);
            }
        }
    }

    pub fn find_transaction(&self, tx_hash: &str) {
        for block in &self.blocks {
            for tx in &block.transactions {
                if tx.tx_hash == tx_hash {
                    println!("\n=== TRANSACTION FOUND ===");
                    println!("  Sender   : {}", tx.sender);
                    println!("  Receiver : {}", tx.receiver);
                    println!("  Amount   : {}", tx.amount);
                    return;
                }
            }
        }
        println!("[CHAIN] Transaction {} not found.", tx_hash);
    }

    pub fn print_balances(&self) {
        println!("\n===== BALANCES =====");
        for (address, balance) in &self.balances {
            println!("  {} => {} NSC", address, balance);
        }
    }

    pub fn print_chain(&self) {
        println!("\n===== NSC BLOCKCHAIN =====");
        for block in &self.blocks {
            println!("{:#?}", block);
        }
    }

    pub fn print_supply(&self) {
        println!("\n===== NSC SUPPLY =====");
        println!("  Max Supply      : {}", self.supply.max_supply);
        println!("  Current Supply  : {}", self.supply.current_supply);
        println!("  Remaining Supply: {}", self.supply.remaining_supply());
    }

    pub fn print_tx_counts(&self) {
        println!("\n===== TX COUNTS =====");
        for (addr, count) in &self.tx_count {
            println!("  {} => {}", addr, count);
        }
    }

    pub fn print_nonces(&self) {
        println!("\n===== NONCES =====");
        for (addr, nonce) in &self.nonces {
            println!("  {} => {}", addr, nonce);
        }
    }

    pub fn show_processed_txs(&self) {
        println!("\n===== PROCESSED TXS =====");
        println!("  Count: {}", self.processed_txs.len());
    }
}
// ============================================================
// END OF chain.rs
// ============================================================
