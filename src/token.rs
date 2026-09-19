use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Token {
    pub name: String,
    pub symbol: String,
    pub total_supply: u128,
    pub balances: HashMap<String, u128>,
}

impl Token {
    pub fn new(
        name: String,
        symbol: String,
        supply: u128,
        owner: String,
    ) -> Self {

        let mut balances =
            HashMap::new();

        balances.insert(
            owner,
            supply,
        );

        Self {
            name,
            symbol,
            total_supply: supply,
            balances,
        }
    }

    pub fn balance_of(
        &self,
        address: &str,
    ) -> u128 {
        *self.balances
            .get(address)
            .unwrap_or(&0)
    }

    pub fn transfer(
        &mut self,
        from: String,
        to: String,
        amount: u128,
    ) -> bool {

        let from_balance =
            self.balance_of(&from);

        if from_balance < amount {
            return false;
        }

        self.balances.insert(
            from.clone(),
            from_balance - amount,
        );

        // [P4-hardening, 2026-08-16] checked_add instead of raw + —
        // practically unreachable at real token-supply magnitudes, but
        // fails safely instead of silently wrapping if it ever were.
        let to_balance = self.balance_of(&to);
        let new_to_balance = match to_balance.checked_add(amount) {
            Some(v) => v,
            None => {
                // Roll back the sender debit above so this function
                // stays atomic (all-or-nothing) even on this
                // practically-unreachable path.
                self.balances.insert(from, from_balance);
                return false;
            }
        };
        self.balances.insert(to, new_to_balance);

        true
    }
}
