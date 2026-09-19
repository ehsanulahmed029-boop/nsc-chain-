// ============================================================
// L2 Bridge — L1-side Escrow / Lock-Mint / Burn-Unlock module
// Phase 1: Sequencer + re-execution fraud proof
// ============================================================
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use sha2::{Sha256, Digest};

pub const CHALLENGE_WINDOW_SECS: u64 = 3600; // 60 minutes
pub const SEQUENCER_BOND_MIN: u128 = 10_000_000_000_000_000_000; // 10 NSC (18 decimals) - tune later

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct L2Deposit {
    pub depositor: String,
    pub amount: String,       // u128 as string (avoid json! panic pattern)
    pub l2_address: String,
    pub timestamp: u64,
    pub processed: bool,
}

/// A single balance-affecting operation, recorded so that a batch's
/// claimed state_root can be independently re-derived later rather than
/// trusted from whoever submits the batch. This is the actual data
/// availability payload -- not just a hash of it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum L2OpKind {
    Deposit,
    Transfer,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct L2Op {
    pub kind: L2OpKind,
    pub from: String,   // empty for Deposit (minted, not sent by anyone on L2)
    pub to: String,
    pub amount: String, // u128 as string
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct L2Batch {
    pub batch_id: u64,
    pub state_root: String,       // hash of L2 balances after replaying `ops` (deposit+transfer ops only; withdrawal burns are tracked separately and are not part of this root -- see request_withdrawal)
    pub tx_data_hash: String,     // hash of `ops` (real data-availability commitment, recomputed server-side, not client-trusted)
    #[serde(default)]
    pub ops: Vec<L2Op>,           // the actual ops included in this batch, so state_root can be independently re-derived
    pub prev_state_root: String,
    pub sequencer: String,
    pub submitted_at: u64,
    pub challenge_deadline: u64,
    pub finalized: bool,
    pub challenged: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct L2Withdrawal {
    pub withdrawal_id: u64,
    pub l2_burner: String,
    pub l1_recipient: String,
    pub amount: String,
    pub batch_id: u64,           // which batch this withdrawal was included in
    pub requested_at: u64,
    pub challenge_deadline: u64,
    pub finalized: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct L2BondBurn {
    pub sequencer: String,
    pub amount: String,
    pub batch_id: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct L2BridgeState {
    #[serde(default)]
    pub l2_balances: HashMap<String, u128>,
    #[serde(default)]
    pub deposits: Vec<L2Deposit>,
    #[serde(default)]
    pub batches: Vec<L2Batch>,
    #[serde(default)]
    pub withdrawals: Vec<L2Withdrawal>,
    #[serde(default)]
    pub sequencer_address: String,
    #[serde(default)]
    pub sequencer_bond: String,     // u128 as string
    #[serde(default)]
    pub sequencer_bond_locked: bool,
    #[serde(default)]
    pub next_batch_id: u64,
    #[serde(default)]
    pub next_withdrawal_id: u64,
    #[serde(default)]
    pub pending_ops: Vec<L2Op>,   // deposit/transfer ops since the last batch was submitted
    #[serde(default)]
    pub burn_log: Vec<L2BondBurn>,
}

impl L2BridgeState {
    pub fn get_l2_balance(&self, address: &str) -> u128 {
        *self.l2_balances.get(address).unwrap_or(&0)
    }

    pub fn credit_l2_balance(&mut self, address: &str, amount: u128) -> Result<(), String> {
        let current = self.get_l2_balance(address);
        let new_bal = current.checked_add(amount).ok_or("balance overflow")?;
        self.l2_balances.insert(address.to_string(), new_bal);
        Ok(())
    }

    pub fn debit_l2_balance(&mut self, address: &str, amount: u128) -> Result<(), String> {
        let current = self.get_l2_balance(address);
        let new_bal = current.checked_sub(amount).ok_or("insufficient L2 balance")?;
        self.l2_balances.insert(address.to_string(), new_bal);
        Ok(())
    }

    /// Instant, off-chain L2 transfer. No L1 interaction -- this is
    /// the whole point of L2 (fast, free, batched only periodically).
    pub fn l2_transfer(&mut self, from: &str, to: &str, amount: u128, now: u64) -> Result<(), String> {
        if amount == 0 {
            return Err("amount must be greater than zero".to_string());
        }
        if from == to {
            return Err("cannot transfer to self".to_string());
        }
        self.debit_l2_balance(from, amount)?;
        // credit_l2_balance cannot fail after a successful debit except on
        // overflow; if it does, roll back the debit so state stays consistent.
        if let Err(e) = self.credit_l2_balance(to, amount) {
            let _ = self.credit_l2_balance(from, amount); // rollback
            return Err(e);
        }
        // Record the op only after both sides of the balance change have
        // definitely succeeded -- a rolled-back transfer must never appear
        // in pending_ops, since it never actually happened.
        self.pending_ops.push(L2Op {
            kind: L2OpKind::Transfer,
            from: from.to_string(),
            to: to.to_string(),
            amount: amount.to_string(),
            timestamp: now,
        });
        Ok(())
    }

    /// Marks a pending deposit as processed and credits the L2 balance.
    /// Called by the sequencer when building/finalizing a batch that
    /// includes this deposit.
    pub fn process_deposit(&mut self, depositor: &str, timestamp: u64) -> Result<(), String> {
        let amount: u128 = {
            let dep = self.deposits.iter_mut()
                .find(|d| d.depositor == depositor && d.timestamp == timestamp && !d.processed)
                .ok_or("deposit not found or already processed")?;
            dep.processed = true;
            dep.amount.parse().map_err(|_| "corrupt deposit amount")?
        };
        let l2_address = self.deposits.iter()
            .find(|d| d.depositor == depositor && d.timestamp == timestamp)
            .map(|d| d.l2_address.clone())
            .unwrap_or_else(|| depositor.to_string());
        self.credit_l2_balance(&l2_address, amount)?;
        // Record the op only after the credit has definitely succeeded.
        self.pending_ops.push(L2Op {
            kind: L2OpKind::Deposit,
            from: String::new(),
            to: l2_address,
            amount: amount.to_string(),
            timestamp,
        });
        Ok(())
    }

    pub fn new() -> Self {
        Self { next_batch_id: 1, next_withdrawal_id: 1, ..Default::default() }
    }

    /// L1 -> L2 deposit: caller must have already verified personal_sign
    /// (message format: "NSC_L2_DEPOSIT:{amount}:{l2_address}:{timestamp}")
    /// and already debited depositor's L1 balance via checked_sub before calling this.
    pub fn record_deposit(&mut self, depositor: String, amount: u128, l2_address: String, timestamp: u64) {
        self.deposits.push(L2Deposit {
            depositor,
            amount: amount.to_string(),
            l2_address,
            timestamp,
            processed: false,
        });
    }

    /// Sequencer submits a new batch. Requires bond to be locked.
    /// [FIX, 2026-08-19] state_root and tx_data_hash are no longer accepted
    /// from the caller -- they were previously fully client-supplied and
    /// never checked against anything, i.e. no real verification existed.
    /// Now both are computed server-side from `pending_ops`, the actual
    /// deposit/transfer operations recorded since the last batch. This is
    /// the real data-availability payload; the batch stores `ops` itself,
    /// not just a hash of it, so it can be independently replayed later
    /// (see challenge_batch).
    pub fn submit_batch(&mut self, sequencer: String, now: u64) -> Result<u64, String> {
        if !self.sequencer_bond_locked {
            return Err("sequencer bond not locked — cannot submit batch".to_string());
        }
        if sequencer != self.sequencer_address {
            return Err("only registered sequencer may submit batches".to_string());
        }
        // ── Credit any pending deposits into this batch. Simplified
        // model: every unprocessed deposit at submission time is folded
        // into the batch being submitted. A production sequencer would
        // decide deposit inclusion explicitly.
        // process_deposit() appends to pending_ops itself, so this must
        // run BEFORE pending_ops is drained into the batch below.
        let pending: Vec<(String, u64)> = self.deposits.iter()
            .filter(|d| !d.processed)
            .map(|d| (d.depositor.clone(), d.timestamp))
            .collect();
        for (depositor, timestamp) in pending {
            // Best-effort: a single corrupt deposit should not block the
            // whole batch, but should be logged for operator review.
            if let Err(e) = self.process_deposit(&depositor, timestamp) {
                eprintln!("[L2] Failed to process deposit {}:{} during batch submit: {}", depositor, timestamp, e);
            }
        }

        // Drain pending_ops into this batch -- these are the ops actually
        // being committed. If there are none, the batch is still valid
        // (an empty batch with an unchanged state_root relative to prior
        // cumulative balances is a legitimate, if pointless, checkpoint).
        let ops: Vec<L2Op> = std::mem::take(&mut self.pending_ops);
        let tx_data_hash = hash_ops(&ops);

        // state_root = hash of cumulative L2 balances after replaying
        // every op from every batch so far (genesis through this one).
        // Using current l2_balances directly here (rather than replaying)
        // is safe and equivalent because ops in `ops` have already been
        // applied to l2_balances as they occurred -- l2_balances IS the
        // post-state. challenge_batch() independently re-derives the same
        // thing from scratch via ops replay, which is what makes this
        // checkable rather than merely asserted.
        let state_root = hash_balances(&self.l2_balances);

        let prev_root = self.batches.last().map(|b| b.state_root.clone()).unwrap_or_default();
        let batch = L2Batch {
            batch_id: self.next_batch_id,
            state_root,
            tx_data_hash,
            ops,
            prev_state_root: prev_root,
            sequencer,
            submitted_at: now,
            challenge_deadline: now + CHALLENGE_WINDOW_SECS,
            finalized: false,
            challenged: false,
        };
        let id = batch.batch_id;
        self.batches.push(batch);
        self.next_batch_id += 1;
        Ok(id)
    }

    /// Anyone can challenge a batch within the window. Unlike the previous
    /// placeholder, this no longer trusts a client-supplied recomputed_root
    /// at all: it independently replays every op from every batch from
    /// genesis up to and including the challenged batch, and compares the
    /// resulting hash against the batch's claimed state_root itself. A
    /// mismatch here can only mean the stored batch data is internally
    /// inconsistent (e.g. tampered ops, or a bug in submit_batch), since
    /// the replay uses only data already committed on-chain (batch.ops),
    /// not anything supplied fresh by the caller.
    pub fn challenge_batch(&mut self, batch_id: u64, now: u64) -> Result<bool, String> {
        let batch_index = self.batches.iter().position(|b| b.batch_id == batch_id)
            .ok_or("batch not found")?;
        if self.batches[batch_index].finalized {
            return Err("batch already finalized — too late to challenge".to_string());
        }
        if now > self.batches[batch_index].challenge_deadline {
            return Err("challenge window expired".to_string());
        }

        // Replay every op from every batch from genesis through batch_index,
        // in order, and compute the resulting balances/hash independently.
        let mut replayed: HashMap<String, u128> = HashMap::new();
        for b in &self.batches[..=batch_index] {
            for op in &b.ops {
                let amount: u128 = match op.amount.parse() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                match op.kind {
                    L2OpKind::Deposit => {
                        let bal = replayed.entry(op.to.clone()).or_insert(0);
                        *bal = bal.saturating_add(amount);
                    }
                    L2OpKind::Transfer => {
                        let from_bal = replayed.entry(op.from.clone()).or_insert(0);
                        *from_bal = from_bal.saturating_sub(amount);
                        let to_bal = replayed.entry(op.to.clone()).or_insert(0);
                        *to_bal = to_bal.saturating_add(amount);
                    }
                }
            }
        }
        let recomputed_root = hash_balances(&replayed);
        let recomputed_tx_hash = hash_ops(&self.batches[batch_index].ops);

        let batch = &mut self.batches[batch_index];
        if recomputed_root != batch.state_root || recomputed_tx_hash != batch.tx_data_hash {
            batch.challenged = true;
            let burned_sequencer = self.sequencer_address.clone();
            let burned_amount = self.sequencer_bond.clone();
            self.sequencer_bond_locked = false; // slash: bond forfeited, sequencer suspended
            self.sequencer_bond = "0".to_string(); // [FIX, mainnet-prep] forfeited bond is burned, never re-credited to anyone
            self.burn_log.push(L2BondBurn {
                sequencer: burned_sequencer,
                amount: burned_amount,
                batch_id,
                timestamp: now,
            });
            return Ok(true); // fraud proven
        }
        Ok(false) // no fraud, independently-replayed root matches
    }

    /// Finalize a batch once challenge window has passed with no successful challenge.
    pub fn finalize_batch(&mut self, batch_id: u64, now: u64) -> Result<(), String> {
        let batch = self.batches.iter_mut().find(|b| b.batch_id == batch_id)
            .ok_or("batch not found")?;
        if batch.challenged {
            return Err("cannot finalize a successfully-challenged batch".to_string());
        }
        if now < batch.challenge_deadline {
            return Err("challenge window still open".to_string());
        }
        batch.finalized = true;
        Ok(())
    }

    /// Scan all batches for ones whose challenge window has passed with no
    /// successful challenge, and finalize them. Called periodically from the
    /// node heartbeat loop so finalization never depends on any single user
    /// or admin action (permissionless liveness) -- previously finalize_batch()
    /// had zero call sites anywhere, so no batch could ever be finalized and
    /// no withdrawal could ever complete. Returns the batch_ids finalized in
    /// this call, for logging/persistence by the caller.
    pub fn auto_finalize_eligible_batches(&mut self, now: u64) -> Vec<u64> {
        let eligible: Vec<u64> = self.batches.iter()
            .filter(|b| !b.finalized && !b.challenged && now >= b.challenge_deadline)
            .map(|b| b.batch_id)
            .collect();
        let mut finalized_ids = Vec::new();
        for id in eligible {
            if self.finalize_batch(id, now).is_ok() {
                finalized_ids.push(id);
            }
        }
        finalized_ids
    }

    /// L2 -> L1 withdrawal request (called after burn confirmed in an L2 batch)
    pub fn request_withdrawal(&mut self, l2_burner: String, l1_recipient: String, amount: u128, batch_id: u64, now: u64) -> Result<u64, String> {
        if amount == 0 {
            return Err("amount must be greater than zero".to_string());
        }
        if !self.batches.iter().any(|b| b.batch_id == batch_id) {
            return Err("batch_id not found".to_string());
        }
        // Debit first: if this fails (insufficient balance), no withdrawal
        // record is created and no funds are locked up incorrectly.
        // Previously this method never debited at all -- a double-spend
        // gap allowing repeated withdrawal requests against the same
        // L2 balance. Fixed here.
        self.debit_l2_balance(&l2_burner, amount)?;
        // [FRAUD-PROOF-FIX, 2026-09-12] Previously this debit was applied
        // directly to l2_balances with NO corresponding L2Op recorded.
        // submit_batch() computes state_root by hashing the live
        // l2_balances (which already reflects this debit), but
        // challenge_batch() independently REPLAYS only the ops list from
        // scratch -- with no op recorded here, that replay would never
        // see this debit, so its recomputed root would permanently
        // mismatch the batch's claimed root for any batch containing a
        // withdrawal. That would make challenge_batch() report "fraud
        // proven" against an honest sequencer purely because someone
        // withdrew, incorrectly slashing and burning their bond. Recording
        // this as a Transfer-kind op (to a burn address, since the funds
        // are leaving L2 for L1, not going to another L2 account) makes
        // the withdrawal debit part of the same replayable, verifiable
        // history as deposits and transfers.
        self.pending_ops.push(L2Op {
            kind: L2OpKind::Transfer,
            from: l2_burner.clone(),
            to: "L2_WITHDRAWAL_BURN".to_string(),
            amount: amount.to_string(),
            timestamp: now,
        });
        let id = self.next_withdrawal_id;
        self.withdrawals.push(L2Withdrawal {
            withdrawal_id: id,
            l2_burner,
            l1_recipient,
            amount: amount.to_string(),
            batch_id,
            requested_at: now,
            challenge_deadline: now + CHALLENGE_WINDOW_SECS,
            finalized: false,
        });
        self.next_withdrawal_id += 1;
        Ok(id)
    }

    /// Finalize withdrawal (unlock on L1) — only after batch is finalized
    /// AND withdrawal's own challenge window has passed.
    pub fn can_finalize_withdrawal(&self, withdrawal_id: u64, now: u64) -> Result<&L2Withdrawal, String> {
        let w = self.withdrawals.iter().find(|w| w.withdrawal_id == withdrawal_id)
            .ok_or("withdrawal not found")?;
        if w.finalized {
            return Err("already finalized".to_string());
        }
        let batch = self.batches.iter().find(|b| b.batch_id == w.batch_id)
            .ok_or("associated batch not found")?;
        if !batch.finalized {
            return Err("associated batch not yet finalized".to_string());
        }
        if now < w.challenge_deadline {
            return Err("withdrawal challenge window still open".to_string());
        }
        Ok(w)
    }

    /// True if any batch is still within its own challenge window. Bond
    /// must never be released while this is true -- otherwise a sequencer
    /// could submit fraud and reclaim their bond before anyone can challenge it.
    pub fn has_open_challenge_window(&self, now: u64) -> bool {
        self.batches.iter().any(|b| !b.finalized && !b.challenged && now < b.challenge_deadline)
    }

    /// Sequencer voluntarily reclaims their locked bond. Only allowed once
    /// every submitted batch is past its challenge window (finalized cleanly,
    /// or already challenged -- in which case bond was already zeroed by
    /// challenge_batch() and this correctly errors since bond_locked is false).
    pub fn unbond_sequencer(&mut self, sequencer: String, now: u64) -> Result<u128, String> {
        if !self.sequencer_bond_locked {
            return Err("no bond currently locked".to_string());
        }
        if sequencer != self.sequencer_address {
            return Err("only the registered sequencer may unbond".to_string());
        }
        if self.has_open_challenge_window(now) {
            return Err("cannot unbond: at least one batch is still within its challenge window".to_string());
        }
        let amount: u128 = self.sequencer_bond.parse().map_err(|_| "corrupt bond amount".to_string())?;
        self.sequencer_bond_locked = false;
        self.sequencer_bond = "0".to_string();
        Ok(amount)
    }
}

pub fn hash_tx_data(tx_blob: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tx_blob);
    format!("{:x}", hasher.finalize())
}

/// Hash of a serialized op list -- the real tx_data_hash, computed from
/// actual data rather than trusted from the submitter.
pub fn hash_ops(ops: &[L2Op]) -> String {
    let json = serde_json::to_string(ops).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Deterministically replays a sequence of ops from an empty balance sheet
/// and returns the resulting balances. This is the actual re-execution:
/// given the same ops, this always produces the same result, so a claimed
/// state_root can be checked by calling this and hashing the output.
pub fn compute_balances_from_ops(ops: &[L2Op]) -> HashMap<String, u128> {
    let mut balances: HashMap<String, u128> = HashMap::new();
    for op in ops {
        let amount: u128 = match op.amount.parse() {
            Ok(a) => a,
            Err(_) => continue, // corrupt op amount -- skip rather than panic; will show up as a root mismatch
        };
        match op.kind {
            L2OpKind::Deposit => {
                let bal = balances.entry(op.to.clone()).or_insert(0);
                *bal = bal.saturating_add(amount);
            }
            L2OpKind::Transfer => {
                let from_bal = balances.entry(op.from.clone()).or_insert(0);
                *from_bal = from_bal.saturating_sub(amount);
                let to_bal = balances.entry(op.to.clone()).or_insert(0);
                *to_bal = to_bal.saturating_add(amount);
            }
        }
    }
    balances
}

/// Hashes a balance map deterministically (sorted by address) so the same
/// balances always produce the same hash regardless of HashMap iteration order.
pub fn hash_balances(balances: &HashMap<String, u128>) -> String {
    let mut keys: Vec<&String> = balances.keys().collect();
    keys.sort();
    let mut s = String::new();
    for k in keys {
        s.push_str(k);
        s.push(':');
        s.push_str(&balances[k].to_string());
        s.push(';');
    }
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}
