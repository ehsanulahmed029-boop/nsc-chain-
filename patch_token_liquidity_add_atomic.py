import shutil, datetime, sys

path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    lines = f.readlines()

expected = {
    2721: "save_usdt_balance(&wallet, prev_usdt - usdt_amount)",
    2722: "save_tokens(&tokens)",
    2731: "let mut lp_minted: u128 = 0;",
    2752: "save_token_lp(&symbol, total_lp, &holders)",
    2758: "save_token_pool(&symbol, pool.0, pool.1)",
    2759: "json_response(200, json!({",
}
for lineno, needle in expected.items():
    actual = lines[lineno - 1]
    if needle not in actual:
        print(f"ERROR: line {lineno} does not contain expected text.")
        print(f"  expected substring: {needle}")
        print(f"  actual line: {actual!r}")
        sys.exit(1)
print("All anchor lines verified.")

idx_2721 = 2721 - 1
idx_2722 = 2722 - 1
idx_2731 = 2731 - 1  # "let mut lp_minted: u128 = 0;" -- insert new Option decl right after this
idx_2752 = 2752 - 1
idx_2758 = 2758 - 1
idx_2759 = 2759 - 1  # json_response line, insert batch block before this

indent_31 = "                                  "  # matches lp_minted line indent
indent_52 = "                                          "  # matches save_token_lp line indent
indent_top = "                                "  # matches save_token_pool / json_response indent

# 1) Blank the two early sequential saves (usdt_balance, tokens).
lines[idx_2721] = ""
lines[idx_2722] = ""

# 2) Insert an Option<> declaration right after the lp_minted declaration line.
lines[idx_2731] = lines[idx_2731] + indent_31 + "let mut token_lp_writes: Option<(std::path::PathBuf, Vec<u8>)> = None;\n"

# 3) Replace the save_token_lp call with capturing into token_lp_writes.
lines[idx_2752] = indent_52 + "token_lp_writes = crate::storage::prepare_token_lp_write(&symbol, total_lp, &holders);\n"

# 4) Blank the save_token_pool call.
lines[idx_2758] = ""

# 5) Insert the batch block before json_response.
batch_block = (
    indent_top + "// [ATOMICITY FIX] usdt_balances.json, tokens.json,\n" +
    indent_top + "// the optional token_lp.json update, and the token's\n" +
    indent_top + "// pool file committed together instead of up to four\n" +
    indent_top + "// sequential independent saves.\n" +
    indent_top + "let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();\n" +
    indent_top + "if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt - usdt_amount) {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "if let Some(w) = token_lp_writes {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "if let Some(w) = crate::storage::prepare_token_pool_write(&symbol, pool.0, pool.1) {\n" +
    indent_top + "    batch_writes.push(w);\n" +
    indent_top + "}\n" +
    indent_top + "if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {\n" +
    indent_top + "    eprintln!(\"[API] Failed to commit token/liquidity/add batch: {}\", e);\n" +
    indent_top + "}\n"
)
lines.insert(idx_2759, batch_block)

with open(path, "w") as f:
    f.writelines(lines)

print("Patched: /token/liquidity/add now uses atomic_write_batch (line-based patch)")
