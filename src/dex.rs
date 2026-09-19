// ============================================================
// NUSACOIN (NSC) — dex.rs — Hardened Replacement (DRAFT)
// ============================================================
// THIS FILE IS A DRAFT. IT HAS NOT BEEN COMPILED OR TESTED.
//
// WHAT WAS WRONG WITH THE PREVIOUS VERSION:
// - create_order() let anyone list any amount of any token for
//   sale with no check that the seller actually owned it.
// - There was no fill/buy function at all — orders could be
//   created but never executed. As a standalone file it wasn't
//   a working exchange, just a list.
// - No cancellation — once created, an order (even a bogus one)
//   sat there forever.
//
// WHAT THIS VERSION DOES:
// [FIX-01] create_order() now ESCROWS the seller's tokens at
//          listing time — it debits their real balance into the
//          order itself. The seller cannot sell what they don't
//          have, because the attempt to escrow fails immediately
//          if their balance is insufficient.
// [FIX-02] fill_order() actually exists. It debits the buyer's
//          NSC payment, credits the seller, and releases the
//          escrowed token to the buyer — atomically, with no
//          path that moves one side without the other.
// [FIX-03] cancel_order() returns escrowed funds to the seller
//          and removes the order. Only the original seller (or,
//          if you wire it that way, an authorized admin/governance
//          action) can cancel.
// [FIX-04] Partial fills are explicitly NOT supported in this
//          version — every fill is all-or-nothing for the order's
//          full amount. Partial fills need their own remaining-
//          amount bookkeeping and are a meaningfully bigger
//          feature; I did not bolt that on without you deciding
//          you actually want it, since half-implemented partial
//          fills are a common source of accounting bugs.
// [FIX-05] Orders have unique ids, a status (Open/Filled/
//          Cancelled), and a timestamp, so order state is
//          unambiguous and history is preserved instead of
//          orders disappearing on fill/cancel.
// [FIX-06] All arithmetic uses checked operations.
// [FIX-07] An order cannot be filled by its own creator (no
//          wash-trading against yourself within a single order).
// ============================================================

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Ledger interface ─────────────────────────────────────────
//
// Same pattern as amm.rs's BalanceLedger — the DEX never touches
// raw balances directly, it goes through this interface so your
// real chain.rs validation/locking is always what actually moves
// funds. Implement this for Blockchain once (it can be the exact
// same impl block used for amm.rs's BalanceLedger if you want one
// shared trait — I've kept it separate here only because dex.rs
// also needs to escrow arbitrary "token" balances, not just
// NSC/USDT, which your real implementation may or may not support
// yet; see wiring note 2 at the bottom).

pub trait DexLedger {
    /// Returns the balance of `token` held by `address`. For NSC
    /// itself, `token` will be "NSC". For other tokens, this must
    /// be wired to whatever your token_registry.rs / token.rs
    /// actually tracks.
    fn token_balance(&self, address: &str, token: &str) -> u64;

    /// Debits `amount` of `token` from `address`. Returns false
    /// (and changes nothing) if balance is insufficient.
    fn debit_token(&mut self, address: &str, token: &str, amount: u64) -> bool;

    /// Credits `amount` of `token` to `address`.
    fn credit_token(&mut self, address: &str, token: &str, amount: u64) -> bool;
}

// ── Order ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Open,
    Filled,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub seller: String,
    pub token: String,
    pub amount: u64,
    /// Total price in NSC for the full `amount` of `token`.
    pub price_nsc: u64,
    pub status: OrderStatus,
    pub created_at: u64,
    pub buyer: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DexError {
    ZeroAmount,
    InsufficientBalance,
    OrderNotFound,
    OrderNotOpen,
    NotOrderOwner,
    CannotFillOwnOrder,
    BalanceUpdateFailed,
    Overflow,
}

impl std::fmt::Display for DexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DexError::ZeroAmount => write!(f, "amount and price must be greater than zero"),
            DexError::InsufficientBalance => write!(f, "insufficient balance to escrow or pay"),
            DexError::OrderNotFound => write!(f, "order not found"),
            DexError::OrderNotOpen => write!(f, "order is not open (already filled or cancelled)"),
            DexError::NotOrderOwner => write!(f, "only the order's creator may do this"),
            DexError::CannotFillOwnOrder => write!(f, "cannot fill your own order"),
            DexError::BalanceUpdateFailed => write!(f, "failed to update balance"),
            DexError::Overflow => write!(f, "arithmetic overflow"),
        }
    }
}

#[derive(Debug)]
pub struct Dex {
    pub orders: HashMap<u64, Order>,
    next_order_id: u64,
}

