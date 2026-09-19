import sys

path = "/root/nsc-chain/src/network.rs"
with open(path, "r") as f:
    content = f.read()

anchor = '''                        // IMPORTANT: this appends a single block without
                        // re-running full chain validation or re-checking
                        // every transaction's balance/nonce against
                        // current chain state. That re-check is the
                        // "missing function" flagged earlier in review —
                        // it must exist and be called HERE, before this
                        // push, not assumed. Do not deploy this file until
                        // that per-block-apply function exists and is
                        // wired in at this exact point.
                        chain.blocks.push(block.clone());
                        chain.save();
                        println!("[NET] Accepted block {} from peer {}.", block.index, ip);'''

count = content.count(anchor)
if count != 1:
    print(f"FATAL: anchor found {count} times, expected 1. Aborting.")
    sys.exit(1)

replacement = '''                        // [P1-FIX 2026-08-16] Per-transaction validation
                        // now runs BEFORE this block is appended. Every
                        // tx's signature, nonce sequencing, and sender
                        // balance is checked against current chain state
                        // via validate_incoming_block() (chain.rs). If
                        // any transaction fails, the WHOLE block is
                        // rejected — an already-mined peer block's tx
                        // list is fixed by its hash/PoW, so transactions
                        // cannot be selectively dropped the way
                        // mine_pending_transactions() drops bad txs from
                        // its own mempool.
                        if !chain.validate_incoming_block(&block) {
                            eprintln!(
                                "[NET] Peer {} sent block {} that failed transaction validation. Rejecting.",
                                ip, block.index
                            );
                            return false;
                        }

                        chain.apply_incoming_block_transactions(&block);
                        chain.blocks.push(block.clone());
                        chain.save();
                        println!("[NET] Accepted block {} from peer {}.", block.index, ip);'''

content = content.replace(anchor, replacement, 1)

with open(path, "w") as f:
    f.write(content)

print("network.rs patched successfully.")
