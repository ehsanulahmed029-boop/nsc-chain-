import re
import shutil
import datetime

path = "/root/nsc-chain/src/amm.rs"
ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
backup = f"/root/backups/amm.rs.{ts}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

old = """        // [FIX-05] Compute LP mint in u128 to avoid overflow on nsc*usdt.
        let product: u128 = nsc.saturating_mul(usdt as u128);
        let lp_minted: u128 = integer_sqrt_u128(product);

        if lp_minted == 0 {
            // Deposit too small to mint any LP tokens — reject rather
            // than silently accepting a deposit that grants nothing.
            return Err(AmmError::ZeroAmount);
        }"""

new = """        // [FIX-10] LP minting must only use sqrt(nsc*usdt) for the
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
        }"""

count = content.count(old)
if count != 1:
    print(f"ERROR: expected exactly 1 match, found {count}. Aborting, no changes written.")
    exit(1)

content = content.replace(old, new)

with open(path, "w") as f:
    f.write(content)

print("Patch applied successfully to amm.rs")