impl Dex {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            next_order_id: 1,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Creates a sell order, escrowing the seller's tokens
    /// immediately.
    ///
    /// [FIX-01] The seller's `token` balance is debited right
    /// here, at creation time — not assumed to exist later at
    /// fill time. If the seller doesn't actually have `amount` of
    /// `token`, this fails and no order is created.
    pub fn create_order<L: DexLedger>(
        &mut self,
        ledger: &mut L,
        seller: &str,
        token: &str,
        amount: u64,
        price_nsc: u64,
    ) -> Result<u64, DexError> {
        if amount == 0 || price_nsc == 0 {
            return Err(DexError::ZeroAmount);
        }

        if ledger.token_balance(seller, token) < amount {
            return Err(DexError::InsufficientBalance);
        }

        if !ledger.debit_token(seller, token, amount) {
            return Err(DexError::BalanceUpdateFailed);
        }

        let id = self.next_order_id;
        self.next_order_id = self.next_order_id.checked_add(1).ok_or(DexError::Overflow)?;

        let order = Order {
            id,
            seller: seller.to_string(),
            token: token.to_string(),
            amount,
            price_nsc,
            status: OrderStatus::Open,
            created_at: Self::now(),
            buyer: None,
        };

        println!(
            "[DEX] Order {} created: {} selling {} {} for {} NSC (escrowed).",
            id, seller, amount, token, price_nsc
        );

        self.orders.insert(id, order);
        Ok(id)
    }

    /// Fills an order completely. Debits the buyer's NSC payment,
    /// credits it to the seller, and releases the escrowed token
    /// to the buyer.
    ///
    /// [FIX-02] This is the function that didn't exist before —
    /// orders could be created but never actually executed.
    /// [FIX-07] A seller cannot fill their own order.
    pub fn fill_order<L: DexLedger>(
        &mut self,
        ledger: &mut L,
        order_id: u64,
        buyer: &str,
    ) -> Result<(), DexError> {
        let order = self.orders.get(&order_id).ok_or(DexError::OrderNotFound)?.clone();

        if order.status != OrderStatus::Open {
            return Err(DexError::OrderNotOpen);
        }
        if order.seller == buyer {
            return Err(DexError::CannotFillOwnOrder);
        }
        if ledger.token_balance(buyer, "NSC") < order.price_nsc {
            return Err(DexError::InsufficientBalance);
        }

        // Debit buyer's NSC first. If this fails, nothing else
        // happens and the order remains open.
        if !ledger.debit_token(buyer, "NSC", order.price_nsc) {
            return Err(DexError::BalanceUpdateFailed);
        }

        // Pay the seller.
        if !ledger.credit_token(&order.seller, "NSC", order.price_nsc) {
            // Unwind the buyer's debit — we haven't released the
            // escrowed token yet, so this is still recoverable.
            ledger.credit_token(buyer, "NSC", order.price_nsc);
            return Err(DexError::BalanceUpdateFailed);
        }

        // Release the escrowed token to the buyer.
        if !ledger.credit_token(buyer, &order.token, order.amount) {
            // CRITICAL: at this point the buyer has paid and the
            // seller has been paid, but the token release failed.
            // Unwinding the seller's credit AND the buyer's debit
            // is the only way to leave the system consistent — do
            // both, then surface this loudly, because reaching this
            // branch at all means something is wrong with credit_token
            // itself (it's documented as "should not fail" in the
            // BalanceLedger contract).
            eprintln!(
                "[DEX] FATAL: order {} payment succeeded but token release failed. Unwinding.",
                order_id
            );
            ledger.debit_token(&order.seller, "NSC", order.price_nsc);
            ledger.credit_token(buyer, "NSC", order.price_nsc);
            return Err(DexError::BalanceUpdateFailed);
        }

        if let Some(o) = self.orders.get_mut(&order_id) {
            o.status = OrderStatus::Filled;
            o.buyer = Some(buyer.to_string());
        }

        println!(
            "[DEX] Order {} filled: {} bought {} {} from {} for {} NSC.",
            order_id, buyer, order.amount, order.token, order.seller, order.price_nsc
        );

        Ok(())
    }

