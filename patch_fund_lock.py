import re
import shutil
import datetime

ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
path = "/root/nsc-chain/src/api.rs"
backup = f"/root/backups/api.rs.{ts}.bak"
shutil.copy(path, backup)
print(f"Backup saved: {backup}")

with open(path, "r") as f:
    content = f.read()

# ── Step 1: add FUND_LOCK static after the existing constants block ──
anchor = 'const CORS_ORIGIN_ENV: &str = "NSC_API_CORS_ORIGIN";'
if content.count(anchor) != 1:
    print(f"ERROR: expected 1 match for CORS_ORIGIN_ENV anchor, found {content.count(anchor)}")
    exit(1)

fund_lock_decl = anchor + '''

/// [FUND-LOCK] Global serialization lock for all fund-mutating,
/// multi-step, file-backed read-modify-write endpoints (pool
/// reserves, custom-token balances, USDT balances, LP shares,
/// pending withdrawals). These operate on plain JSON files via
/// storage.rs, entirely outside the Mutex<Blockchain> that protects
/// native NSC balances -- so without this lock, two concurrent
/// requests touching the same pool/token/balance file can both read
/// stale state, both pass their checks, and both write, silently
/// double-spending or corrupting reserves.
///
/// This does NOT provide crash-mid-sequence atomicity (that needs a
/// real write-ahead journal, tracked separately) -- it only
/// serializes concurrent requests so at most one is ever mutating
/// this class of state at a time. Always acquire FUND_LOCK before
/// blockchain.lock() where both are needed in the same handler, to
/// keep lock ordering consistent and avoid deadlock.
static FUND_LOCK: std::sync::LazyLock<Mutex<()>> = std::sync::LazyLock::new(|| Mutex::new(()));'''

content = content.replace(anchor, fund_lock_decl)
print("Inserted FUND_LOCK static.")

# ── Step 2: insert the guard acquisition into each target route arm ──
routes = [
    "/usdt_withdraw",
    "/wnsc_release",
    "/usdt_credit",
    "/usdt_transfer",
    "/liquidity/add",
    "/token/liquidity/remove",
    "/token/liquidity/add",
    "/token/create",
    "/swap",
]

guard_line_template = '{indent}let _fund_guard = FUND_LOCK.lock().expect("fund lock");\n'

for route in routes:
    route_pat = f'"{route}" => {{'
    occurrences = [m.start() for m in re.finditer(re.escape(route_pat), content)]
    if len(occurrences) != 1:
        print(f"ERROR: expected exactly 1 occurrence of route arm {route_pat!r}, found {len(occurrences)}")
        exit(1)
    route_start = occurrences[0]

    # Find the next "Ok(body) => {" after this route arm starts, within a reasonable window.
    window_end = route_start + 3000
    search_region = content[route_start:window_end]
    m = re.search(r'Ok\(body\)\s*=>\s*\{', search_region)
    if not m:
        print(f"ERROR: could not find 'Ok(body) => {{' after route {route}")
        exit(1)

    insert_pos = route_start + m.end()  # position right after the opening brace

    # Determine indentation to use: find the whitespace at the start of the
    # line containing "Ok(body)" and add 4 spaces for the inserted line.
    line_start = content.rfind("\n", 0, route_start + m.start()) + 1
    existing_indent = re.match(r'[ \t]*', content[line_start:route_start + m.start()]).group(0)
    indent = existing_indent + "    "

    guard_line = "\n" + guard_line_template.format(indent=indent).rstrip("\n")

    content = content[:insert_pos] + guard_line + content[insert_pos:]
    print(f"Inserted FUND_LOCK guard into: {route}")

with open(path, "w") as f:
    f.write(content)

print("All patches applied successfully to api.rs")
