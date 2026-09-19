import sys

path = "/root/nsc-chain/src/chain.rs"
with open(path, "r") as f:
    content = f.read()

anchor = '''    pub fn increment_nonce(&mut self, sender: &str) {
        let current = *self.nonces.get(sender).unwrap_or(&0);
        self.nonces.insert(sender.to_string(), current + 1);
    }
'''

count = content.count(anchor)
if count != 1:
    print(f"FATAL: anchor found {count} times, expected 1. Aborting.")
    sys.exit(1)

new_functions = '''    pub fn increment_nonce(&mut self, sender: &str) {
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

            if tx.fee > 0 {
                let _ = self.treasury.deposit(tx.fee);
            }

            self.nonces.insert(tx.sender.clone(), tx.nonce.saturating_add(1));
            self.processed_txs.insert(tx.tx_hash.clone());
        }
    }
'''

content = content.replace(anchor, new_functions, 1)

with open(path, "w") as f:
    f.write(content)

print("chain.rs patched successfully.")