    /// Cancels an open order, returning the escrowed token to the
    /// seller.
    ///
    /// [FIX-03] Previously there was no cancellation at all — an
    /// order, once created, had no way to be removed, and (since
    /// there was no escrow before) cancellation wasn't even
    /// meaningful. Now that creation escrows real funds,
    /// cancellation must exist to give sellers their funds back.
    pub fn cancel_order<L: DexLedger>(
        &mut self,
        ledger: &mut L,
        order_id: u64,
        caller: &str,
    ) -> Result<(), DexError> {
        let order = self.orders.get(&order_id).ok_or(DexError::OrderNotFound)?.clone();

        if order.status != OrderStatus::Open {
            return Err(DexError::OrderNotOpen);
        }
        if order.seller != caller {
            return Err(DexError::NotOrderOwner);
        }

        if !ledger.credit_token(&order.seller, &order.token, order.amount) {
            return Err(DexError::BalanceUpdateFailed);
        }

        if let Some(o) = self.orders.get_mut(&order_id) {
            o.status = OrderStatus::Cancelled;
        }

        println!(
            "[DEX] Order {} cancelled by {}. {} {} returned from escrow.",
            order_id, caller, order.amount, order.token
        );

        Ok(())
    }

    // ── Read-only views ──────────────────────────────────────

    pub fn list_open_orders(&self) {
        println!("\n===== DEX OPEN ORDERS =====");
        for order in self.orders.values().filter(|o| o.status == OrderStatus::Open) {
            println!(
                "#{} {} selling {} {} @ {} NSC (escrowed since {})",
                order.id, order.seller, order.amount, order.token, order.price_nsc, order.created_at
            );
        }
    }

    pub fn list_orders(&self) {
        println!("\n===== DEX ALL ORDERS =====");
        for order in self.orders.values() {
            println!(
                "#{} [{:?}] {} selling {} {} @ {} NSC (buyer={:?})",
                order.id, order.status, order.seller, order.amount, order.token, order.price_nsc, order.buyer
            );
        }
    }

    pub fn get_order(&self, order_id: u64) -> Option<&Order> {
        self.orders.get(&order_id)
    }

    pub fn orders_by_seller(&self, seller: &str) -> Vec<&Order> {
        self.orders.values().filter(|o| o.seller == seller).collect()
    }
}

impl Default for Dex {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// WIRING NOTES — read before integrating
// ============================================================
//
// 1. Implement DexLedger for your Blockchain type, e.g.:
//
//    impl DexLedger for Blockchain {
//        fn token_balance(&self, address: &str, token: &str) -> u64 {
//            if token == "NSC" {
//                self.get_balance(address)
//            } else {
//                self.token_registry.balance_of(address, token)
//            }
//        }
//        fn debit_token(&mut self, address: &str, token: &str, amount: u64) -> bool {
//            if token == "NSC" {
//                let bal = self.get_balance(address);
//                if bal < amount { return false; }
//                self.balances.insert(address.to_string(), bal - amount);
//                true
//            } else {
//                self.token_registry.debit(address, token, amount)
//            }
//        }
//        fn credit_token(&mut self, address: &str, token: &str, amount: u64) -> bool {
//            if token == "NSC" {
//                let bal = self.get_balance(address);
//                self.balances.insert(address.to_string(), bal.saturating_add(amount));
//                true
//            } else {
//                self.token_registry.credit(address, token, amount)
//            }
//        }
//    }
//
//    I do not know the real shape of your token_registry.rs /
//    token.rs (unreviewed so far) — adapt the non-NSC branch to
//    whatever actually exists there.
//
// 2. Same atomicity caveat as amm.rs and treasury.rs: fill_order
//    does three separate ledger calls (debit buyer, credit seller,
//    credit buyer) with manual unwind logic on failure. This
//    protects against a credit/debit call cleanly returning false,
//    but NOT against a hard process crash between calls. For real
//    mainnet safety this sequence should be expressed as a single
//    atomic operation at the chain.rs/storage.rs level (write the
//    full intended state change to a journal before applying any
//    part of it, replay on recovery) rather than three independent
//    calls from dex.rs. I'm flagging this consistently across every
//    file in this review because it's a single underlying gap —
//    your persistence layer doesn't yet have atomic multi-step
//    transactions — and it needs to be solved once, centrally, not
//    patched per-feature.
//
// 3. No partial fills, no order expiry, no minimum order size. All
//    are reasonable real-DEX features but each adds its own
//    accounting surface — I deliberately did not add them
//    speculatively. Decide if you need them before launch, and if
//    so we should design each one specifically rather than guessing.
//
// 4. api.rs needs real routes for create_order / fill_order /
//    cancel_order if any exist currently — I have not seen DEX
//    routes in api.rs yet. If they exist, they must call these new
//    Result-returning functions and propagate DexError properly,
//    not silently swallow failures.
//
// 5. Untested. Before deployment: test that create_order fails
//    cleanly with insufficient balance (and does NOT create a
//    partial/zombie order); that fill_order rejects self-fills;
//    that cancel_order only works for the original seller; that a
//    filled or cancelled order can never be filled or cancelled
//    again.
// ============================================================

