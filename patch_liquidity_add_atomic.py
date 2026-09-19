import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    lines = f.readlines()

expected = {
    1712: "{",
    1713: "let mut chain = blockchain.lock()",
    1716: "save_evm_state(&chain.balances, &chain.nonces)",
    1717: "}",
    1718: "save_usdt_balance(&wallet, sender_usdt_balance - usdt)",
    1723: "save_pool(pool.0, pool.1)",
}
for lineno, needle in expected.items():
    actual = lines[lineno - 1]
    if needle not in actual:
        print(f"ERROR: line {lineno} does not contain expected text.")
        print(f"  expected substring: {needle}")
        print(f"  actual line: {actual!r}")
        sys.exit(1)
print("All anchor lines verified.")

idx_1712 = 1712 - 1
idx_1716 = 1716 - 1
idx_1717 = 1717 - 1
idx_1718 = 1718 - 1
idx_1723 = 1723 - 1
idx_1724 = 1724 - 1  # json_response line, insert batch block before this

indent_block = "                            "  # matches line 1712/1717 indent
indent_inner = "                                "  # matches line 1716 indent
indent_top   = "                        "        # matches line 1718/1723 indent

# 1) Turn the `{` at 1712 into `let evm_state_writes = {`
lines[idx_1712] = lines[idx_1712].replace("{", "let evm_state_writes = {", 1)

# 2) Replace save_evm_state call (1716) with the prepare-call as the
#    block's trailing expression (still inside the lock scope).
lines[idx_1716] = indent_inner + "crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces)\n"

# 3) Add semicolon to the closing brace at 1717 (was `}` -> `};`)
if lines[idx_1717].strip() == "}":
    lines[idx_1717] = lines[idx_1717].rstrip("\n") + ";\n"
else:
    print(f"ERROR: line 1717 unexpected content: {lines[idx_1717]!r}")
    sys.exit(1)

# 4) Blank out save_usdt_balance (1718) and save_pool (1723) -- highest
#    index first so earlier indices in this pass remain valid.
lines[idx_1723] = ""
lines[idx_1718] = ""

# 5) Insert the batch block before json_response (idx_1724).
batch_block = (
    indent_top + "// [ATOMICITY FIX] evm_state.json, usdt_balances.json,\n" +
    indent_top + "// and pool.json committed together instead of three\n" +
    indent_top + "// sequential independent saves.\n" +
    indent_top + "let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();\n" +
    indent_top + "batch_writes.extend(evm_state_writes);\n" +
    indent_top + "if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, sender_usdt_balance - usdt) {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {\n" +
    indent_top + "    eprintln!(\"[API] Failed to commit liquidity/add batch: {}\", e);\n" +
    indent_top + "}\n"
)
lines.insert(idx_1724, batch_block)

with open(path, "w") as f:
    f.writelines(lines)

print("Patched: /liquidity/add now uses atomic_write_batch (line-based patch)")
