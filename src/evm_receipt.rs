// ============================================================
// NUSACOIN (NSC) — evm_receipt.rs
// Persisted EVM transaction receipts + logs, recorded at block-
// confirmation time (inside mine_pending_transactions()), not at
// mempool-accept time. eth_getTransactionReceipt only ever reports
// a tx that has actually been mined into a block, with real
// block_number/block_hash/tx_index — unlike the old ephemeral
// EvmState.receipts map (evm_rpc.rs), which recorded receipts at
// eth_sendRawTransaction time with fake block data and was lost
// on every node restart.
// ============================================================

use serde::{Serialize, Deserialize};

/// A single EVM-style event log entry attached to a receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmLog {
    /// Contract/address that emitted this log. For native NSC
    /// transfers routed through the EVM RPC path (no real
    /// contract), this is the zero address.
    pub address:   String,
    /// Up to 4 indexed topics, 32-byte hex ("0x...").
    pub topics:    Vec<String>,
    /// ABI-encoded non-indexed data, hex ("0x...").
    pub data:      String,
    /// Position of this log within the block.
    pub log_index: u64,
}

/// A confirmed EVM transaction receipt. Only created once a tx has
/// actually been mined into a block — never at mempool-accept time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmReceipt {
    pub tx_hash:             String,
    pub from:                String,
    pub to:                  Option<String>,
    pub status:              bool,
    pub block_number:        u64,
    pub block_hash:          String,
    pub tx_index:            u64,
    pub gas_used:            u64,
    pub cumulative_gas_used: u64,
    pub logs:                Vec<EvmLog>,
    pub contract_address:    Option<String>,
}
