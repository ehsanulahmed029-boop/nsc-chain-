// ============================================================
// NUSACOIN (NSC) — Supply Management
// ============================================================
// Max Supply     : 25,000,000 NSC (hard cap)
// Genesis Supply : 2,000,000 NSC (pre-minted)
// Block Rewards  : Halving schedule
// ============================================================

use crate::genesis;

#[derive(Debug, Clone)]
pub struct Supply {
    pub max_supply:     u128,
    pub current_supply: u128,
}

impl Supply {
    pub fn new() -> Self {
        Self {
            max_supply:     genesis::MAX_SUPPLY,
            // Genesis supply pre-minted at startup.
            current_supply: genesis::GENESIS_SUPPLY,
        }
    }

    /// Block reward schedule with halving.
    /// Rewards decrease as chain matures.
    pub fn block_reward(&self, height: u64) -> u128 {
        if height < 100 {
            50 * genesis::DECIMALS   // Early blocks — high reward
        } else if height < 500 {
            25 * genesis::DECIMALS   // Phase 2
        } else if height < 1_000 {
            12 * genesis::DECIMALS   // Phase 3
        } else if height < 5_000 {
            6 * genesis::DECIMALS    // Phase 4
        } else if height < 10_000 {
            3 * genesis::DECIMALS    // Phase 5
        } else {
            1 * genesis::DECIMALS    // Final phase — 1 NSC per block
        }
    }

    /// Mints new coins if supply cap allows.
    /// Returns true if mint succeeded.
    pub fn mint(&mut self, amount: u128) -> bool {
        if amount == 0 {
            return false;
        }
        if self.current_supply
            .saturating_add(amount) > self.max_supply
        {
            return false;
        }
        self.current_supply += amount;
        true
    }

    /// Returns remaining mintable supply.
    pub fn remaining_supply(&self) -> u128 {
        self.max_supply
            .saturating_sub(self.current_supply)
    }

    /// Returns circulating supply as percentage of max.
    pub fn circulating_percent(&self) -> f64 {
        if self.max_supply == 0 {
            return 0.0;
        }
        (self.current_supply as f64
            / self.max_supply as f64)
            * 100.0
    }

    /// Returns true if supply cap has been reached.
    pub fn is_maxed(&self) -> bool {
        self.current_supply >= self.max_supply
    }
}
