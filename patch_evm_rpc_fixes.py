import shutil, datetime, sys

path = "/root/nsc-chain/src/evm_rpc.rs"
backup = f"/root/backups/evm_rpc.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── 1) eth_blockNumber — read live chain height instead of frozen closure value ──
old1 = '''    // eth_blockNumber — current chain height, hex-encoded.
    let height_for_closure = chain_height;
    module.register_method("eth_blockNumber", move |_params, _ctx, _ext| {
        format!("0x{:x}", height_for_closure)
    })?;'''

new1 = '''    // eth_blockNumber — current chain height, hex-encoded.
    // [FIX] Previously captured chain_height once at server startup via
    // a closure, so it never changed afterwards and broke wallet
    // confirmation tracking. Now reads the live height from the chain
    // on every call, same as eth_getBlockByNumber does.
    module.register_method("eth_blockNumber", |_params, ctx, _ext| {
        let chain = ctx.blockchain.lock().expect("chain lock");
        let height = chain.chain_height();
        format!("0x{:x}", height)
    })?;'''

if content.count(old1) != 1:
    print(f"ERROR: eth_blockNumber anchor found {content.count(old1)} times, expected 1")
    sys.exit(1)
content = content.replace(old1, new1)
print("Patched: eth_blockNumber now reads live chain height")

# ── 2) eth_call — remove double-scaling in balanceOf / totalSupply ──
old2 = '''                let addr_param = &data_trimmed[8+24..8+64];
                let addr = format!("0x{}", addr_param.to_lowercase());
                let bal = token.balance_of(&addr);
                let bal_wei = (bal as u128) * 1_000_000_000_000_000_000u128;
                format!("0x{:0>64x}", bal_wei)
            },
            "313ce567" => {
                // decimals()
                format!("0x{:0>64x}", 18)
            },
            "18160ddd" => {
                // totalSupply()
                let supply_wei = (token.total_supply as u128) * 1_000_000_000_000_000_000u128;
                format!("0x{:0>64x}", supply_wei)
            },'''

new2 = '''                let addr_param = &data_trimmed[8+24..8+64];
                let addr = format!("0x{}", addr_param.to_lowercase());
                // [FIX] token.balance_of() already returns an
                // 18-decimal-scaled u128 (Token supply/balances are
                // stored pre-scaled by DECIMALS at /token/create and
                // /token/transfer time, same convention as native NSC
                // and pool reserves). Multiplying by 1e18 again here
                // double-scaled the value, making wallets like MetaMask
                // display balances ~1e18x too large. Return as-is.
                let bal = token.balance_of(&addr);
                format!("0x{:0>64x}", bal)
            },
            "313ce567" => {
                // decimals()
                format!("0x{:0>64x}", 18)
            },
            "18160ddd" => {
                // totalSupply()
                // [FIX] Same double-scaling issue as balanceOf above --
                // token.total_supply is already 18-decimal-scaled.
                format!("0x{:0>64x}", token.total_supply)
            },'''

if content.count(old2) != 1:
    print(f"ERROR: eth_call anchor found {content.count(old2)} times, expected 1")
    sys.exit(1)
content = content.replace(old2, new2)
print("Patched: eth_call balanceOf/totalSupply no longer double-scale")

with open(path, "w") as f:
    f.write(content)

print("Done.")
