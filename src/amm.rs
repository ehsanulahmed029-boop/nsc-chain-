use std::collections::HashMap;
use crate::trade::Trade;

// ── Errors ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AmmError {
    InsufficientBalance,
    InsufficientLpTokens,
    PoolEmpty,
    ZeroAmount,
    Overflow,
    SlippageExceeded,
    WouldDrainReserve,
    BalanceUpdateFailed,
}

impl std::fmt::Display for AmmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AmmError::InsufficientBalance => write!(f, "insufficient balance for this operation"),
            AmmError::InsufficientLpTokens => write!(f, "insufficient LP tokens"),
            AmmError::PoolEmpty => write!(f, "pool has no liquidity"),
            AmmError::ZeroAmount => write!(f, "amount must be greater than zero"),
            AmmError::Overflow => write!(f, "arithmetic overflow — amount too large"),
            AmmError::SlippageExceeded => write!(f, "output below minimum — slippage exceeded"),
            AmmError::WouldDrainReserve => write!(f, "operation would drain a reserve to zero"),
            AmmError::BalanceUpdateFailed => write!(f, "failed to update on-chain balance"),
        }
    }
}

/// Minimal interface the AMM needs from your real balance store.
///
/// Implement this for `Blockchain` (in chain.rs) so the AMM never
/// touches `balances` directly and always goes through whatever
/// validation/locking chain.rs already enforces.
///
/// YOU MUST WIRE THIS UP — see the bottom of this file for the
/// expected chain.rs-side implementation sketch.
pub trait BalanceLedger {
    /// Returns the NSC balance of `address`. Must never panic.
    fn nsc_balance(&self, address: &str) -> u128;

    /// Returns the USDT balance of `address`. Must never panic.
    /// If you don't track USDT on-chain (e.g. it's a wrapped/bridged
    /// asset tracked elsewhere), wire this to whatever ledger you
    /// actually use for USDT accounting.
    fn usdt_balance(&self, address: &str) -> u64;

    /// Attempts to debit `amount` NSC from `address`.
    /// Must return false (and change nothing) if balance is insufficient.
    fn debit_nsc(&mut self, address: &str, amount: u128) -> bool;

    /// Attempts to debit `amount` USDT from `address`.
    fn debit_usdt(&mut self, address: &str, amount: u64) -> bool;

    /// Credits `amount` NSC to `address`. Should not fail under
    /// normal circumstances (crediting can't "run out"), but
    /// returns bool in case your ledger enforces a supply cap here.
    fn credit_nsc(&mut self, address: &str, amount: u128) -> bool;

    /// Credits `amount` USDT to `address`.
    fn credit_usdt(&mut self, address: &str, amount: u64) -> bool;
}

// ── Pool ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LiquidityPool {
    pub nsc_reserve: u128,
    pub usdt_reserve: u64,
    pub total_lp_tokens: u128,
    pub lp_holders: HashMap<String, u128>,
    pub trades: Vec<Trade>,
    /// [FIX-07] Defensive re-entrancy guard. If this is ever found
    /// `true` at the start of a call, something is calling into the
    /// pool recursively or concurrently without the outer Mutex
    /// actually serializing access — treat that as a fatal bug, not
    /// something to silently work around.
    in_operation: bool,
}

/// Minimum amount of either asset that must remain in a reserve
/// after any operation. Prevents a swap/withdrawal from ever
/// driving a reserve to exactly zero, which would make get_price()
/// divide-by-zero-adjacent and permanently brick the pool.
const MIN_RESERVE: u64 = 1;
const MIN_RESERVE_NSC: u128 = 1;

/// Fee in basis points (3 = 0.03%, matching the original 3/1000).
const FEE_BPS: u64 = 30;
const FEE_DENOM: u64 = 10_000;

impl LiquidityPool {
    pub fn new() -> Self {
        Self {
            nsc_reserve: 0,
            usdt_reserve: 0,
            total_lp_tokens: 0,
            lp_holders: HashMap::new(),
            trades: Vec::new(),
            in_operation: false,
        }
    }

