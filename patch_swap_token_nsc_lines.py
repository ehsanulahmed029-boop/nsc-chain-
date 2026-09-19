import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    lines = f.readlines()

expected = {
    2334: "save_tokens(&tokens)",
    2337: "save_token_pool(from, tpool.0, tpool.1)",
    2341: "save_pool(pool.0, pool.1)",
    2344: "let mut chain = blockchain.lock()",
    2347: "save_evm_state(&chain.balances, &chain.nonces)",
}
for lineno, needle in expected.items():
    actual = lines[lineno - 1]
    if needle not in actual:
        print(f"ERROR: line {lineno} does not contain expected text.")
        print(f"  expected substring: {needle}")
        print(f"  actual line: {actual!r}")
        sys.exit(1)
print("All five anchor lines verified.")

idx_2334 = 2334 - 1
idx_2337 = 2337 - 1
idx_2341 = 2341 - 1
idx_2343 = 2343 - 1  # opening `{` of the chain-lock block
idx_2347 = 2347 - 1
idx_2350 = 2350 - 1  # log_trade line, insert batch block before this

indent_top = "                                          "   # matches lines 2334/2337/2341 indent
indent_inner = "                                              "  # matches line 2347 indent

# 1) Replace the `{` opening the chain-lock block with `let evm_state_writes = {`
lines[idx_2343] = lines[idx_2343].replace("{", "let evm_state_writes = {", 1)

# 2) Replace line 2347 (save_evm_state call) with the prepare-call as the
#    block's trailing expression (no semicolon needed issue -- we still
#    add one and then use the returned Vec via the let below; simplest is
#    to keep it as a statement then reference chain again -- but chain is
#    still in scope here since we're still inside the block).
lines[idx_2347] = indent_inner + "crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces)\n"

# 3) Blank out the three earlier save_* lines (highest index first).
lines[idx_2341] = ""
lines[idx_2337] = ""
lines[idx_2334] = ""

# 4) Insert the batch block before log_trade (idx_2350, still valid since
#    we only blanked/replaced in place, never removed list entries).
batch_block = (
    indent_top + "// [ATOMICITY FIX] tokens.json, the token's pool file,\n" +
    indent_top + "// pool.json, and evm_state.json committed together\n" +
    indent_top + "// instead of four sequential independent saves.\n" +
    indent_top + "let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();\n" +
    indent_top + "if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "if let Some(w) = crate::storage::prepare_token_pool_write(from, tpool.0, tpool.1) {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "batch_writes.extend(evm_state_writes);\n" +
    indent_top + "if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {\n" +
    indent_top + "    eprintln!(\"[API] Failed to commit swap batch (token->NSC): {}\", e);\n" +
    indent_top + "}\n"
)
lines.insert(idx_2350, batch_block)

with open(path, "w") as f:
    f.writelines(lines)

print("Patched: /swap TOKEN->USDT->NSC router branch now uses atomic_write_batch (line-based patch)")
