import shutil, datetime

ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
path = "/root/nsc-chain/src/staking.rs"
shutil.copy(path, f"/root/backups/staking.rs.{ts}.bak")
print(f"Backup saved: /root/backups/staking.rs.{ts}.bak")

with open(path, "r") as f:
    content = f.read()

edits = [
    (
        '''    pub fn total_staked(&self) -> u128 {
        self.validators.values().sum()
    }''',
        '''    pub fn total_staked(&self) -> u128 {
        // [FIX-08] Checked sum instead of `.sum()` — a raw sum silently
        // wraps on overflow in release builds. Saturating here is the
        // right failure mode for a read-only display total: it cannot
        // corrupt any real balance (this function never mutates state),
        // and clamping to u128::MAX is a more honest signal that
        // something is very wrong than silently wrapping to a small
        // number would be.
        self.validators
            .values()
            .fold(0u128, |acc, &stake| acc.saturating_add(stake))
    }'''
    ),
    (
        '''    pub fn slash_stake(&mut self, address: &str, amount: u128) -> Result<u128, StakingError> {
        let current = self.validators.get(address).copied().unwrap_or(0);
        let slashed = amount.min(current);

        if slashed == 0 {
            return Err(StakingError::InsufficientStake);
        }''',
        '''    pub fn slash_stake(&mut self, address: &str, amount: u128) -> Result<u128, StakingError> {
        // [FIX-09] amount == 0 is a caller error, not a statement about
        // the validator's stake — report it as ZeroAmount so callers
        // can distinguish "you asked to slash nothing" from "this
        // validator has nothing left to slash".
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }

        let current = self.validators.get(address).copied().unwrap_or(0);
        let slashed = amount.min(current);

        if slashed == 0 {
            return Err(StakingError::InsufficientStake);
        }'''
    ),
]

for old, new in edits:
    count = content.count(old)
    if count != 1:
        print(f"ERROR: expected 1 match, found {count} for anchor starting: {old[:60]!r}")
        exit(1)
    content = content.replace(old, new)

with open(path, "w") as f:
    f.write(content)

print("Patched staking.rs successfully.")