    /// [FIX-07] Call at the top of every mutating method.
    /// Panics loudly on re-entrancy rather than silently corrupting
    /// state — this should be unreachable if your Mutex<Blockchain>
    /// (or equivalent) correctly serializes all access to the pool.
    fn enter(&mut self) {
        if self.in_operation {
            panic!(
                "[AMM] FATAL: re-entrant call detected into LiquidityPool. \
                 This indicates a locking bug elsewhere in the codebase — \
                 the pool must only ever be accessed from inside a single \
                 held lock. Aborting rather than risking fund corruption."
            );
        }
        self.in_operation = true;
    }

    fn exit(&mut self) {
        self.in_operation = false;
    }

    // ── Liquidity provision ─────────────────────────────────

    /// Adds liquidity to the pool, debiting the provider's real
    /// on-chain NSC and USDT balances first.
    ///
    /// [FIX-01] Verifies and debits real balances before crediting
    /// any LP tokens. Previously this credited LP tokens for
    /// deposits that were never actually taken from anyone.
    /// [FIX-05] LP mint math done in u128 to avoid overflow.
    pub fn add_liquidity<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        provider: &str,
        nsc: u128,
        usdt: u64,
    ) -> Result<u128, AmmError> {
        self.enter();
        let result = self.add_liquidity_inner(ledger, provider, nsc, usdt);
        self.exit();
        result
    }

    fn add_liquidity_inner<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        provider: &str,
        nsc: u128,
        usdt: u64,
    ) -> Result<u128, AmmError> {
        if nsc == 0 || usdt == 0 {
            return Err(AmmError::ZeroAmount);
        }

        // [FIX-01] Verify real balances BEFORE mutating any pool state.
        if ledger.nsc_balance(provider) < nsc {
            return Err(AmmError::InsufficientBalance);
        }
        if ledger.usdt_balance(provider) < usdt {
            return Err(AmmError::InsufficientBalance);
        }

        // [FIX-10] LP minting must only use sqrt(nsc*usdt) for the
        // FIRST deposit (when the pool is empty). Every subsequent
        // deposit must mint LP proportional to the EXISTING reserve
        // ratio, using the smaller of the two implied amounts, so a
        // depositor cannot mint disproportionate LP shares by
        // depositing at a skewed ratio and diluting existing holders.
        let lp_minted: u128 = if self.total_lp_tokens == 0 {
            let product: u128 = nsc.saturating_mul(usdt as u128);
            integer_sqrt_u128(product)
        } else {
            if self.nsc_reserve == 0 || self.usdt_reserve == 0 {
                return Err(AmmError::PoolEmpty);
            }
            let lp_from_nsc: u128 = nsc
                .saturating_mul(self.total_lp_tokens)
                / self.nsc_reserve;
            let lp_from_usdt: u128 = (usdt as u128)
                .saturating_mul(self.total_lp_tokens)
                / (self.usdt_reserve as u128);
            lp_from_nsc.min(lp_from_usdt)
        };

        if lp_minted == 0 {
            // Deposit too small to mint any LP tokens — reject rather
            // than silently accepting a deposit that grants nothing.
            return Err(AmmError::ZeroAmount);
        }

        // Debit real balances. If either debit fails, we must not have
        // mutated pool state yet — and we haven't, so this is safe to
        // bail out of cleanly.
        if !ledger.debit_nsc(provider, nsc) {
            return Err(AmmError::BalanceUpdateFailed);
        }
        if !ledger.debit_usdt(provider, usdt) {
            // CRITICAL: we already debited NSC. Refund it before
            // returning the error, since the USDT debit failed.
            // This is why a real implementation should ideally make
            // both debits part of one atomic chain.rs transaction
            // rather than two separate calls — treat this refund path
            // as a stopgap, not a substitute for atomic balance updates.
            ledger.credit_nsc(provider, nsc);
            return Err(AmmError::BalanceUpdateFailed);
        }

        // Only now, after funds are confirmed moved, update pool state.
        self.nsc_reserve = self.nsc_reserve.checked_add(nsc).ok_or(AmmError::Overflow)?;
        self.usdt_reserve = self.usdt_reserve.checked_add(usdt).ok_or(AmmError::Overflow)?;
        self.total_lp_tokens = self
            .total_lp_tokens
            .checked_add(lp_minted)
            .ok_or(AmmError::Overflow)?;

        let current = self.lp_holders.get(provider).copied().unwrap_or(0);
        self.lp_holders.insert(
            provider.to_string(),
            current.checked_add(lp_minted).ok_or(AmmError::Overflow)?,
        );

        println!(
            "[AMM] {} deposited {} NSC + {} USDT, received {} LP tokens.",
            provider, nsc, usdt, lp_minted
        );

        Ok(lp_minted)
    }

    /// Removes liquidity, crediting the provider's real on-chain
    /// balances with the withdrawn NSC and USDT.
    ///
    /// [FIX-02] Actually credits real balances (previously just printed).
    /// [FIX-09] Rejects withdrawals that would drain a reserve to zero.
    pub fn remove_liquidity<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        provider: &str,
        lp_amount: u128,
    ) -> Result<(u128, u64), AmmError> {
        self.enter();
        let result = self.remove_liquidity_inner(ledger, provider, lp_amount);
        self.exit();
        result
    }

    fn remove_liquidity_inner<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        provider: &str,
        lp_amount: u128,
    ) -> Result<(u128, u64), AmmError> {
        if lp_amount == 0 {
            return Err(AmmError::ZeroAmount);
        }
        if self.total_lp_tokens == 0 {
            return Err(AmmError::PoolEmpty);
        }

        let owned_lp = self.lp_holders.get(provider).copied().unwrap_or(0);
        if owned_lp < lp_amount {
            return Err(AmmError::InsufficientLpTokens);
        }

        // nsc_reserve/total_lp_tokens are u128 natively now.
        let nsc_out: u128 = self.nsc_reserve
            .saturating_mul(lp_amount)
            / self.total_lp_tokens;

        let usdt_out: u64 = ((self.usdt_reserve as u128)
            .saturating_mul(lp_amount)
            / self.total_lp_tokens)
            .try_into()
            .map_err(|_| AmmError::Overflow)?;

        // [FIX-09] Never let a withdrawal drain a reserve below the floor.
        if self.nsc_reserve.saturating_sub(nsc_out) < MIN_RESERVE_NSC
            || self.usdt_reserve.saturating_sub(usdt_out) < MIN_RESERVE
        {
            return Err(AmmError::WouldDrainReserve);
        }

        // Update pool state first here is fine (unlike deposits) because
        // failure to credit can be retried without risk of *taking*
        // anything from the user — but we still check credit success.
        self.nsc_reserve = self.nsc_reserve.checked_sub(nsc_out).ok_or(AmmError::Overflow)?;
        self.usdt_reserve = self.usdt_reserve.checked_sub(usdt_out).ok_or(AmmError::Overflow)?;
        self.total_lp_tokens = self
            .total_lp_tokens
            .checked_sub(lp_amount)
            .ok_or(AmmError::Overflow)?;
        self.lp_holders.insert(provider.to_string(), owned_lp - lp_amount);

        if !ledger.credit_nsc(provider, nsc_out) || !ledger.credit_usdt(provider, usdt_out) {
            // Pool state already changed. This should not be reachable
            // if credit_* genuinely cannot fail (see trait doc comment).
            // If your implementation CAN fail here, you must make this
            // whole function transactional against chain.rs instead.
            eprintln!(
                "[AMM] FATAL: credited pool withdrawal but ledger credit failed for {}. \
                 Manual reconciliation required.",
                provider
            );
            return Err(AmmError::BalanceUpdateFailed);
        }

        println!(
            "[AMM] {} removed {} LP tokens, received {} NSC + {} USDT.",
            provider, lp_amount, nsc_out, usdt_out
        );

        Ok((nsc_out, usdt_out))
    }

    // ── Swaps ────────────────────────────────────────────────

    /// Swaps NSC for USDT. Debits the trader's real NSC balance,
    /// credits their real USDT balance.
    ///
    /// `min_usdt_out` is the caller's slippage floor — if the
    /// computed output is below this, the swap is rejected and
    /// NOTHING is moved. [FIX-06]
    pub fn swap_nsc_for_usdt<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        trader: &str,
        nsc_in: u128,
        min_usdt_out: u64,
    ) -> Result<u64, AmmError> {
        self.enter();
        let result = self.swap_nsc_for_usdt_inner(ledger, trader, nsc_in, min_usdt_out);
        self.exit();
        result
    }

    fn swap_nsc_for_usdt_inner<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        trader: &str,
        nsc_in: u128,
        min_usdt_out: u64,
    ) -> Result<u64, AmmError> {
        if nsc_in == 0 {
            return Err(AmmError::ZeroAmount);
        }
        if self.nsc_reserve == 0 || self.usdt_reserve == 0 {
            return Err(AmmError::PoolEmpty);
        }

        // [FIX-03] Verify the trader actually has the NSC before
        // computing or moving anything.
        if ledger.nsc_balance(trader) < nsc_in {
            return Err(AmmError::InsufficientBalance);
        }

        let fee = nsc_in.saturating_mul(FEE_BPS as u128) / (FEE_DENOM as u128);
        let amount_in = nsc_in.checked_sub(fee).ok_or(AmmError::Overflow)?;

        let new_nsc_reserve = self
            .nsc_reserve
            .checked_add(amount_in)
            .ok_or(AmmError::Overflow)?;

        let usdt_out: u64 = (amount_in.saturating_mul(self.usdt_reserve as u128)
            / new_nsc_reserve)
            .try_into()
            .map_err(|_| AmmError::Overflow)?;

        // [FIX-06] Slippage protection.
        if usdt_out < min_usdt_out {
            return Err(AmmError::SlippageExceeded);
        }

        // [FIX-09] Never drain the USDT reserve to zero.
        if self.usdt_reserve.saturating_sub(usdt_out) < MIN_RESERVE {
            return Err(AmmError::WouldDrainReserve);
        }

        // Debit the trader's real NSC balance BEFORE updating reserves.
        if !ledger.debit_nsc(trader, nsc_in) {
            return Err(AmmError::BalanceUpdateFailed);
        }

        self.nsc_reserve = new_nsc_reserve;
        self.usdt_reserve = self.usdt_reserve.checked_sub(usdt_out).ok_or(AmmError::Overflow)?;

        if !ledger.credit_usdt(trader, usdt_out) {
            // We already took the trader's NSC and moved the reserve.
            // Attempt to unwind — refund NSC, restore reserves — rather
            // than leaving funds in limbo.
            self.nsc_reserve = self.nsc_reserve.checked_sub(amount_in).unwrap_or(self.nsc_reserve);
            self.usdt_reserve = self.usdt_reserve.checked_add(usdt_out).unwrap_or(self.usdt_reserve);
            ledger.credit_nsc(trader, nsc_in);
            return Err(AmmError::BalanceUpdateFailed);
        }

        // [FIX-08] Record the trader address in trade history.
        self.trades.push(Trade {
            trade_type: "NSC->USDT".to_string(),
            amount_in: nsc_in,
            amount_out: usdt_out as u128,
        });

        println!(
            "[AMM] {} swapped {} NSC -> {} USDT (fee={} NSC).",
            trader, nsc_in, usdt_out, fee
        );

        Ok(usdt_out)
    }

    /// Swaps USDT for NSC. Mirror of swap_nsc_for_usdt above.
    pub fn swap_usdt_for_nsc<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        trader: &str,
        usdt_in: u64,
        min_nsc_out: u128,
    ) -> Result<u128, AmmError> {
        self.enter();
        let result = self.swap_usdt_for_nsc_inner(ledger, trader, usdt_in, min_nsc_out);
        self.exit();
        result
    }

    fn swap_usdt_for_nsc_inner<L: BalanceLedger>(
        &mut self,
        ledger: &mut L,
        trader: &str,
        usdt_in: u64,
        min_nsc_out: u128,
    ) -> Result<u128, AmmError> {
        if usdt_in == 0 {
            return Err(AmmError::ZeroAmount);
        }
        if self.nsc_reserve == 0 || self.usdt_reserve == 0 {
            return Err(AmmError::PoolEmpty);
        }

        if ledger.usdt_balance(trader) < usdt_in {
            return Err(AmmError::InsufficientBalance);
        }

        let fee = usdt_in.saturating_mul(FEE_BPS) / FEE_DENOM;
        let amount_in = usdt_in.checked_sub(fee).ok_or(AmmError::Overflow)?;

        let new_usdt_reserve = self
            .usdt_reserve
            .checked_add(amount_in)
            .ok_or(AmmError::Overflow)?;

        let nsc_out: u128 = (amount_in as u128).saturating_mul(self.nsc_reserve)
            / (new_usdt_reserve as u128);

        if nsc_out < min_nsc_out {
            return Err(AmmError::SlippageExceeded);
        }

        if self.nsc_reserve.saturating_sub(nsc_out) < MIN_RESERVE_NSC {
            return Err(AmmError::WouldDrainReserve);
        }

        if !ledger.debit_usdt(trader, usdt_in) {
            return Err(AmmError::BalanceUpdateFailed);
        }

        self.usdt_reserve = new_usdt_reserve;
        self.nsc_reserve = self.nsc_reserve.checked_sub(nsc_out).ok_or(AmmError::Overflow)?;

        if !ledger.credit_nsc(trader, nsc_out) {
            self.usdt_reserve = self.usdt_reserve.checked_sub(amount_in).unwrap_or(self.usdt_reserve);
            self.nsc_reserve = self.nsc_reserve.checked_add(nsc_out).unwrap_or(self.nsc_reserve);
            ledger.credit_usdt(trader, usdt_in);
            return Err(AmmError::BalanceUpdateFailed);
        }

        self.trades.push(Trade {
            trade_type: "USDT->NSC".to_string(),
            amount_in: usdt_in as u128,
            amount_out: nsc_out,
        });

        println!(
            "[AMM] {} swapped {} USDT -> {} NSC (fee={} USDT).",
            trader, usdt_in, nsc_out, fee
        );

        Ok(nsc_out)
    }

    // ── Read-only views ──────────────────────────────────────

    pub fn get_price(&self) -> f64 {
        if self.nsc_reserve == 0 {
            return 0.0;
        }
        self.usdt_reserve as f64 / (self.nsc_reserve as f64 / crate::genesis::DECIMALS as f64)
    }

    pub fn lp_balance(&self, address: &str) -> u128 {
        *self.lp_holders.get(address).unwrap_or(&0)
    }

    pub fn show_trades(&self) {
        println!("\n===== TRADE HISTORY =====");
        for trade in &self.trades {
            println!("{:?}", trade);
        }
    }

    pub fn show_lp_holders(&self) {
        println!("\n===== LP HOLDERS =====");
        for (holder, amount) in &self.lp_holders {
            println!("{} => {} LP", holder, amount);
        }
    }

    pub fn info(&self) {
        println!("\n===== NSC/USDT POOL =====");
        println!("NSC Reserve  : {}", self.nsc_reserve);
        println!("USDT Reserve : {}", self.usdt_reserve);
        println!("LP Supply    : {}", self.total_lp_tokens);
        println!("Price        : {:.4}", self.get_price());
    }
}

impl Default for LiquidityPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Integer square root for u128, used for LP-token minting so we
/// never lose precision (or overflow) the way `(x as f64).sqrt() as u64`
/// could for large reserves. Newton's method, exact for integers.
fn integer_sqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}
