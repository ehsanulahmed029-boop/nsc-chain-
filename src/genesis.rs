// ============================================================
// NUSACOIN (NSC) — Genesis Configuration
// ============================================================
// Max Supply    : 25,000,000 NSC
// Genesis Supply: 20,000,000 NSC (sent to founder wallet)
// Block Time    : 60 seconds
// ============================================================

/// Total coins minted at genesis block.
/// Sent directly to the founder wallet.
pub const DECIMALS: u128 = 1_000_000_000_000_000_000; // 18 decimals

pub const GENESIS_SUPPLY: u128 = 2_000_000 * DECIMALS;

/// Target block time in seconds.
pub const GENESIS_BLOCK_TIME: u64 = 60;

/// Hard cap — must match Supply::max_supply.
pub const MAX_SUPPLY: u128 = 25_000_000 * DECIMALS;

/// Founder wallet receives genesis supply.
/// Change this to your real NSC wallet address.
pub const GENESIS_WALLET: &str = "NSC19a650ed2a11d340";
