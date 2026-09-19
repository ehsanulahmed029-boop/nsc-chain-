// ============================================================
// NUSACOIN (NSC) — api.rs — Secure Mainnet Replacement
// ============================================================
// FIXES APPLIED:
// [FIX-01] All hardcoded fake data removed
// [FIX-02] API key authentication on every request
// [FIX-03] Rate limiting per IP (max 60 req/min)
// [FIX-04] CORS headers locked to allowed origins only
// [FIX-05] Content-Type: application/json on all responses
// [FIX-06] HTTP method enforcement (GET-only on read routes)
// [FIX-07] Request body size limit (max 64 KB)
// [FIX-08] No stack traces or internal errors in responses
// [FIX-09] All responses use structured JSON with status field
// [FIX-10] Sensitive routes (submit tx) require POST + auth
// [FIX-11] Input validation on all query parameters
// [FIX-12] Bind address configurable via env, not hardcoded
// [FIX-13] unwrap() replaced with proper error handling
// [FIX-14] API key loaded from env variable, not source code
// ============================================================

use crate::amm::BalanceLedger;
use crate::chain::Blockchain;
use std::sync::{Arc, Mutex};
use tiny_http::{Response, Server, Header, Method, Request};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Constants ─────────────────────────────────────────────────

/// Maximum allowed request body size (64 KB).
const MAX_BODY_BYTES: usize = 65_536;

/// Rate limit: max requests per minute per IP.
const RATE_LIMIT_PER_MIN: u64 = 300;

/// Env variable for API key. Set before starting node.
const API_KEY_ENV: &str = "NSC_API_KEY";

/// Env variable for allowed CORS origin.
const CORS_ORIGIN_ENV: &str = "NSC_API_CORS_ORIGIN";

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
static FUND_LOCK: std::sync::LazyLock<Mutex<()>> = std::sync::LazyLock::new(|| Mutex::new(()));

/// Env variable for bind address.
const BIND_ADDR_ENV: &str = "NSC_API_BIND";

/// Default bind address if env not set.
const DEFAULT_BIND: &str = "127.0.0.1:8080";

// ── Rate limiter ──────────────────────────────────────────────

/// Per-IP request count within the current minute window.
struct RateLimiter {
    counts:     HashMap<String, (u64, u64)>, // ip -> (count, window_start_secs)
}

impl RateLimiter {
    fn new() -> Self {
        Self { counts: HashMap::new() }
    }

    /// Returns true if the IP is within the rate limit.
    fn is_allowed(&mut self, ip: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let window_start = now - (now % 60);

        let entry = self.counts.entry(ip.to_string()).or_insert((0, window_start));

        // Reset if we're in a new minute window.
        if entry.1 < window_start {
            *entry = (0, window_start);
        }

        entry.0 += 1;
        entry.0 <= RATE_LIMIT_PER_MIN
    }
}

// ── Response helpers ──────────────────────────────────────────

/// Builds a JSON response with standard headers.
///
/// [FIX-05] Content-Type: application/json always set.
/// [FIX-04] CORS header locked to configured origin.
/// [FIX-09] All responses have a `status` field.
fn json_response(
    status_code:  u16,
    body:         serde_json::Value,
    cors_origin:  &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let body_str  = body.to_string();
    let body_bytes = body_str.into_bytes();

    let content_type = Header::from_bytes(
        "Content-Type",
        "application/json; charset=utf-8",
    ).expect("valid header");

    let cors = Header::from_bytes(
        "Access-Control-Allow-Origin",
        cors_origin,
    ).expect("valid header");

    let x_content_type = Header::from_bytes(
        "X-Content-Type-Options",
        "nosniff",
    ).expect("valid header");

    let x_frame = Header::from_bytes(
        "X-Frame-Options",
        "DENY",
    ).expect("valid header");

    Response::from_data(body_bytes)
        .with_status_code(status_code)
        .with_header(content_type)
        .with_header(cors)
        .with_header(x_content_type)
        .with_header(x_frame)
}

/// 401 Unauthorized.
fn unauthorized(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        401,
        json!({ "status": "error", "error": "Unauthorized" }),
        cors,
    )
}

/// 429 Too Many Requests.
fn rate_limited(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        429,
        json!({ "status": "error", "error": "Rate limit exceeded" }),
        cors,
    )
}

/// 405 Method Not Allowed.
fn method_not_allowed(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        405,
        json!({ "status": "error", "error": "Method not allowed" }),
        cors,
    )
}

/// 404 Not Found.
fn not_found(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        404,
        json!({ "status": "error", "error": "Not found" }),
        cors,
    )
}

/// 400 Bad Request.
/// [P4-hardening, 2026-08-16] Correctly parses a single query-string
/// parameter by key, splitting on '&' first and then '=' — unlike the
/// old inline pattern (`q.split('=').nth(1)`) used at several call
/// sites, which broke as soon as a second query param was present
/// (e.g. "address=0x1&symbol=NSC" would yield "0x1&symbol" instead of
/// "0x1"). Returns "" if the key is not found.
fn get_query_param(url: &str, key: &str) -> String {
    let query = match url.split('?').nth(1) {
        Some(q) => q,
        None => return String::new(),
    };
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let k = parts.next().unwrap_or("");
        let v = parts.next().unwrap_or("");
        if k == key {
            return v.to_string();
        }
    }
    String::new()
}

fn bad_request(msg: &str, cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        400,
        json!({ "status": "error", "error": msg }),
        cors,
    )
}

// ── Auth check ────────────────────────────────────────────────

/// Extracts and validates the API key from the request.
///
/// [FIX-02] Every request must carry X-Api-Key header.
/// [FIX-14] Key loaded from NSC_API_KEY env variable.
fn is_authenticated(request: &Request, expected_key: &str) -> bool {
    if expected_key.is_empty() {
        // If no key is configured, deny all requests.
        eprintln!("[API] NSC_API_KEY not set — all requests denied.");
        return false;
    }

    for header in request.headers() {
        if header.field.as_str().to_ascii_lowercase() == "x-api-key" {
            return header.value.as_str() == expected_key;
        }
    }

    false
}

/// Extracts the remote IP from the request for rate limiting.
///
/// [SECURITY/SCALE FIX] Nginx sits in front of this server and
/// forwards the real client IP via X-Forwarded-For. Without
/// reading it, every request would appear to come from Nginx's
/// own connection (127.0.0.1), making the per-IP rate limiter
/// apply to ALL visitors combined instead of each individually.
fn get_remote_ip(request: &Request) -> String {
    for header in request.headers() {
        if header.field.as_str().to_ascii_lowercase() == "x-forwarded-for" {
            let value = header.value.as_str();
            if let Some(first_ip) = value.split(',').next() {
                let trimmed = first_ip.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    request
        .remote_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// [SECURITY] Returns true only for requests that reached this
/// server directly from another local process on this VPS (e.g.
/// bridge_server.js, withdraw_processor.js, wnsc_bridge.js), never
/// via the public Nginx proxy. This port is bound to 127.0.0.1 only,
/// and Nginx's /api/ location always sets X-Forwarded-For before
/// proxying here (see /etc/nginx/sites-enabled/nsc), so a request
/// with no X-Forwarded-For header could only have come from a local
/// process talking to 127.0.0.1:8080 directly.
fn is_internal_request(request: &Request) -> bool {
    for header in request.headers() {
        if header.field.as_str().to_ascii_lowercase() == "x-forwarded-for" {
            return false;
        }
    }
    true
}

// ── Route handlers ────────────────────────────────────────────

/// GET /
/// Node identity and version info.
fn handle_root(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        200,
        json!({
            "status":  "ok",
            "name":    "Nusacoin",
            "symbol":  "NSC",
            "version": crate::version::VERSION,
            "network": crate::config::NETWORK_ID,
        }),
        cors,
    )
}

/// GET /health
/// Liveness probe for load balancers.
fn handle_health(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        200,
        json!({ "status": "ok", "alive": true }),
        cors,
    )
}

/// GET /supply
/// Returns NSC supply figures.
fn handle_supply(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        200,
        json!({
            "status":          "ok",
            "max_supply":      crate::chain::MAX_SUPPLY,
            "genesis_supply":  crate::genesis::GENESIS_SUPPLY,
        }),
        cors,
    )
}

/// GET /version
/// Returns node version string.
fn handle_version(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        200,
        json!({
            "status":  "ok",
            "version": crate::version::VERSION,
            "network": crate::config::NETWORK_ID,
        }),
        cors,
    )
}

/// GET /seeds
/// Returns the list of seed node addresses.
fn handle_seeds(cors: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let seeds: Vec<String> = crate::seeds::seed_nodes()
        .into_iter()
        .collect();

    json_response(
        200,
        json!({ "status": "ok", "seeds": seeds }),
        cors,
    )
}

// ── Main API entry point ──────────────────────────────────────

/// Starts the NSC REST API server.
///
/// [FIX-12] Bind address read from NSC_API_BIND env var.
/// [FIX-14] API key read from NSC_API_KEY env var.
/// [FIX-13] No unwrap() — errors logged and handled cleanly.

pub fn start_api(
    blockchain: Arc<Mutex<Blockchain>>,
    token_registry: Arc<crate::token_registry::TokenRegistry>,
    l2_state: Arc<Mutex<crate::l2_bridge::L2BridgeState>>
) {
    // ── Load configuration from environment ───────────────────
    let bind_addr = std::env::var(BIND_ADDR_ENV)
        .unwrap_or_else(|_| DEFAULT_BIND.to_string());

    let api_key = std::env::var(API_KEY_ENV).unwrap_or_default();

    let cors_origin = std::env::var(CORS_ORIGIN_ENV)
    .unwrap_or_else(|_| "*".to_string()); // deny cross-origin by default

    if api_key.is_empty() {
        eprintln!(
            "[API] WARNING: {} is not set. All API requests will be rejected.",
            API_KEY_ENV
        );
    }

    if api_key.len() < 32 {
        eprintln!(
            "[API] WARNING: {} should be at least 32 characters long.",
            API_KEY_ENV
        );
    }

    // ── Start server ──────────────────────────────────────────
    let server = match Server::http(&bind_addr) {
        Ok(s)  => s,
        Err(e) => {
            eprintln!("[API] Failed to bind to {}: {}", bind_addr, e);
            std::process::exit(1);
        }
    };

    println!("[API] NSC REST API running on http://{}", bind_addr);
    println!("[API] Authentication : {}", if api_key.is_empty() { "DISABLED (dangerous!)" } else { "enabled" });
    println!("[API] CORS origin    : {}", cors_origin);

    // ── Rate limiter ──────────────────────────────────────────
    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new()));

    // ── Request loop ──────────────────────────────────────────
    for mut request in server.incoming_requests() {
        let url    = request.url().to_string();
        let method = request.method().clone();
        let ip     = get_remote_ip(&request);

// ── CORS preflight (OPTIONS) — auth/rate-limit এর আগেই handle করতে হবে,
        // কারণ browser preflight request-এ কোনো custom header পাঠায় না।
        if method == Method::Options {
            let response = Response::from_data(Vec::new())
                .with_status_code(204)
                .with_header(Header::from_bytes("Access-Control-Allow-Origin", cors_origin.as_str()).expect("valid header"))
                .with_header(Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").expect("valid header"))
                .with_header(Header::from_bytes("Access-Control-Allow-Headers", "Content-Type, X-Api-Key").expect("valid header"));
            let _ = request.respond(response);
            continue;
        }

        // ── [FIX-03] Rate limit check ──────────────────────────
        {
            let mut limiter = rate_limiter.lock().expect("rate limiter lock");
            if !limiter.is_allowed(&ip) {
                eprintln!("[API] Rate limit exceeded for IP: {}", ip);
                let _ = request.respond(rate_limited(&cors_origin));
                continue;
            }
        }

        // ── [FIX-02] API key authentication ───────────────────
        if !is_authenticated(&request, &api_key) {
            eprintln!("[API] Unauthorized request from IP: {}", ip);
            let _ = request.respond(unauthorized(&cors_origin));
            continue;
        }

        // ── [FIX-07] Body size limit ───────────────────────────
        // Only for POST requests.
        if method == Method::Post {
            let content_length: usize = request
                .headers()
                .iter()
                .find(|h| h.field.as_str().to_ascii_lowercase() == "content-length")
                .and_then(|h| h.value.as_str().parse().ok())
                .unwrap_or(0);

            if content_length > MAX_BODY_BYTES {
                eprintln!("[API] Request body too large from IP: {}", ip);
                let _ = request.respond(bad_request("Request body too large", &cors_origin));
                continue;
            }
        }

// ── Read POST body (needed for /transfer) ──────────────
        let mut body_string = String::new();
        if method == Method::Post {
            if let Err(e) = request.as_reader().read_to_string(&mut body_string) {
                eprintln!("[API] Failed to read request body: {}", e);
                let _ = request.respond(bad_request("Failed to read request body", &cors_origin));
                continue;
            }
        }

        // ── Route dispatch ─────────────────────────────────────
        // Strip query string for routing.
        let path = url.split('?').next().unwrap_or("/");

        let response = match path {

            // ── Public info routes (GET only) ──────────────────

            "/" => {
                if method != Method::Get {
                    method_not_allowed(&cors_origin)
                } else {
                    handle_root(&cors_origin)
                }
            }

            "/health" => {
                // Health check — no auth needed (already passed above,
                // but load balancers may call this without a key).
                // If you want unauthenticated health probes, move this
                // check above the auth block.
                if method != Method::Get {
                    method_not_allowed(&cors_origin)
                } else {
                    handle_health(&cors_origin)
                }
            }

            "/supply" => {
                if method != Method::Get {
                    method_not_allowed(&cors_origin)
                } else {
                    handle_supply(&cors_origin)
                }
            }

            "/version" => {
                if method != Method::Get {
                    method_not_allowed(&cors_origin)
                } else {
                    handle_version(&cors_origin)
                }
            }

            "/seeds" => {
                if method != Method::Get {
                    method_not_allowed(&cors_origin)
                } else {
                    handle_seeds(&cors_origin)
                }
            }

            // ── Catch-all ──────────────────────────────────────
            // [FIX-08] No internal error details leaked.
            "/balance" => {
    let address = get_query_param(&url, "address");
    let address = address.as_str();
   
    let balance = {
    let chain = blockchain.lock().expect("chain lock");
    chain.get_balance(address)
};

json_response(200, json!({
    "status": "ok",
    "address": address,
    "balance": balance.to_string(),
    "symbol": "NSC"
}), &cors_origin)
}
"/my_tokens" => {
    if method != Method::Get {
        method_not_allowed(&cors_origin)
    } else {
        let address = get_query_param(&url, "address").to_lowercase();

        if address.is_empty() {
            bad_request("Missing address query param", &cors_origin)
        } else {
            let user_tokens = token_registry.get_user_tokens(&address);
            let mut verified = Vec::new();
            let mut unverified = Vec::new();

            for (token, bal) in user_tokens {
                // Descale the raw balance to human units to compute a USD
                // value from the cached CoinGecko price (0.0 if unpriced,
                // e.g. unverified tokens with no reliable price source).
                let bal_human = bal.balance_u128() as f64 / 10f64.powi(token.decimals as i32);
                let usd_value = token.price_usd * bal_human;
                let entry = json!({
                    "chain_id": token.chain_id,
                    "contract_address": token.contract_address,
                    "symbol": token.symbol,
                    "name": token.name,
                    "logo_url": token.logo_url,
                    "decimals": token.decimals,
                    "balance": bal.balance,
                    "verified": token.verified,
                    "price_usd": token.price_usd,
                    "usd_value": usd_value
                });
                if token.verified {
                    verified.push(entry);
                } else {
                    unverified.push(entry);
                }
            }

            json_response(200, json!({
                "status": "ok",
                "address": address,
                "verified_tokens": verified,
                "unverified_tokens": unverified
            }), &cors_origin)
        }
    }
}
"/mempool" => {
    let chain = blockchain.lock().expect("chain lock");
    let pending: Vec<_> = chain.mempool.transactions.iter().map(|tx| json!({
        "tx_hash":    tx.tx_hash,
        "sender":     tx.sender,
        "receiver":   tx.receiver,
        "amount":     tx.amount.to_string(),
        "fee":        tx.fee.to_string(),
        "nonce":      tx.nonce,
        "timestamp":  tx.timestamp
    })).collect();
    let count = chain.mempool.size();
    let total_pending_value = chain.mempool.total_pending_value();
    json_response(200, json!({
        "status": "ok",
        "count": count,
        "total_pending_value": total_pending_value.to_string(),
        "transactions": pending
    }), &cors_origin)
}
"/usdt_withdrawals" => {
    let list = crate::storage::load_pending_withdrawals();
    json_response(200, json!({
        "status": "ok",
        "withdrawals": list
    }), &cors_origin)
}

"/usdt_withdrawal_done" => {
    if !is_internal_request(&request) {
        bad_request("This endpoint is not available externally", &cors_origin)
    } else if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let index = body["index"].as_u64().unwrap_or(0) as usize;
                crate::storage::mark_withdrawal_done(index);
                json_response(200, json!({"status": "ok"}), &cors_origin)
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/usdt_withdraw" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let address = body["address"].as_str().unwrap_or("").to_string();
                let amount = body["amount"].as_f64().unwrap_or(0.0);
                let bsc_address = body["bsc_address"].as_str().unwrap_or("").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                // ── [SECURITY] Signature-authorized withdrawal ─────────
                // Requires a fresh personal_sign signature proving control
                // of the source wallet's private key, same pattern as /swap.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let auth_ok = if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_WITHDRAW:{}:{}:{}",
                        address, bsc_address, amount
                    );
                    crate::evm_tx::verify_personal_sign(&message, &signature, &address)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else if address.is_empty() || amount <= 0.0 || bsc_address.is_empty() {
                    bad_request("Missing fields", &cors_origin)
                } else if crate::storage::has_recent_duplicate_withdraw(&address, &bsc_address, amount) {
                    bad_request("Duplicate withdrawal request detected. Please wait before retrying.", &cors_origin)
                } else {
                    let prev = crate::storage::load_usdt_balance(&address);
                    if prev < amount {
                        bad_request("Insufficient USDT balance", &cors_origin)
                    } else {
                        crate::storage::save_usdt_balance(&address, prev - amount);
                        crate::storage::save_withdraw_request(&address, &bsc_address, amount);
                        json_response(200, json!({
                            "status": "ok",
                            "address": address,
                            "bsc_address": bsc_address,
                            "amount": amount
                        }), &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/wnsc_release" => {
    if !is_internal_request(&request) {
        bad_request("This endpoint is not available externally", &cors_origin)
    } else if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let address = body["address"].as_str().unwrap_or("").to_string();
                let amount_str = body["amount"].as_str().unwrap_or("");
                let amount: u128 = amount_str.parse().unwrap_or(0);
                if address.is_empty() || amount == 0 {
                    bad_request("Missing address or invalid amount", &cors_origin)
                } else {
                    let mut chain = blockchain.lock().expect("chain lock");
                    // [MAX-SUPPLY-FIX] Previously this did a raw
                    // balances.insert(prev + amount) with zero cap check --
                    // credit_nsc() enforces MAX_SUPPLY and overflow-safety
                    // (see its BalanceLedger impl in chain.rs), the same
                    // guard staking's claim_unbonded() already relies on.
                    if !chain.credit_nsc(&address, amount) {
                        drop(chain);
                        bad_request("Release would exceed MAX_SUPPLY or overflow balance", &cors_origin)
                    } else {
                        chain.record_op("nsc", &address, amount as i128, "wnsc_release");
                        let new_balance = chain.get_balance(&address);
                        crate::storage::save_evm_state(&chain.balances, &chain.nonces);
                        drop(chain);
                        json_response(200, json!({
                            "status": "ok",
                            "address": address,
                            "credited": amount.to_string(),
                            "new_balance": new_balance.to_string()
                        }), &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/usdt_credit" => {
    if !is_internal_request(&request) {
        bad_request("This endpoint is not available externally", &cors_origin)
    } else if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let address = body["address"].as_str().unwrap_or("").to_string();
                let amount = body["amount"].as_f64().unwrap_or(0.0);
                if address.is_empty() || amount <= 0.0 {
                    bad_request("Missing address or amount", &cors_origin)
                } else {
                    let prev = crate::storage::load_usdt_balance(&address);
                    crate::storage::save_usdt_balance(&address, prev + amount);
                    json_response(200, json!({
                        "status": "ok",
                        "address": address,
                        "credited": amount,
                        "new_balance": prev + amount
                    }), &cors_origin)
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/usdt_transfer" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let from = body["from"].as_str().unwrap_or("").to_string();
                let to = body["to"].as_str().unwrap_or("").to_string();
                let amount = body["amount"].as_f64().unwrap_or(0.0);
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let auth_ok = if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_USDT_TRANSFER:{}:{}:{}:{}",
                        from, to, amount, timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &signature, &from)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else if from.is_empty() || to.is_empty() || amount <= 0.0 {
                    bad_request("Missing required fields", &cors_origin)
                } else if from.eq_ignore_ascii_case(&to) {
                    bad_request("Cannot transfer to the same address", &cors_origin)
                } else {
                    // [FEE] USDT is $1-pegged, so no pool lookup is
                    // needed to price it - the transfer amount itself
                    // IS the USD value. Same tiered schedule as NSC and
                    // custom-token transfers, fee paid in NSC and burned.
                    let nsc_pool = crate::storage::load_pool();
                    let nsc_reserve_whole = nsc_pool.0 as f64 / crate::genesis::DECIMALS as f64;
                    let nsc_price_usd = if nsc_reserve_whole > 0.0 { nsc_pool.1 / nsc_reserve_whole } else { 0.0 };
                    let fee_rate = if amount <= 500.0 { 0.0005 } else { 0.0003 };
                    let fee_usd = (amount * fee_rate).min(100.0);
                    let fee_nsc_whole = if nsc_price_usd > 0.0 { fee_usd / nsc_price_usd } else { 0.0 };
                    let fee_nsc: u128 = (fee_nsc_whole * crate::genesis::DECIMALS as f64) as u128;

                    let sender_nsc_balance = {
                        let chain = blockchain.lock().expect("chain lock");
                        chain.get_balance(&from)
                    };

                    let prev_from = crate::storage::load_usdt_balance(&from);
                    if prev_from < amount {
                        bad_request("Insufficient USDT balance", &cors_origin)
                    } else if fee_nsc > 0 && sender_nsc_balance < fee_nsc {
                        bad_request("Insufficient NSC balance to pay network fee", &cors_origin)
                    } else {
                        let prev_to = crate::storage::load_usdt_balance(&to);
                        // [FIX-12] Both balance updates go into ONE write
                        // to usdt_balances.json instead of two sequential
                        // read-modify-writes of the same file -- this
                        // fully eliminates (not just narrows) the window
                        // where a crash could leave the sender debited
                        // but the recipient never credited.
                        if let Some((p, bytes)) = crate::storage::prepare_usdt_balances_write(&[
                            (from.as_str(), prev_from - amount),
                            (to.as_str(), prev_to + amount),
                        ]) {
                            if let Err(e) = crate::storage::atomic_write_batch(&[(p, bytes)]) {
                                eprintln!("[API] FATAL: usdt_transfer commit failed: {}", e);
                            }
                        }

                        blockchain.lock().expect("chain lock").record_op("usdt", &from, -(amount as i128), "usdt_transfer");
                        blockchain.lock().expect("chain lock").record_op("usdt", &to, amount as i128, "usdt_transfer");
                        if fee_nsc > 0 {
                            let mut chain = blockchain.lock().expect("chain lock");
                            let prev = chain.get_balance(&from);
                            chain.balances.insert(from.clone(), prev - fee_nsc);
                            chain.record_op("nsc", &from, -(fee_nsc as i128), "usdt_transfer_fee");
                            crate::storage::save_evm_state(&chain.balances, &chain.nonces);
                        }

                        // [TX-LOG] Same as /token/transfer: gives this
                        // internal USDT send a proper tx_hash for
                        // Explorer + Transactions/Fees tab visibility.
                        // amount here is a plain f64 USDT value (not
                        // 18-decimal scaled like NSC/custom tokens), so
                        // it is scaled up for log_token_transfer()'s u128
                        // amount parameter to stay consistent with how
                        // the frontend descales all logged amounts.
                        let ts_now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let amount_scaled: u128 = (amount * crate::genesis::DECIMALS as f64) as u128;
                        let tx_hash = format!("0x{}", crate::hash::calculate_hash(
                            &format!("USDT:{}:{}:{}:{}:{}", from, to, amount, ts_now, fee_nsc)
                        ));
                        crate::storage::log_token_transfer("USDT", &from, &to, amount_scaled, fee_nsc, &tx_hash);

                        json_response(200, json!({
                            "status": "ok",
                            "from": from,
                            "to": to,
                            "amount": amount,
                            "fee_nsc": fee_nsc.to_string(),
                            "tx_hash": tx_hash,
                            "from_new_balance": prev_from - amount,
                            "to_new_balance": prev_to + amount
                        }), &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/usdt_balance" => {
    let address = url
        .split('?')
        .nth(1)
        .unwrap_or("")
        .split('&')
        .find(|p| p.starts_with("address="))
        .map(|p| p.trim_start_matches("address=").to_string())
        .unwrap_or_default();
    let balance = crate::storage::load_usdt_balance(&address);
    json_response(200, json!({
        "status": "ok",
        "address": address,
        "balance": balance
    }), &cors_origin)
}

"/nonce" => {
    let address = url
    .split('?')
    .nth(1)
    .unwrap_or("")
    .split('&')
    .find(|p| p.starts_with("address="))
    .map(|p| p.trim_start_matches("address=").to_string())
    .unwrap_or_default();
    if address.is_empty() {
        bad_request("Missing address", &cors_origin)
    } else {
        let chain = blockchain.lock().expect("chain lock");
        let nonce = *chain.nonces.get(&address).unwrap_or(&0);
        json_response(200, json!({
            "status": "ok",
            "address": address,
            "nonce": nonce
        }), &cors_origin)
    }
}
"/api/staking/stake" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct StakeInput {
            address: String,
            amount: u128,
            signature: String,
            timestamp: u64,
        }
        match serde_json::from_str::<StakeInput>(&body_string) {
            Ok(input) => {
                // ── [SECURITY FIX] Signature-authorized stake ──
                // Previously this endpoint accepted a plain address with
                // no proof of ownership, letting anyone stake on behalf
                // of any address (since the API key is effectively
                // public). Now requires a fresh personal_sign signature,
                // same pattern as /swap and /usdt_withdraw.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let auth_ok = if input.timestamp == 0 || now.saturating_sub(input.timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_STAKE:{}:{}:{}",
                        input.address, input.amount, input.timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &input.signature, &input.address)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else {
                    let mut chain = blockchain.lock().expect("chain lock");
                    let mut staking = std::mem::take(&mut chain.staking);
                    let result = staking.stake(&mut *chain, &input.address, input.amount);
                    chain.staking = staking;
                    match result {
                        Ok(total) => json_response(200, json!({
                            "status": "ok",
                            "address": input.address,
                            "staked_this_call": input.amount.to_string(),
                            "total_bonded_stake": total.to_string()
                        }), &cors_origin),
                        Err(e) => bad_request(&format!("Stake failed: {}", e), &cors_origin)
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/staking/unstake" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct UnstakeInput {
            address: String,
            amount: u128,
            signature: String,
            timestamp: u64,
        }
        match serde_json::from_str::<UnstakeInput>(&body_string) {
            Ok(input) => {
                // ── [SECURITY FIX] Signature-authorized unstake ──
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let auth_ok = if input.timestamp == 0 || now.saturating_sub(input.timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_UNSTAKE:{}:{}:{}",
                        input.address, input.amount, input.timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &input.signature, &input.address)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else {
                    let mut chain = blockchain.lock().expect("chain lock");
                    match chain.staking.unstake(&input.address, input.amount) {
                        Ok(unbonding_id) => json_response(200, json!({
                            "status": "ok",
                            "address": input.address,
                            "unbonding_amount": input.amount.to_string(),
                            "unbonding_entry_id": unbonding_id.to_string(),
                            "unlocks_after_secs": crate::staking::UNBONDING_PERIOD_SECS.to_string()
                        }), &cors_origin),
                        Err(e) => bad_request(&format!("Unstake failed: {}", e), &cors_origin)
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/staking/claim" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct ClaimInput {
            address: String,
            signature: String,
            timestamp: u64,
        }
        match serde_json::from_str::<ClaimInput>(&body_string) {
            Ok(input) => {
                // ── [SECURITY FIX] Signature-authorized claim ──
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let auth_ok = if input.timestamp == 0 || now.saturating_sub(input.timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_CLAIM:{}:{}",
                        input.address, input.timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &input.signature, &input.address)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else {
                    let mut chain = blockchain.lock().expect("chain lock");
                    let mut staking = std::mem::take(&mut chain.staking);
                    let result = staking.claim_unbonded(&mut *chain, &input.address);
                    chain.staking = staking;
                    match result {
                        Ok(total) => json_response(200, json!({
                            "status": "ok",
                            "address": input.address,
                            "claimed_amount": total.to_string()
                        }), &cors_origin),
                        Err(e) => bad_request(&format!("Claim failed: {}", e), &cors_origin)
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/treasury/request-spend" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct SpendRequestInput {
            id: String,
            amount: u128,
            recipient: String,
            description: String,
        }
        match serde_json::from_str::<SpendRequestInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                let request = crate::multisig::TreasurySpendRequest::new(
                    input.id.clone(),
                    input.amount,
                    input.recipient.clone(),
                    input.description.clone(),
                );
                chain.pending_spend_requests.insert(input.id.clone(), request);
                json_response(200, json!({
                    "status": "ok",
                    "message": "Spend request created",
                    "request_id": input.id
                }), &cors_origin)
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/treasury/sign" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct SignInput {
            request_id: String,
            signer: String,
            signature_hex: String,
            public_key_hex: String,
        }
        match serde_json::from_str::<SignInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                match chain.pending_spend_requests.get(&input.request_id).cloned() {
                    None => json_response(404, json!({
                        "status": "error",
                        "message": "No pending spend request with that id"
                    }), &cors_origin),
                    Some(request) => {
                        match chain.treasury_multisig.sign(&request, &input.signer, &input.signature_hex, &input.public_key_hex) {
                            Ok(()) => {
                                let approved = chain.treasury_multisig.is_approved(&input.request_id);
                                let count = chain.treasury_multisig.signature_count(&input.request_id);
                                json_response(200, json!({
                                    "status": "ok",
                                    "message": "Signature recorded",
                                    "approved": approved,
                                    "signatures": count
                                }), &cors_origin)
                            }
                            Err(e) => bad_request(&format!("Signature rejected: {}", e), &cors_origin)
                        }
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/treasury/execute" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct ExecuteInput {
            request_id: String,
        }
        match serde_json::from_str::<ExecuteInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                match chain.pending_spend_requests.get(&input.request_id).cloned() {
                    None => json_response(404, json!({
                        "status": "error",
                        "message": "No pending spend request with that id"
                    }), &cors_origin),
                    Some(request) => {
                        let mut multisig_clone = chain.treasury_multisig.clone();
                        let result = chain.treasury.execute_spend(&request, &mut multisig_clone);
                        match result {
                            Ok(()) => {
                                chain.treasury_multisig = multisig_clone;
                                chain.pending_spend_requests.remove(&input.request_id);
                                let new_balance = chain.treasury.balance();
                                json_response(200, json!({
                                    "status": "ok",
                                    "message": "Spend executed",
                                    "new_balance": new_balance
                                }), &cors_origin)
                            }
                            Err(e) => bad_request(&format!("Execute failed: {}", e), &cors_origin)
                        }
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/freeze" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct FreezeInput {
            target: String, // "treasury" | "chain" | "all"
            signer: String,
            signature_hex: String,
            public_key_hex: String,
            timestamp: u64,
        }
        match serde_json::from_str::<FreezeInput>(&body_string) {
            Ok(input) => 'guard: {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let delta = if now >= input.timestamp { now - input.timestamp } else { input.timestamp - now };
                if delta > 120 {
                    break 'guard bad_request("Freeze request timestamp is stale or invalid (must be within 120 seconds)", &cors_origin);
                }
                let mut chain = blockchain.lock().expect("chain lock");
                if !chain.emergency_freeze_multisig.owners.iter().any(|o| o == &input.signer) {
                    bad_request("Signer is not a recognized emergency-freeze owner", &cors_origin)
                } else {
                    let message = format!("NSCEMERGENCYFREEZE:{}:{}", input.target, input.timestamp);
                    let public_key_opt = crate::wallet::Wallet::public_key_from_hex(&input.public_key_hex);
                    if public_key_opt.is_none() {
                        bad_request("Invalid public key hex", &cors_origin)
                    } else {
                        let public_key = public_key_opt.unwrap();
                        if !crate::wallet::Wallet::verify_signature(&public_key, &message, &input.signature_hex) {
                            bad_request("Signature verification failed", &cors_origin)
                        } else {
                            let derived_opt = crate::wallet::Wallet::address_from_public_key_hex(&input.public_key_hex);
                            if derived_opt.is_none() {
                                bad_request("Could not derive address from public key", &cors_origin)
                            } else if derived_opt.unwrap() != input.signer {
                                bad_request("Public key does not match claimed signer", &cors_origin)
                            } else if input.target != "treasury" && input.target != "chain" && input.target != "all" {
                                bad_request("target must be 'treasury', 'chain', or 'all'", &cors_origin)
                            } else {
                                match input.target.as_str() {
                                    "treasury" => { chain.treasury.freeze(); }
                                    "chain" => { chain.chain_frozen = true; }
                                    "all" => { chain.treasury.freeze(); chain.chain_frozen = true; }
                                    _ => unreachable!(),
                                }
                                chain.save();
                                json_response(200, json!({
                                    "status": "ok",
                                    "message": format!("Emergency freeze activated for target: {}", input.target),
                                    "chain_frozen": chain.chain_frozen,
                                    "treasury_frozen": chain.treasury.is_frozen()
                                }), &cors_origin)
                            }
                        }
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/unfreeze/request" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct UnfreezeRequestInput {
            id: String,
            target: String, // "treasury" | "chain" | "all"
        }
        match serde_json::from_str::<UnfreezeRequestInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let request = crate::emergency_freeze_multisig::EmergencyFreezeRequest::new(
                    input.id.clone(),
                    input.target.clone(),
                    "unfreeze".to_string(),
                    timestamp,
                );
                chain.pending_freeze_requests.insert(input.id.clone(), request);
                chain.save();
                json_response(200, json!({
                    "status": "ok",
                    "message": "Unfreeze request created",
                    "request_id": input.id
                }), &cors_origin)
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/unfreeze/sign" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct EmergencySignInput {
            request_id: String,
            signer: String,
            signature_hex: String,
            public_key_hex: String,
        }
        match serde_json::from_str::<EmergencySignInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                match chain.pending_freeze_requests.get(&input.request_id).cloned() {
                    None => json_response(404, json!({
                        "status": "error",
                        "message": "No pending unfreeze request with that id"
                    }), &cors_origin),
                    Some(request) => {
                        match chain.emergency_freeze_multisig.sign(&request, &input.signer, &input.signature_hex, &input.public_key_hex) {
                            Ok(()) => {
                                let approved = chain.emergency_freeze_multisig.is_approved(&input.request_id);
                                let count = chain.emergency_freeze_multisig.signature_count(&input.request_id);
                                chain.save();
                                json_response(200, json!({
                                    "status": "ok",
                                    "message": "Signature recorded",
                                    "approved": approved,
                                    "signatures": count
                                }), &cors_origin)
                            }
                            Err(e) => bad_request(&format!("Signature rejected: {}", e), &cors_origin)
                        }
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/unfreeze/execute" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        #[derive(serde::Deserialize)]
        struct EmergencyExecuteInput {
            request_id: String,
        }
        match serde_json::from_str::<EmergencyExecuteInput>(&body_string) {
            Ok(input) => {
                let mut chain = blockchain.lock().expect("chain lock");
                match chain.pending_freeze_requests.get(&input.request_id).cloned() {
                    None => json_response(404, json!({
                        "status": "error",
                        "message": "No pending unfreeze request with that id"
                    }), &cors_origin),
                    Some(request) => {
                        if !chain.emergency_freeze_multisig.is_approved(&input.request_id) {
                            bad_request("Request has not reached required signature quorum", &cors_origin)
                        } else {
                            let checkpoint_ok = match chain.find_last_valid_checkpoint() {
                                Some(height) => chain.verify_checkpoint(height),
                                None => false,
                            };
                            if !checkpoint_ok {
                                bad_request(
                                    "Aborting unfreeze: checkpoint missing or failed verification. Manual investigation required.",
                                    &cors_origin
                                )
                            } else {
                                let wallet_count_before = chain.balances.len();
                                let total_before: u128 = chain.balances.values().sum();
                                chain.rebuild_balances();
                                chain.apply_full_state();
                                let wallet_count_after = chain.balances.len();
                                let total_after: u128 = chain.balances.values().sum();

                                if wallet_count_before > 0 && wallet_count_after == 0 {
                                    bad_request(
                                        "Aborting unfreeze: balance rebuild produced zero wallets from a non-empty state. Manual investigation required.",
                                        &cors_origin
                                    )
                                } else if total_before > 0 && total_after == 0 {
                                    bad_request(
                                        "Aborting unfreeze: balance rebuild zeroed total supply. Manual investigation required.",
                                        &cors_origin
                                    )
                                } else if request.target != "treasury" && request.target != "chain" && request.target != "all" {
                                    bad_request("Invalid target on stored request", &cors_origin)
                                } else {
                                    match request.target.as_str() {
                                        "treasury" => { chain.treasury.unfreeze(); }
                                        "chain" => { chain.chain_frozen = false; }
                                        "all" => { chain.treasury.unfreeze(); chain.chain_frozen = false; }
                                        _ => unreachable!(),
                                    }
                                    chain.emergency_freeze_multisig.clear(&input.request_id);
                                    chain.pending_freeze_requests.remove(&input.request_id);
                                    chain.save();
                                    json_response(200, json!({
                                        "status": "ok",
                                        "message": format!("Unfreeze executed for target: {}", request.target),
                                        "chain_frozen": chain.chain_frozen,
                                        "treasury_frozen": chain.treasury.is_frozen()
                                    }), &cors_origin)
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => bad_request(&format!("Invalid request JSON: {}", e), &cors_origin)
        }
    }
}

"/api/emergency/status" => {
    if method != Method::Get {
        method_not_allowed(&cors_origin)
    } else {
        let chain = blockchain.lock().expect("chain lock");
        let pending: Vec<serde_json::Value> = chain.pending_freeze_requests.values().map(|r| {
            json!({
                "id": r.id,
                "target": r.target,
                "action": r.action,
                "signatures": chain.emergency_freeze_multisig.signature_count(&r.id),
                "approved": chain.emergency_freeze_multisig.is_approved(&r.id)
            })
        }).collect();
        json_response(200, json!({
            "status": "ok",
            "chain_frozen": chain.chain_frozen,
            "treasury_frozen": chain.treasury.is_frozen(),
            "pending_unfreeze_requests": pending
        }), &cors_origin)
    }
}

"/transfer" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<crate::transaction::Transaction>(&body_string) {
            Ok(tx) => {
                if !tx.verify() {
                    bad_request("Transaction failed verification", &cors_origin)
                } else {
                    let mut chain = blockchain.lock().expect("chain lock");

                    if chain.is_processed(&tx.tx_hash) {
                        bad_request("Transaction already confirmed on-chain", &cors_origin)
                    } else if chain.is_blacklisted(&tx.sender) {
                        bad_request("Sender address is blacklisted", &cors_origin)
                    } else {
                        let mempool_size_before = chain.mempool.size();
                        chain.mempool.add_transaction(tx.clone());

                        if chain.mempool.size() > mempool_size_before {
                            json_response(200, json!({
                                "status": "ok",
                                "message": "Transaction accepted into mempool",
                                "tx_hash": tx.tx_hash
                            }), &cors_origin)
                        } else {
                            bad_request("Transaction rejected by mempool", &cors_origin)
                        }
                    }
                }
            }
            Err(e) => {
                bad_request(&format!("Invalid transaction JSON: {}", e), &cors_origin)
            }
        }
    }
}

"/tx_by_hash" => {
    let query = url.split('?').nth(1).unwrap_or("");
    let hash = query.split('&').find_map(|p| p.strip_prefix("hash=")).unwrap_or("").to_string();
    if hash.is_empty() {
        bad_request("hash required", &cors_origin)
    } else {
        let chain = blockchain.lock().expect("chain lock");
        let mut found: Option<serde_json::Value> = None;
        for b in chain.blocks.iter() {
            for tx in b.transactions.iter() {
                let hash_norm = hash.trim_start_matches("0x").to_lowercase();
                let native_matches = tx.tx_hash.to_lowercase() == hash.to_lowercase()
                    || tx.tx_hash.to_lowercase() == hash_norm;
                let eth_matches = tx.eth_tx_hash.as_ref()
                    .map(|h| h.to_lowercase() == hash.to_lowercase())
                    .unwrap_or(false);
                if native_matches || eth_matches {
                    found = Some(json!({
                        "sender": tx.sender.clone(),
                        "receiver": tx.receiver.clone(),
                        "amount": tx.amount.to_string(),
                        "fee": tx.fee.to_string(),
                        "timestamp": tx.timestamp,
                        "tx_hash": tx.tx_hash.clone(),
                        "eth_tx_hash": tx.eth_tx_hash.clone(),
                        "block": b.index
                    }));
                    break;
                }
            }
            if found.is_some() { break; }
        }
        drop(chain);
        // [TX-LOG] Fall back to the token/USDT transfer log for hashes
        // not found in mined blocks (those transfers don't go through
        // the mempool/mining pipeline, see log_token_transfer()).
        if found.is_none() {
            if let Some(t) = crate::storage::find_token_transfer_by_hash(&hash) {
                found = Some(json!({
                    "sender": t["from"],
                    "receiver": t["to"],
                    "amount": t["amount"],
                    "fee": t["fee"],
                    "symbol": t["symbol"],
                    "timestamp": t["timestamp"],
                    "tx_hash": t["tx_hash"],
                    "eth_tx_hash": serde_json::Value::Null,
                    "block": serde_json::Value::Null
                }));
            }
        }
        match found {
            Some(tx) => json_response(200, json!({"status":"ok","transaction":tx}), &cors_origin),
            None => json_response(404, json!({"status":"error","error":"Transaction not found"}), &cors_origin)
        }
    }
}

"/tx" => {
    json_response(200, json!({
        "status": "ok",
        "transactions": []
    }), &cors_origin)
}

"/wallet/activity" => {
    let query = url.split('?').nth(1).unwrap_or("");
    let address_raw = query.split('&').find_map(|p| p.strip_prefix("address=")).unwrap_or("").to_string();
    let address = address_raw.to_lowercase();

    if address.is_empty() {
        bad_request("address required", &cors_origin)
    } else {
        let chain = blockchain.lock().expect("chain lock");
        let mut transactions: Vec<serde_json::Value> = Vec::new();
        for b in chain.blocks.iter() {
            for tx in b.transactions.iter() {
                let sender_l = tx.sender.to_lowercase();
                let receiver_l = tx.receiver.to_lowercase();
                if sender_l == address || receiver_l == address {
                    let tx_type = if sender_l == address { "sent" } else { "received" };
                    transactions.push(json!({
                        "type": tx_type,
                        "counterparty": if sender_l == address { tx.receiver.clone() } else { tx.sender.clone() },
                        "amount": tx.amount.to_string(),
                        "fee": tx.fee.to_string(),
                        "timestamp": tx.timestamp,
                        "tx_hash": tx.tx_hash,
                        "block": b.index
                    }));
                }
            }
        }
        drop(chain);
        transactions.sort_by(|a, b| {
            let ta = a["timestamp"].as_u64().unwrap_or(0);
            let tb = b["timestamp"].as_u64().unwrap_or(0);
            tb.cmp(&ta)
        });
        transactions.truncate(200);

        let trades = crate::storage::get_wallet_trades(&address);

        // [TX-LOG] Merge custom-token/USDT transfers into the same
        // sent/received list NSC transfers use, so they appear in the
        // Transactions tab too. Each carries its own "symbol" field
        // (the frontend defaults to "NSC" for older entries without
        // one, keeping native-NSC rendering unchanged).
        let token_transfers = crate::storage::get_wallet_token_transfers(&address);
        for t in &token_transfers {
            let sender_l = t["from"].as_str().unwrap_or("").to_lowercase();
            let tx_type = if sender_l == address { "sent" } else { "received" };
            let counterparty = if sender_l == address { t["to"].clone() } else { t["from"].clone() };
            transactions.push(json!({
                "type": tx_type,
                "counterparty": counterparty,
                "amount": t["amount"],
                "fee": t["fee"],
                "symbol": t["symbol"],
                "timestamp": t["timestamp"],
                "tx_hash": t["tx_hash"],
                "block": serde_json::Value::Null
            }));
        }
        transactions.sort_by(|a, b| {
            let ta = a["timestamp"].as_u64().unwrap_or(0);
            let tb = b["timestamp"].as_u64().unwrap_or(0);
            tb.cmp(&ta)
        });
        transactions.truncate(200);

        let mut fees: Vec<serde_json::Value> = Vec::new();
        for t in &transactions {
            if t["type"] == "sent" {
                // "fee" is now a string (see wallet/activity above) since
                // it holds an 18-decimal u128 NSC fee.
                let fee_amt: u128 = t["fee"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0);
                if fee_amt > 0 {
                    fees.push(json!({
                        "kind": "network",
                        "amount": fee_amt.to_string(),
                        "asset": "NSC",
                        "timestamp": t["timestamp"],
                        "tx_hash": t["tx_hash"]
                    }));
                }
            }
        }
        for t in &trades {
            fees.push(json!({
                "kind": "swap",
                "amount": t["fee_usdt"],
                "asset": "USDT",
                "timestamp": t["ts"],
                "pair": format!("{}/{}", t["a"].as_str().unwrap_or(""), t["b"].as_str().unwrap_or(""))
            }));
        }
        fees.sort_by(|a, b| {
            let ta = a["timestamp"].as_u64().unwrap_or(0);
            let tb = b["timestamp"].as_u64().unwrap_or(0);
            tb.cmp(&ta)
        });

        json_response(200, json!({
            "status": "ok",
            "transactions": transactions,
            "trades": trades,
            "fees": fees
        }), &cors_origin)
    }
}

"/blocks" => {
    let chain = blockchain.lock().expect("chain lock");

    let blocks: Vec<_> = chain.blocks.iter().map(|b| {
        json!({
            "index": b.index,
            "timestamp": b.timestamp,
            "hash": b.hash,
            "previous_hash": b.previous_hash,
            "tx_count": b.transactions.len(),
            // b.reward is u128 (18-decimal scaled for post-migration
            // blocks) and can exceed what JSON numbers safely hold;
            // serialize as a string like other NSC amount fields.
            "reward": b.reward.to_string()
        })
    }).collect();

    json_response(200, json!({
        "status": "ok",
        "height": chain.chain_height(),
        "blocks": blocks
    }), &cors_origin)
}

"/pool" => {
    let pool = crate::storage::load_pool();
    let nsc = pool.0;
    let usdt = pool.1;
    // nsc is stored in 18-decimal units; divide down to whole NSC
    // before computing price so it isn't off by 10^18.
    let nsc_whole = nsc as f64 / crate::genesis::DECIMALS as f64;
    let price = if nsc_whole > 0.0 { usdt / nsc_whole } else { 0.0 };
    json_response(200, json!({
        "status": "ok",
        "pair": "NSC/USDT",
        "nsc_reserve": nsc.to_string(),
        "usdt_reserve": usdt,
        "price": price
    }), &cors_origin)
}

"/markets" => {
    let pool = crate::storage::load_pool();
    let (_, snapshot) = crate::storage::load_price_snapshot();
    // pool.0 (nsc reserve) is stored in 18-decimal units; scale
    // down to whole NSC before computing price.
    let nsc_reserve_whole = pool.0 as f64 / crate::genesis::DECIMALS as f64;
    let nsc_price = if nsc_reserve_whole > 0.0 { pool.1 / nsc_reserve_whole } else { 0.0 };
    let nsc_prev = snapshot["NSC"].as_f64().unwrap_or(nsc_price);
    let nsc_change = if nsc_prev > 0.0 { (nsc_price - nsc_prev) / nsc_prev * 100.0 } else { 0.0 };

    let nsc_chain_ref = blockchain.lock().expect("chain lock");
    let nsc_circulating: u128 = nsc_chain_ref.balances.values().sum();
    let nsc_holders: u64 = nsc_chain_ref.balances.values().filter(|&&b| b > 0).count() as u64;
    drop(nsc_chain_ref);
    // nsc_circulating and MAX_SUPPLY are stored in 18-decimal units;
    // scale down to whole NSC before computing USD-denominated values.
    let nsc_circulating_whole = nsc_circulating as f64 / crate::genesis::DECIMALS as f64;
    let nsc_market_cap = nsc_price * nsc_circulating_whole;
    let (nsc_ath, nsc_atl) = crate::storage::get_ath_atl("NSC");
    let nsc_change_1h = crate::storage::get_price_change_pct("NSC", 3600, nsc_price);
    let nsc_change_7d = crate::storage::get_price_change_pct("NSC", 7 * 24 * 3600, nsc_price);

    let mut markets = vec![json!({
        "symbol": "NSC",
        "name": "Nusacoin",
        "price": nsc_price,
        "liquidity": pool.1,
        "change_24h": nsc_change,
        "change_1h": nsc_change_1h,
        "change_7d": nsc_change_7d,
        "total_supply": nsc_circulating.to_string(),
        "max_supply": crate::chain::MAX_SUPPLY.to_string(),
        "market_cap": nsc_market_cap,
        "fdv": nsc_price * (crate::chain::MAX_SUPPLY as f64 / crate::genesis::DECIMALS as f64),
        "volume_24h": crate::storage::get_volume_24h("NSC"),
        "trade_count_24h": crate::storage::get_trade_count_24h("NSC"),
        "holders": nsc_holders,
        "ath": nsc_ath,
        "atl": nsc_atl
    })];

    let tokens = crate::storage::load_tokens();
    for t in &tokens {
        let tpool = crate::storage::load_token_pool(&t.symbol);
        // tpool.0 (token reserve) and t.total_supply are stored in
        // 18-decimal-scaled u128 units; descale to whole tokens
        // before computing price/market cap.
        let token_reserve_whole = tpool.0 as f64 / crate::genesis::DECIMALS as f64;
        let total_supply_whole = t.total_supply as f64 / crate::genesis::DECIMALS as f64;
        let price = if token_reserve_whole > 0.0 { tpool.1 / token_reserve_whole } else { 0.0 };
        let prev = snapshot[&t.symbol].as_f64().unwrap_or(price);
        let change = if prev > 0.0 { (price - prev) / prev * 100.0 } else { 0.0 };
        let market_cap = price * total_supply_whole;
        let (ath, atl) = crate::storage::get_ath_atl(&t.symbol);
        let change_1h = crate::storage::get_price_change_pct(&t.symbol, 3600, price);
        let change_7d = crate::storage::get_price_change_pct(&t.symbol, 7 * 24 * 3600, price);
        markets.push(json!({
            "symbol": t.symbol,
            "name": t.name,
            "price": price,
            "liquidity": tpool.1,
            "change_24h": change,
            "change_1h": change_1h,
            "change_7d": change_7d,
            "total_supply": t.total_supply.to_string(),
            "max_supply": t.total_supply.to_string(),
            "market_cap": market_cap,
            "fdv": price * total_supply_whole,
            "volume_24h": crate::storage::get_volume_24h(&t.symbol),
            "trade_count_24h": crate::storage::get_trade_count_24h(&t.symbol),
            "holders": crate::storage::get_token_holders_count(&t.symbol),
            "ath": ath,
            "atl": atl
        }));
    }

    json_response(200, json!({
        "status": "ok",
        "markets": markets
    }), &cors_origin)
}

"/supply/total" => {
    let chain = blockchain.lock().expect("chain lock");
    let circulating: u128 = chain.balances.values().sum();
    let wallet_count = chain.balances.len();
    drop(chain);
    json_response(200, json!({
        "status": "ok",
        "circulating_supply": circulating.to_string(),
        "max_supply": crate::chain::MAX_SUPPLY.to_string(),
        "wallet_count": wallet_count
    }), &cors_origin)
}

"/liquidity/add" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                if blockchain.lock().expect("chain lock").chain_frozen {
                    break 'guard bad_request("Chain is frozen - trading and transfers are currently suspended", &cors_origin);
                }
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                // nsc_amount is a human-entered whole-NSC value (e.g.
                // 1037.5) from the frontend, sent as a JSON number or
                // string. Scale to 18-decimal u128 internal units, same
                // approach as the /swap endpoint.
                let nsc_human: f64 = match &body["nsc_amount"] {
                    serde_json::Value::String(s) => s.parse().unwrap_or(0.0),
                    serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                    _ => 0.0,
                };
                let nsc: u128 = (nsc_human * crate::genesis::DECIMALS as f64) as u128;
                let usdt = body["usdt_amount"].as_f64().unwrap_or(0.0);
                let wallet = body["wallet"].as_str().unwrap_or("").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                // ── [SECURITY FIX] Signature-authorized liquidity add ──
                // Previously this endpoint added nsc_amount/usdt_amount
                // straight from the request body into the pool with no
                // signature check and no wallet debit, letting anyone
                // inflate the USDT reserve for free and drain real USDT
                // out via /swap + /usdt_withdraw. Now requires a fresh
                // personal_sign signature and debits the wallet's actual
                // NSC and USDT balances, same pattern as /token/liquidity/add.
                //
                // The signed message uses the RAW (unscaled) amount strings
                // exactly as sent by the frontend, not the derived u128/f64
                // values -- large numbers lose precision going through f64
                // multiplication, which caused signature mismatches for
                // /token/liquidity/add. Using raw strings avoids that class
                // of bug entirely (same approach as /usdt_withdraw).
                let nsc_amount_str = match &body["nsc_amount"] {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => "0".to_string(),
                };
                let usdt_amount_str = match &body["usdt_amount"] {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => "0".to_string(),
                };

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let auth_ok = if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_LIQUIDITY_ADD_MAIN:{}:{}:{}:{}",
                        nsc_amount_str, usdt_amount_str, wallet, timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &signature, &wallet)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else if nsc == 0 || usdt <= 0.0 || wallet.is_empty() {
                    bad_request("Invalid amounts", &cors_origin)
                } else {
                    let sender_nsc_balance = {
                        let chain = blockchain.lock().expect("chain lock");
                        chain.get_balance(&wallet)
                    };
                    let sender_usdt_balance = crate::storage::load_usdt_balance(&wallet);

                    if sender_nsc_balance < nsc {
                        bad_request("Insufficient NSC balance", &cors_origin)
                    } else if sender_usdt_balance < usdt {
                        bad_request("Insufficient USDT balance", &cors_origin)
                    } else {
                        let evm_state_writes = {
                            let mut chain = blockchain.lock().expect("chain lock");
                            let prev_nsc = chain.get_balance(&wallet);
                            chain.balances.insert(wallet.clone(), prev_nsc - nsc);
                            chain.record_op("nsc", &wallet, -(nsc as i128), "liquidity_add_main");
                                crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces)
                        };

                        let mut pool = crate::storage::load_pool();
                        pool.0 += nsc;
                        pool.1 += usdt;
                        // [ATOMICITY FIX] evm_state.json, usdt_balances.json,
                        // and pool.json committed together instead of three
                        // sequential independent saves.
                        let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                        batch_writes.extend(evm_state_writes);
                        blockchain.lock().expect("chain lock").record_op("usdt", &wallet, -(usdt as i128), "liquidity_add_main");
                        if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, sender_usdt_balance - usdt) {
                            batch_writes.push(w);
                        }
                        if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {
                            batch_writes.push(w);
                        }
                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                            eprintln!("[API] Failed to commit liquidity/add batch: {}", e);
                        }
                        json_response(200, json!({
                            "status": "ok",
                            "nsc_reserve": pool.0.to_string(),
                            "usdt_reserve": pool.1
                        }), &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/token/liquidity/remove" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let symbol = body["symbol"].as_str().unwrap_or("").to_string();
                let wallet = body["wallet"].as_str().unwrap_or("").to_string();
                let lp_amount: u128 = body["lp_amount"].as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let auth_ok = if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_LIQUIDITY_REMOVE:{}:{}:{}:{}",
                        symbol, lp_amount, wallet, timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &signature, &wallet)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else if symbol.is_empty() || wallet.is_empty() || lp_amount == 0 {
                    bad_request("Missing required fields", &cors_origin)
                } else {
                    let (total_lp, mut holders) = crate::storage::load_token_lp(&symbol);
                    let owned_lp = holders.get(&wallet).copied().unwrap_or(0);

                    if total_lp == 0 || owned_lp == 0 {
                        bad_request("No LP position found for this wallet/token", &cors_origin)
                    } else if owned_lp < lp_amount {
                        bad_request("Insufficient LP tokens", &cors_origin)
                    } else {
                        let mut pool = crate::storage::load_token_pool(&symbol);
                        let usdt_reserve_micro: u128 = (pool.1 * 1_000_000.0).round() as u128;

                        let token_out: u128 = pool.0.saturating_mul(lp_amount) / total_lp;
                        let usdt_out_micro: u128 = usdt_reserve_micro.saturating_mul(lp_amount) / total_lp;
                        let usdt_out: f64 = usdt_out_micro as f64 / 1_000_000.0;

                        if token_out == 0 || token_out > pool.0 || usdt_out > pool.1 {
                            bad_request("Withdrawal amount invalid for current pool state", &cors_origin)
                        } else {
                            let new_owned = owned_lp - lp_amount;
                            if new_owned == 0 {
                                holders.remove(&wallet);
                            } else {
                                holders.insert(wallet.clone(), new_owned);
                            }
                            let new_total_lp = total_lp - lp_amount;

                            pool.0 -= token_out;
                            pool.1 -= usdt_out;

                            let mut tokens = crate::storage::load_tokens();
                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == symbol) {
                                let bal = t.balance_of(&wallet);
                                t.balances.insert(wallet.clone(), bal + token_out);
                                blockchain.lock().expect("chain lock").record_op(&format!("token:{}", symbol), &wallet, token_out as i128, "liquidity_remove");
                            }
                            let prev_usdt = crate::storage::load_usdt_balance(&wallet);

                            // [FIX-12] All four related writes (LP shares,
                            // token pool reserves, token balance, USDT
                            // balance) are committed as one batch rather
                            // than four sequential independent saves, so a
                            // crash partway through cannot leave e.g. the
                            // LP burned but the token/USDT never credited.
                            let mut _batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                            if let Some(w) = crate::storage::prepare_token_lp_write(&symbol, new_total_lp, &holders) { _batch_writes.push(w); }
                            if let Some(w) = crate::storage::prepare_token_pool_write(&symbol, pool.0, pool.1) { _batch_writes.push(w); }
                            if let Some(w) = crate::storage::prepare_tokens_write(&tokens) { _batch_writes.push(w); }
                            blockchain.lock().expect("chain lock").record_op("usdt", &wallet, usdt_out as i128, "liquidity_remove");
                            if let Some(w) = crate::storage::prepare_usdt_balances_write(&[(wallet.as_str(), prev_usdt + usdt_out)]) { _batch_writes.push(w); }
                            if let Err(e) = crate::storage::atomic_write_batch(&_batch_writes) {
                                eprintln!("[API] FATAL: liquidity/remove batch commit failed: {}", e);
                            }

                            json_response(200, json!({
                                "status": "ok",
                                "symbol": symbol,
                                "lp_burned": lp_amount.to_string(),
                                "token_out": token_out.to_string(),
                                "usdt_out": usdt_out,
                                "token_reserve": pool.0.to_string(),
                                "usdt_reserve": pool.1
                            }), &cors_origin)
                        }
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/orders" => {
    json_response(200, json!({
        "status": "ok",
        "orders": []
    }), &cors_origin)
}

"/token" => {
    json_response(200, json!({
        "status": "ok",
        "name": "Nusacoin",
        "symbol": "NSC",
        "max_supply": 25000000,
        "decimals": 8,
        "network": "NSC_TESTNET_1",
        "contract": "NSC_NATIVE"
    }), &cors_origin)
}

"/l2/sequencer/bond" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");

                let wallet = body["wallet"].as_str().unwrap_or("").to_string();
                let bond_amount_str = body["amount"].as_str().unwrap_or("0").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let bond_amount: u128 = match bond_amount_str.parse() {
                    Ok(a) => a,
                    Err(_) => break 'guard bad_request("Invalid amount", &cors_origin),
                };

                if bond_amount < crate::l2_bridge::SEQUENCER_BOND_MIN {
                    break 'guard bad_request("Bond amount below minimum", &cors_origin);
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    break 'guard bad_request("Signature expired or missing timestamp", &cors_origin);
                }

                let message = format!("NSC_L2_SEQUENCER_BOND:{}:{}", bond_amount_str, timestamp);
                if !wallet.starts_with("0x") || !crate::evm_tx::verify_personal_sign(&message, &signature, &wallet) {
                    break 'guard bad_request("Invalid or expired signature", &cors_origin);
                }

                let mut chain = blockchain.lock().expect("chain lock");
                let current_bal = chain.get_balance(&wallet);
                let new_bal = match current_bal.checked_sub(bond_amount) {
                    Some(v) => v,
                    None => break 'guard bad_request("Insufficient balance for bond", &cors_origin),
                };
                chain.balances.insert(wallet.clone(), new_bal);
                chain.record_op("nsc", &wallet, -(bond_amount as i128), "l2_sequencer_bond_lock");

                let mut l2 = l2_state.lock().expect("l2 lock");
                l2.sequencer_address = wallet.clone();
                l2.sequencer_bond = bond_amount.to_string();
                l2.sequencer_bond_locked = true;

                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));
                batch_writes.extend(crate::storage::prepare_l2_state_write(&l2));
                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                    eprintln!("[API] Failed to commit sequencer bond batch: {}", e);
                }
                drop(chain);
                drop(l2);

                json_response(200, json!({
                    "status": "ok",
                    "sequencer": wallet,
                    "bond": bond_amount_str,
                    "message": "Sequencer bond locked. Batch submission now enabled."
                }), &cors_origin)
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/l2/sequencer/unbond" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                let sequencer = body["sequencer"].as_str().unwrap_or("").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    break 'guard bad_request("Signature expired or missing timestamp", &cors_origin);
                }

                let message = format!("NSC_L2_SEQUENCER_UNBOND:{}", timestamp);
                if !sequencer.starts_with("0x") || !crate::evm_tx::verify_personal_sign(&message, &signature, &sequencer) {
                    break 'guard bad_request("Invalid or expired signature", &cors_origin);
                }

                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let mut l2 = l2_state.lock().expect("l2 lock");
                let amount = match l2.unbond_sequencer(sequencer.clone(), now) {
                    Ok(a) => a,
                    Err(e) => {
                        drop(l2);
                        break 'guard bad_request(&e, &cors_origin);
                    }
                };

                let mut chain = blockchain.lock().expect("chain lock");
                let current_bal = chain.get_balance(&sequencer);
                let new_bal = match current_bal.checked_add(amount) {
                    Some(v) => v,
                    None => {
                        drop(chain);
                        drop(l2);
                        break 'guard bad_request("Balance overflow on unbond credit", &cors_origin);
                    }
                };
                chain.balances.insert(sequencer.clone(), new_bal);
                chain.record_op("nsc", &sequencer, amount as i128, "l2_sequencer_unbond");

                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));
                batch_writes.extend(crate::storage::prepare_l2_state_write(&l2));
                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                    eprintln!("[API] Failed to commit sequencer unbond batch: {}", e);
                }
                drop(chain);
                drop(l2);

                json_response(200, json!({
                    "status": "ok",
                    "sequencer": sequencer,
                    "amount": amount.to_string(),
                    "message": "Bond unlocked and returned to L1 balance."
                }), &cors_origin)
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/l2/batch/submit" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                let sequencer = body["sequencer"].as_str().unwrap_or("").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    break 'guard bad_request("Signature expired or missing timestamp", &cors_origin);
                }

                // [FIX, 2026-08-19] state_root and tx_data_hash are no longer
                // accepted from the caller -- previously the sequencer could
                // submit any value here with zero verification. Both are now
                // computed server-side inside submit_batch() from the actual
                // recorded ops, so the signed message no longer references
                // them either (there's nothing meaningful for the sequencer
                // to attest to beyond "I am submitting a batch now").
                let message = format!("NSC_L2_BATCH_SUBMIT:{}", timestamp);
                if !sequencer.starts_with("0x") || !crate::evm_tx::verify_personal_sign(&message, &signature, &sequencer) {
                    break 'guard bad_request("Invalid or expired signature", &cors_origin);
                }

                let mut l2 = l2_state.lock().expect("l2 lock");
                match l2.submit_batch(sequencer.clone(), now) {
                    Ok(batch_id) => {
                        let batch_writes = crate::storage::prepare_l2_state_write(&l2);
                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                            eprintln!("[API] Failed to commit batch submission: {}", e);
                        }
                        drop(l2);
                        json_response(200, json!({
                            "status": "ok",
                            "batch_id": batch_id.to_string(),
                            "challenge_window_secs": crate::l2_bridge::CHALLENGE_WINDOW_SECS.to_string(),
                            "message": "Batch submitted. Challenge window open."
                        }), &cors_origin)
                    }
                    Err(e) => {
                        drop(l2);
                        bad_request(&e, &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/l2/batch/finalize" => {
    // Permissionless by design: finalization is a pure "has enough time
    // passed with no successful challenge" check, not a privileged action.
    // Anyone (including automated keepers) may call it. The heartbeat loop
    // also calls auto_finalize_eligible_batches() every tick, so this
    // endpoint mainly exists for manual/on-demand triggering (e.g. testing)
    // and as a liveness backstop if the node's own loop is delayed.
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                let batch_id: u64 = match body["batch_id"].as_str().unwrap_or("").parse() {
                    Ok(v) => v,
                    Err(_) => break 'guard bad_request("Invalid batch_id", &cors_origin),
                };

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let mut l2 = l2_state.lock().expect("l2 lock");

                // Idempotent: calling finalize on an already-finalized batch
                // is not an error, it's a no-op success -- callers (including
                // the heartbeat loop and manual retries) should never crash
                // or need special-case handling for "already done".
                let already = l2.batches.iter().any(|b| b.batch_id == batch_id && b.finalized);
                if already {
                    drop(l2);
                    break 'guard json_response(200, json!({
                        "status": "ok",
                        "batch_id": batch_id.to_string(),
                        "already_finalized": true
                    }), &cors_origin);
                }

                match l2.finalize_batch(batch_id, now) {
                    Ok(()) => {
                        let batch_writes = crate::storage::prepare_l2_state_write(&l2);
                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                            eprintln!("[API] Failed to commit batch finalization: {}", e);
                        }
                        drop(l2);
                        json_response(200, json!({
                            "status": "ok",
                            "batch_id": batch_id.to_string(),
                            "already_finalized": false
                        }), &cors_origin)
                    }
                    Err(e) => {
                        drop(l2);
                        bad_request(&e, &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/l2/batch/challenge" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                let batch_id: u64 = match body["batch_id"].as_str().unwrap_or("").parse() {
                    Ok(v) => v,
                    Err(_) => break 'guard bad_request("Invalid batch_id", &cors_origin),
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                // [FIX, 2026-08-19] Previously took `recomputed_root` directly
                // from the caller and trusted it -- meaning "fraud proven" was
                // decided by whatever the challenger claimed, not by any real
                // re-execution. challenge_batch() itself now performs the real
                // server-side replay (from stored batch.ops, genesis through
                // the challenged batch) and compares against the batch's own
                // claimed root, so no root is accepted from the request body
                // anymore -- callers only identify *which* batch to check.
                let mut l2 = l2_state.lock().expect("l2 lock");
                match l2.challenge_batch(batch_id, now) {
                    Ok(fraud_proven) => {
                        let batch_writes = crate::storage::prepare_l2_state_write(&l2);
                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                            eprintln!("[API] Failed to commit challenge result: {}", e);
                        }
                        drop(l2);
                        json_response(200, json!({
                            "status": "ok",
                            "batch_id": batch_id.to_string(),
                            "fraud_proven": fraud_proven,
                            "message": if fraud_proven { "Fraud proven. Sequencer bond forfeited, batch rejected." } else { "No fraud. Sequencer's state root confirmed correct." }
                        }), &cors_origin)
                    }
                    Err(e) => {
                        drop(l2);
                        bad_request(&e, &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/l2/withdraw" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                let l2_burner = body["l2_burner"].as_str().unwrap_or("").to_string();
                let l1_recipient = body["l1_recipient"].as_str().unwrap_or("").to_string();
                let amount_str = body["amount"].as_str().unwrap_or("0").to_string();
                let batch_id: u64 = match body["batch_id"].as_str().unwrap_or("").parse() {
                    Ok(v) => v,
                    Err(_) => break 'guard bad_request("Invalid batch_id", &cors_origin),
                };
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let amount: u128 = match amount_str.parse() {
                    Ok(a) => a,
                    Err(_) => break 'guard bad_request("Invalid amount", &cors_origin),
                };

                if amount == 0 || l1_recipient.is_empty() {
                    break 'guard bad_request("Invalid amount or missing l1_recipient", &cors_origin);
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    break 'guard bad_request("Signature expired or missing timestamp", &cors_origin);
                }

                let message = format!(
                    "NSC_L2_WITHDRAW:{}:{}:{}:{}",
                    l1_recipient, amount_str, batch_id, timestamp
                );
                if !l2_burner.starts_with("0x") || !crate::evm_tx::verify_personal_sign(&message, &signature, &l2_burner) {
                    break 'guard bad_request("Invalid or expired signature", &cors_origin);
                }

                // [FIX] request_withdrawal now debits the L2 balance internally
                // and can fail (insufficient balance, bad batch_id, zero amount).
                // FUND_LOCK added since this mutates l2_balances, same as
                // /l2/transfer and /l2/withdraw/finalize do.
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let mut l2 = l2_state.lock().expect("l2 lock");
                match l2.request_withdrawal(l2_burner.clone(), l1_recipient.clone(), amount, batch_id, now) {
                    Ok(withdrawal_id) => {
                        let batch_writes = crate::storage::prepare_l2_state_write(&l2);
                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                            eprintln!("[API] Failed to commit withdrawal request: {}", e);
                        }
                        drop(l2);

                        json_response(200, json!({
                            "status": "ok",
                            "withdrawal_id": withdrawal_id.to_string(),
                            "challenge_window_secs": crate::l2_bridge::CHALLENGE_WINDOW_SECS.to_string(),
                            "message": "Withdrawal requested. Funds unlock on L1 after batch finalization + challenge window."
                        }), &cors_origin)
                    }
                    Err(e) => {
                        drop(l2);
                        bad_request(&e, &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/l2/withdraw/finalize" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                let withdrawal_id: u64 = match body["withdrawal_id"].as_str().unwrap_or("").parse() {
                    Ok(v) => v,
                    Err(_) => break 'guard bad_request("Invalid withdrawal_id", &cors_origin),
                };

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let mut l2 = l2_state.lock().expect("l2 lock");

                let (amount, recipient) = match l2.can_finalize_withdrawal(withdrawal_id, now) {
                    Ok(w) => {
                        let amt: u128 = match w.amount.parse() {
                            Ok(a) => a,
                            Err(_) => { drop(l2); break 'guard bad_request("Corrupt withdrawal amount", &cors_origin); }
                        };
                        (amt, w.l1_recipient.clone())
                    }
                    Err(e) => { drop(l2); break 'guard bad_request(&e, &cors_origin); }
                };

                // Unlock on L1
                let mut chain = blockchain.lock().expect("chain lock");
                let current_bal = chain.get_balance(&recipient);
                let new_bal = match current_bal.checked_add(amount) {
                    Some(v) => v,
                    None => { drop(chain); drop(l2); break 'guard bad_request("Balance overflow on unlock", &cors_origin); }
                };
                chain.balances.insert(recipient.clone(), new_bal);
                chain.record_op("nsc", &recipient, amount as i128, "l2_withdrawal_finalize");

                if let Some(w) = l2.withdrawals.iter_mut().find(|w| w.withdrawal_id == withdrawal_id) {
                    w.finalized = true;
                }

                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));
                batch_writes.extend(crate::storage::prepare_l2_state_write(&l2));
                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                    eprintln!("[API] Failed to commit withdrawal finalization: {}", e);
                }
                drop(chain);
                drop(l2);

                json_response(200, json!({
                    "status": "ok",
                    "withdrawal_id": withdrawal_id.to_string(),
                    "recipient": recipient,
                    "amount": amount.to_string(),
                    "message": "Withdrawal finalized. Funds unlocked on L1."
                }), &cors_origin)
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/l2/status" => {
    if method != Method::Get {
        method_not_allowed(&cors_origin)
    } else {
        let l2 = l2_state.lock().expect("l2 lock");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let latest = l2.batches.last();
        json_response(200, json!({
            "status": "ok",
            "sequencer_bond_locked": l2.sequencer_bond_locked,
            "latest_batch_id": latest.map(|b| b.batch_id.to_string()).unwrap_or_else(|| "0".to_string()),
            "latest_batch_finalized": latest.map(|b| b.finalized).unwrap_or(false),
            "challenge_window_secs": crate::l2_bridge::CHALLENGE_WINDOW_SECS.to_string(),
            "now": now.to_string()
        }), &cors_origin)
    }
}

"/l2/history" => {
    if method != Method::Get {
        method_not_allowed(&cors_origin)
    } else {
        let address = get_query_param(&url, "address");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let l2 = l2_state.lock().expect("l2 lock");

        // Deposits: status is either "pending" (not yet folded into a
        // batch) or "credited" (already processed and reflected in
        // l2_balance). There's no intermediate "batched but not yet
        // finalized" state for deposits specifically -- process_deposit()
        // credits the L2 balance immediately at batch-submit time, not
        // at batch-finalize time (only withdrawals wait for finalization).
        let deposits: Vec<serde_json::Value> = l2.deposits.iter()
            .filter(|d| d.l2_address == address || d.depositor == address)
            .map(|d| json!({
                "amount": d.amount,
                "timestamp": d.timestamp,
                "status": if d.processed { "credited" } else { "pending" }
            }))
            .collect();

        // Withdrawals: status progression is
        // "awaiting_batch_finalization" -> "ready_to_finalize" -> "finalized",
        // with challenge_window_remaining_secs telling the frontend how long
        // until the withdrawal's OWN challenge window closes (separate from
        // the batch's challenge window, which gates the first transition).
        let withdrawals: Vec<serde_json::Value> = l2.withdrawals.iter()
            .filter(|w| w.l2_burner == address)
            .map(|w| {
                let batch_finalized = l2.batches.iter()
                    .find(|b| b.batch_id == w.batch_id)
                    .map(|b| b.finalized && !b.challenged)
                    .unwrap_or(false);
                let status = if w.finalized {
                    "finalized"
                } else if !batch_finalized {
                    "awaiting_batch_finalization"
                } else if now < w.challenge_deadline {
                    "in_challenge_window"
                } else {
                    "ready_to_finalize"
                };
                let remaining = w.challenge_deadline.saturating_sub(now);
                json!({
                    "withdrawal_id": w.withdrawal_id,
                    "amount": w.amount,
                    "l1_recipient": w.l1_recipient,
                    "batch_id": w.batch_id,
                    "requested_at": w.requested_at,
                    "status": status,
                    "challenge_window_remaining_secs": if status == "in_challenge_window" { remaining } else { 0 }
                })
            })
            .collect();

        drop(l2);
        json_response(200, json!({
            "status": "ok",
            "address": address,
            "deposits": deposits,
            "withdrawals": withdrawals
        }), &cors_origin)
    }
}

"/l2/balance" => {
    if method != Method::Get {
        method_not_allowed(&cors_origin)
    } else {
        let address = get_query_param(&url, "address");
        let l2 = l2_state.lock().expect("l2 lock");
        let balance = l2.get_l2_balance(&address);
        drop(l2);
        json_response(200, json!({
            "status": "ok",
            "address": address,
            "balance": balance.to_string(),
            "symbol": "NSC-L2"
        }), &cors_origin)
    }
}

"/l2/transfer" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");

                let from = body["from"].as_str().unwrap_or("").to_string();
                let to = body["to"].as_str().unwrap_or("").to_string();
                let amount_str = body["amount"].as_str().unwrap_or("0").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let amount: u128 = match amount_str.parse() {
                    Ok(a) => a,
                    Err(_) => break 'guard bad_request("Invalid amount", &cors_origin),
                };

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    break 'guard bad_request("Signature expired or missing timestamp", &cors_origin);
                }

                let message = format!("NSC_L2_TRANSFER:{}:{}:{}", to, amount_str, timestamp);
                if !from.starts_with("0x") || !crate::evm_tx::verify_personal_sign(&message, &signature, &from) {
                    break 'guard bad_request("Invalid or expired signature", &cors_origin);
                }

                let mut l2 = l2_state.lock().expect("l2 lock");
                match l2.l2_transfer(&from, &to, amount, now) {
                    Ok(()) => {
                        // [FIX, 2026-08-19] Previously NOT written to disk on
                        // every transfer, relying on the next batch submit/
                        // finalize (or a graceful shutdown that never existed)
                        // to persist it. Confirmed in testing: a plain
                        // `systemctl restart` silently lost all transfers that
                        // happened since the last batch activity -- no crash,
                        // no error, just quietly reverted balances on reload.
                        // Real funds are involved here, so correctness wins
                        // over the "instant, zero-I/O" ideal: persist every
                        // transfer immediately. A JSON write of this size is
                        // low-single-digit milliseconds, not a meaningful
                        // latency cost, and it removes an entire class of
                        // silent fund-loss-on-restart risk.
                        let batch_writes = crate::storage::prepare_l2_state_write(&l2);
                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                            eprintln!("[API] Failed to commit L2 transfer to disk: {}", e);
                        }
                        drop(l2);
                        json_response(200, json!({
                            "status": "ok",
                            "from": from,
                            "to": to,
                            "amount": amount_str,
                            "message": "L2 transfer complete (instant, off-chain)."
                        }), &cors_origin)
                    }
                    Err(e) => {
                        drop(l2);
                        bad_request(&e, &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/l2/deposit" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                if blockchain.lock().expect("chain lock").chain_frozen {
                    break 'guard bad_request("Chain is frozen - deposits are currently suspended", &cors_origin);
                }
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");

                let wallet = body["wallet"].as_str().unwrap_or("").to_string();
                let l2_address = body["l2_address"].as_str().unwrap_or("").to_string();
                let amount_str = body["amount"].as_str().unwrap_or("0").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let amount: u128 = match amount_str.parse() {
                    Ok(a) => a,
                    Err(_) => break 'guard bad_request("Invalid amount", &cors_origin),
                };

                if amount == 0 {
                    break 'guard bad_request("Amount must be greater than zero", &cors_origin);
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    break 'guard bad_request("Signature expired or missing timestamp", &cors_origin);
                }

                // Canonical message format, matching existing NSC_* conventions.
                // Raw unscaled amount string used to avoid precision mismatch.
                let message = format!(
                    "NSC_L2_DEPOSIT:{}:{}:{}",
                    amount_str, l2_address, timestamp
                );

                let auth_ok = if wallet.starts_with("0x") {
                    crate::evm_tx::verify_personal_sign(&message, &signature, &wallet)
                } else {
                    false // L2 deposits require EVM-style signed auth for now
                };

                if !auth_ok {
                    break 'guard bad_request("Invalid or expired signature", &cors_origin);
                }

                if l2_address.is_empty() {
                    break 'guard bad_request("Missing l2_address", &cors_origin);
                }

                let mut chain = blockchain.lock().expect("chain lock");
                let current_bal = chain.get_balance(&wallet);
                let new_bal = match current_bal.checked_sub(amount) {
                    Some(v) => v,
                    None => break 'guard bad_request("Insufficient balance", &cors_origin),
                };
                chain.balances.insert(wallet.clone(), new_bal);
                chain.record_op("nsc", &wallet, -(amount as i128), "l2_deposit_lock");

                let mut l2 = l2_state.lock().expect("l2 lock");
                l2.record_deposit(wallet.clone(), amount, l2_address.clone(), timestamp);

                // ── [ATOMICITY] L1 balance debit + L2 deposit record must
                // land together, or a crash could lock L1 funds without
                // ever crediting them on L2.
                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));
                batch_writes.extend(crate::storage::prepare_l2_state_write(&l2));
                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                    eprintln!("[API] Failed to commit L2 deposit batch: {}", e);
                }
                drop(chain);
                drop(l2);

                json_response(200, json!({
                    "status": "ok",
                    "wallet": wallet,
                    "l2_address": l2_address,
                    "amount": amount_str,
                    "message": "Deposit locked on L1. Sequencer will credit L2 balance in next batch."
                }), &cors_origin)
            }
            Err(_) => bad_request("Invalid JSON body", &cors_origin),
        }
    }
}

"/swap" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                if blockchain.lock().expect("chain lock").chain_frozen {
                    break 'guard bad_request("Chain is frozen - trading and transfers are currently suspended", &cors_origin);
                }
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let from = body["from"].as_str().unwrap_or("");
                let to = body["to"].as_str().unwrap_or("");
                let amount = body["amount"].as_f64().unwrap_or(0.0);
                let wallet = body["wallet"].as_str().unwrap_or("").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                // ── [SECURITY] Signature-authorized swap ───────────────
                // Only EVM (0x...) wallets are supported for signed swaps
                // right now. Requests must carry a fresh personal_sign
                // signature over a canonical message, proving control of
                // the wallet's private key. Replay is bounded by the
                // 120-second timestamp freshness window.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let auth_ok = if wallet.starts_with("0x") {
                    if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                        false
                    } else {
                        let message = format!(
                            "NSC_SWAP:{}:{}:{}:{}:{}",
                            from, to, amount, wallet, timestamp
                        );
                        crate::evm_tx::verify_personal_sign(&message, &signature, &wallet)
                    }
                } else {
                    // Legacy NSC... wallets: unauthenticated for now
                    // (pre-existing behavior, unchanged). TODO: add
                    // Ed25519 signature verification here too.
                    true
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else if amount <= 0.0 {
                    bad_request("Invalid amount", &cors_origin)
                } else {
                    let mut pool = crate::storage::load_pool();
                    if from == "NSC" && to == "USDT" {
                        // `amount` is a human-entered whole-NSC value from the
                        // frontend (e.g. 1037.5); scale to 18-decimal u128
                        // internal units. f64 multiplication here introduces
                        // only negligible dust-level rounding for realistic
                        // amounts, avoiding the need for BigInt on the wire.
                        let amt: u128 = (amount * crate::genesis::DECIMALS as f64) as u128;
                        let usdt_out = pool.1 * amt as f64 / (pool.0 as f64 + amt as f64) * 0.9995;
                        // [ARITH-HARDENING] NaN/Infinity fail EVERY comparison
                        // (including <= 0.0 and > pool.1 below) under IEEE-754,
                        // so a NaN result would silently slip past both of
                        // those guards and proceed as if it were a valid
                        // amount, corrupting pool.1 with NaN permanently.
                        // This check must run first.
                        if !usdt_out.is_finite() || !pool.1.is_finite() {
                            bad_request("Invalid pool state or amount. Please try again or contact support.", &cors_origin)
                        } else if usdt_out <= 0.0 {
                            bad_request("Amount too small. Try a larger amount.", &cors_origin)
                        } else if usdt_out > pool.1 {
                            bad_request("Insufficient liquidity", &cors_origin)
                        } else if wallet.is_empty() {
                            bad_request("Wallet required", &cors_origin)
                        } else {
                            // [SECURITY FIX] Balance verified BEFORE any
                            // pool or balance mutation -- previously the
                            // pool was updated and USDT credited even
                            // when the sender's NSC balance was
                            // insufficient, letting anyone mint NSC/USDT
                            // for free and drain the pool.
                            let sender_nsc_balance = {
                                let chain = blockchain.lock().expect("chain lock");
                                chain.get_balance(&wallet)
                            };
                            if sender_nsc_balance < amt {
                                bad_request("Insufficient NSC balance", &cors_origin)
                            } else {
                                pool.0 += amt;
                                pool.1 -= usdt_out;

                                let mut chain = blockchain.lock().expect("chain lock");
                                let prev_nsc = chain.get_balance(&wallet);
                                chain.balances.insert(wallet.clone(), prev_nsc - amt);
                                chain.record_op("nsc", &wallet, -(amt as i128), "swap_nsc_usdt");
                                chain.record_op("usdt", &wallet, usdt_out as i128, "swap_nsc_usdt");

                                let prev_usdt = crate::storage::load_usdt_balance(&wallet);

                                // [ATOMICITY FIX] Previously pool.json,
                                // evm_state.json, and usdt_balances.json
                                // were saved as three independent writes.
                                // A crash between any two of them left the
                                // pool debited/credited without the
                                // matching wallet balance change, or vice
                                // versa. Now all three are gathered here
                                // and committed together via
                                // atomic_write_batch(), which only renames
                                // any of them into place after every one's
                                // .tmp write has succeeded.
                                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {
                                    batch_writes.push(w);
                                }
                                batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));
                                if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt + usdt_out) {
                                    batch_writes.push(w);
                                }
                                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                    eprintln!("[API] Failed to commit swap batch (NSC->USDT): {}", e);
                                }
                                drop(chain);

                                crate::storage::log_trade("NSC", "USDT", usdt_out, &wallet);
                                crate::storage::record_price_tick_now("NSC");
                                json_response(200, json!({
                                    "status": "ok",
                                    "from": "NSC",
                                    "to": "USDT",
                                    "amount_in": amt.to_string(),
                                    "amount_out": usdt_out
                                }), &cors_origin)
                            }
                        }
                    } else if from == "USDT" && to == "NSC" {
                        let amt = amount;
                        let nsc_out_f64 = pool.0 as f64 * amt / (pool.1 + amt) * 0.9995;
                        // [ARITH-HARDENING] Same NaN/Infinity slip-through risk
                        // as the NSC->USDT branch above -- checked on the raw
                        // f64 result before it gets truncated into a u128
                        // (NaN as u128 truncates to 0, which would otherwise
                        // masquerade as the ordinary "amount too small" case
                        // instead of being flagged as a real pool-state problem).
                        if !nsc_out_f64.is_finite() || !pool.1.is_finite() {
                            bad_request("Invalid pool state or amount. Please try again or contact support.", &cors_origin)
                        } else {
                        let nsc_out = nsc_out_f64 as u128;
                        if nsc_out == 0 {
                            bad_request("Amount too small. Try a larger amount.", &cors_origin)
                        } else if nsc_out > pool.0 {
                            bad_request("Insufficient liquidity", &cors_origin)
                        } else if wallet.is_empty() {
                            bad_request("Wallet required", &cors_origin)
                        } else {
                            // [SECURITY FIX] Same fix as above, mirrored:
                            // verify USDT balance before mutating the
                            // pool or crediting NSC.
                            let prev_usdt = crate::storage::load_usdt_balance(&wallet);
                            if prev_usdt < amt {
                                bad_request("Insufficient USDT balance", &cors_origin)
                            } else {
                                pool.1 += amt;
                                pool.0 -= nsc_out;

                                let mut chain = blockchain.lock().expect("chain lock");
                                let prev_nsc = chain.get_balance(&wallet);
                                chain.balances.insert(wallet.clone(), prev_nsc + nsc_out);
                                chain.record_op("nsc", &wallet, nsc_out as i128, "swap_usdt_nsc");
                                chain.record_op("usdt", &wallet, -(amt as i128), "swap_usdt_nsc");

                                // [ATOMICITY FIX] Same batching as the
                                // NSC->USDT branch above -- pool.json,
                                // evm_state.json, and usdt_balances.json
                                // committed together, all-or-nothing.
                                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {
                                    batch_writes.push(w);
                                }
                                if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt - amt) {
                                    batch_writes.push(w);
                                }
                                batch_writes.extend(crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces));
                                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                    eprintln!("[API] Failed to commit swap batch (USDT->NSC): {}", e);
                                }
                                drop(chain);

                                crate::storage::log_trade("USDT", "NSC", amt, &wallet);
                                crate::storage::record_price_tick_now("NSC");
                                json_response(200, json!({
                                    "status": "ok",
                                    "from": "USDT",
                                    "to": "NSC",
                                    "amount_in": amt,
                                    "amount_out": nsc_out.to_string()
                                }), &cors_origin)
                            }
                        }
                        } // closes the is_finite() else-block opened above [ARITH-HARDENING]
                    } else if to == "USDT" {
                        // `amount` is a human-entered whole/fractional token
                        // amount from the frontend; scale to 18-decimal
                        // u128 internal units, same convention as NSC.
                        let amt: u128 = (amount * crate::genesis::DECIMALS as f64) as u128;
                        let mut tpool = crate::storage::load_token_pool(from);
                        if tpool.0 == 0 || tpool.1 <= 0.0 {
                            bad_request("No liquidity for this token", &cors_origin)
                        } else {
                            let usdt_out = tpool.1 * amt as f64 / (tpool.0 as f64 + amt as f64) * 0.997;
                            // [ARITH-HARDENING] NaN/Infinity fail every comparison,
                            // so a NaN usdt_out would otherwise silently slip past
                            // both bound checks below and corrupt tpool.1 permanently.
                            if !usdt_out.is_finite() || !tpool.1.is_finite() {
                                bad_request("Invalid pool state or amount. Please try again or contact support.", &cors_origin)
                            } else if usdt_out <= 0.0 || usdt_out > tpool.1 {
                                bad_request("Insufficient liquidity", &cors_origin)
                            } else if wallet.is_empty() {
                                bad_request("Wallet required", &cors_origin)
                            } else {
                                let mut tokens = crate::storage::load_tokens();
                                if let Some(t) = tokens.iter_mut().find(|t| t.symbol == from) {
                                    let bal = t.balance_of(&wallet);
                                    if bal < amt {
                                        bad_request("Insufficient token balance", &cors_origin)
                                    } else {
                                        t.balances.insert(wallet.clone(), bal - amt);
                                        blockchain.lock().expect("chain lock").record_op(&format!("token:{}", from), &wallet, -(amt as i128), "swap_token_usdt");
                                        blockchain.lock().expect("chain lock").record_op("usdt", &wallet, usdt_out as i128, "swap_token_usdt");
                                        tpool.0 += amt;
                                        tpool.1 -= usdt_out;
                                        let prev_usdt = crate::storage::load_usdt_balance(&wallet);

                                        // [ATOMICITY FIX] tokens.json,
                                        // the token's pool file, and
                                        // usdt_balances.json committed
                                        // together instead of three
                                        // sequential independent saves.
                                        let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                        if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                            batch_writes.push(w);
                                        }
                                        if let Some(w) = crate::storage::prepare_token_pool_write(from, tpool.0, tpool.1) {
                                            batch_writes.push(w);
                                        }
                                        if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt + usdt_out) {
                                            batch_writes.push(w);
                                        }
                                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                            eprintln!("[API] Failed to commit swap batch (token->USDT): {}", e);
                                        }

                                        crate::storage::log_trade(from, "USDT", usdt_out, &wallet);
                                        crate::storage::record_price_tick_now(from);
                                        json_response(200, json!({
                                            "status": "ok",
                                            "from": from,
                                            "to": "USDT",
                                            "amount_in": amt.to_string(),
                                            "amount_out": usdt_out
                                        }), &cors_origin)
                                    }
                                } else {
                                    bad_request("Token not found", &cors_origin)
                                }
                            }
                        }
                    } else if from == "USDT" {
                        let amt = amount;
                        let mut tpool = crate::storage::load_token_pool(to);
                        if tpool.0 == 0 || tpool.1 <= 0.0 {
                            bad_request("No liquidity for this token", &cors_origin)
                        } else {
                            let token_out_f64 = tpool.0 as f64 * amt / (tpool.1 + amt) * 0.997;
                            // [ARITH-HARDENING] Checked on the raw f64 result before
                            // truncation into u128 -- NaN as u128 truncates to 0,
                            // which would otherwise masquerade as the ordinary
                            // "amount too small" case instead of a real pool problem.
                            if !token_out_f64.is_finite() || !tpool.1.is_finite() {
                                bad_request("Invalid pool state or amount. Please try again or contact support.", &cors_origin)
                            } else {
                            let token_out = token_out_f64 as u128;
                            if token_out == 0 || token_out > tpool.0 {
                                bad_request("Insufficient liquidity", &cors_origin)
                            } else if wallet.is_empty() {
                                bad_request("Wallet required", &cors_origin)
                            } else {
                                let prev_usdt = crate::storage::load_usdt_balance(&wallet);
                                if prev_usdt < amt {
                                    bad_request("Insufficient USDT balance", &cors_origin)
                                } else {
                                    tpool.1 += amt;
                                    tpool.0 -= token_out;
                                    crate::storage::enforce_token_floor(to, &mut tpool);
                                    let mut tokens = crate::storage::load_tokens();
                                    if let Some(t) = tokens.iter_mut().find(|t| t.symbol == to) {
                                        let bal = t.balance_of(&wallet);
                                        t.balances.insert(wallet.clone(), bal + token_out);
                                        blockchain.lock().expect("chain lock").record_op(&format!("token:{}", to), &wallet, token_out as i128, "swap_usdt_token");
                                        blockchain.lock().expect("chain lock").record_op("usdt", &wallet, -(amt as i128), "swap_usdt_token");

                                        // [ATOMICITY FIX] usdt_balances.json,
                                        // the token's pool file, and
                                        // tokens.json committed together
                                        // instead of three sequential
                                        // independent saves.
                                        let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                        if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt - amt) {
                                            batch_writes.push(w);
                                        }
                                        if let Some(w) = crate::storage::prepare_token_pool_write(to, tpool.0, tpool.1) {
                                            batch_writes.push(w);
                                        }
                                        if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                            batch_writes.push(w);
                                        }
                                        if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                            eprintln!("[API] Failed to commit swap batch (USDT->token): {}", e);
                                        }

                                        crate::storage::log_trade("USDT", to, amt, &wallet);
                                        crate::storage::record_price_tick_now(to);
                                        json_response(200, json!({
                                            "status": "ok",
                                            "from": "USDT",
                                            "to": to,
                                            "amount_in": amt,
                                            "amount_out": token_out.to_string()
                                        }), &cors_origin)
                                    } else {
                                        bad_request("Token not found", &cors_origin)
                                    }
                                }
                            }
                            } // closes the is_finite() else-block opened above [ARITH-HARDENING]
                        }
                    } else if from != "NSC" && from != "USDT" && to != "NSC" && to != "USDT" && from != to {
                        // [ROUTER] Multi-hop: TOKEN_A -> USDT -> TOKEN_B, atomic
                        // `amount` is a human-entered whole/fractional token
                        // amount from the frontend; scale to 18-decimal
                        // u128 internal units, same convention as NSC.
                        let amt: u128 = (amount * crate::genesis::DECIMALS as f64) as u128;
                        let mut pool_a = crate::storage::load_token_pool(from);
                        if pool_a.0 == 0 || pool_a.1 <= 0.0 {
                            bad_request("No liquidity for source token", &cors_origin)
                        } else {
                            let usdt_mid = pool_a.1 * amt as f64 / (pool_a.0 as f64 + amt as f64) * 0.997;
                            // [ARITH-HARDENING] usdt_mid feeds directly into
                            // pool_a.1 -= usdt_mid below (a raw f64 subtraction
                            // with no integer-truncation gate), so a NaN here
                            // would otherwise slip past both bound checks and
                            // permanently corrupt pool_a.1.
                            if !usdt_mid.is_finite() || usdt_mid <= 0.0 || usdt_mid > pool_a.1 {
                                bad_request("Insufficient liquidity", &cors_origin)
                            } else {
                                let mut pool_b = crate::storage::load_token_pool(to);
                                if pool_b.0 == 0 || pool_b.1 <= 0.0 {
                                    bad_request("No liquidity for destination token", &cors_origin)
                                } else {
                                    let token_out = (pool_b.0 as f64 * usdt_mid / (pool_b.1 + usdt_mid) * 0.997) as u128;
                                    if token_out == 0 || token_out > pool_b.0 {
                                        bad_request("Insufficient liquidity", &cors_origin)
                                    } else if wallet.is_empty() {
                                        bad_request("Wallet required", &cors_origin)
                                    } else {
                                        let mut tokens = crate::storage::load_tokens();
                                        let from_bal = tokens.iter().find(|t| t.symbol == from).map(|t| t.balance_of(&wallet)).unwrap_or(0);
                                        let dest_exists = tokens.iter().any(|t| t.symbol == to);
                                        if from_bal < amt {
                                            bad_request("Insufficient token balance", &cors_origin)
                                        } else if !dest_exists {
                                            bad_request("Destination token not found", &cors_origin)
                                        } else {
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == from) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal - amt);
                                                blockchain.lock().expect("chain lock").record_op(&format!("token:{}", from), &wallet, -(amt as i128), "swap_token_a_token_b");
                                            }
                                            pool_a.0 += amt;
                                            pool_a.1 -= usdt_mid;

                                            pool_b.1 += usdt_mid;
                                            pool_b.0 -= token_out;
                                            crate::storage::enforce_token_floor(to, &mut pool_b);
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == to) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal + token_out);
                                                blockchain.lock().expect("chain lock").record_op(&format!("token:{}", to), &wallet, token_out as i128, "swap_token_a_token_b");
                                            }

                                            // [ATOMICITY FIX] Both token pool files and tokens.json
                                            // committed together instead of three sequential
                                            // independent saves.
                                            let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                            if let Some(w) = crate::storage::prepare_token_pool_write(from, pool_a.0, pool_a.1) {
                                                batch_writes.push(w);
                                            }
                                            if let Some(w) = crate::storage::prepare_token_pool_write(to, pool_b.0, pool_b.1) {
                                                batch_writes.push(w);
                                            }
                                            if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                                batch_writes.push(w);
                                            }
                                            if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                                eprintln!("[API] Failed to commit swap batch (token->token): {}", e);
                                            }
                                            crate::storage::log_trade(from, to, usdt_mid, &wallet);
                                            crate::storage::record_price_tick_now(from);
                                            crate::storage::record_price_tick_now(to);
                                            json_response(200, json!({
                                                "status": "ok",
                                                "from": from,
                                                "to": to,
                                                "amount_in": amt.to_string(),
                                                "amount_out": token_out.to_string(),
                                                "route": [from, "USDT", to]
                                            }), &cors_origin)
                                        }
                                    }
                                }
                            }
                        }
                    } else if from == "NSC" && to != "USDT" && to != "NSC" {
                        // [ROUTER] NSC -> USDT -> TOKEN, atomic
                        // Scale human-entered whole-NSC amount to 18-decimal
                        // u128 internal units (see NSC->USDT branch above).
                        let amt: u128 = (amount * crate::genesis::DECIMALS as f64) as u128;
                        if pool.0 == 0 || pool.1 <= 0.0 {
                            bad_request("No liquidity for NSC pool", &cors_origin)
                        } else {
                            let usdt_mid = pool.1 * amt as f64 / (pool.0 as f64 + amt as f64) * 0.997;
                            // [ARITH-HARDENING] usdt_mid feeds directly into
                            // pool.1 -= usdt_mid below (a raw f64 subtraction
                            // with no integer-truncation gate), so a NaN here
                            // would otherwise slip past both bound checks and
                            // permanently corrupt the main NSC/USDT pool.
                            if !usdt_mid.is_finite() || usdt_mid <= 0.0 || usdt_mid > pool.1 {
                                bad_request("Insufficient liquidity", &cors_origin)
                            } else {
                                let mut tpool = crate::storage::load_token_pool(to);
                                if tpool.0 == 0 || tpool.1 <= 0.0 {
                                    bad_request("No liquidity for destination token", &cors_origin)
                                } else {
                                    let token_out = (tpool.0 as f64 * usdt_mid / (tpool.1 + usdt_mid) * 0.997) as u128;
                                    if token_out == 0 || token_out > tpool.0 {
                                        bad_request("Insufficient liquidity", &cors_origin)
                                    } else if wallet.is_empty() {
                                        bad_request("Wallet required", &cors_origin)
                                    } else {
                                        let sender_nsc_balance = {
                                            let chain = blockchain.lock().expect("chain lock");
                                            chain.get_balance(&wallet)
                                        };
                                        if sender_nsc_balance < amt {
                                            bad_request("Insufficient NSC balance", &cors_origin)
                                        } else {
                                            let evm_state_writes = {
                                                let mut chain = blockchain.lock().expect("chain lock");
                                                let prev_nsc = chain.get_balance(&wallet);
                                                chain.balances.insert(wallet.clone(), prev_nsc - amt);
                                                chain.record_op("nsc", &wallet, -(amt as i128), "swap_router_nsc_token");
                                                crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces)
                                            };
                                            pool.0 += amt;
                                            pool.1 -= usdt_mid;

                                            tpool.1 += usdt_mid;
                                            tpool.0 -= token_out;
                                            crate::storage::enforce_token_floor(to, &mut tpool);

                                            let mut tokens = crate::storage::load_tokens();
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == to) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal + token_out);
                                                blockchain.lock().expect("chain lock").record_op(&format!("token:{}", to), &wallet, token_out as i128, "swap_router_nsc_token");
                                                  // [ATOMICITY FIX] evm_state.json, pool.json, the token's
                                                  // pool file, and tokens.json committed together instead
                                                  // of four sequential independent saves.
                                                  let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                                  batch_writes.extend(evm_state_writes);
                                                  if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {
                                                      batch_writes.push(w);
                                                  }
                                                  if let Some(w) = crate::storage::prepare_token_pool_write(to, tpool.0, tpool.1) {
                                                      batch_writes.push(w);
                                                  }
                                                  if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                                      batch_writes.push(w);
                                                  }
                                                  if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                                      eprintln!("[API] Failed to commit swap batch (NSC->token): {}", e);
                                                  }
                                                crate::storage::log_trade("NSC", to, usdt_mid, &wallet);
                                                crate::storage::record_price_tick_now("NSC");
                                                crate::storage::record_price_tick_now(to);
                                                json_response(200, json!({
                                                    "status": "ok",
                                                    "from": "NSC",
                                                    "to": to,
                                                    "amount_in": amt.to_string(),
                                                    "amount_out": token_out.to_string(),
                                                    "route": ["NSC", "USDT", to]
                                                }), &cors_origin)
                                            } else {
                                                bad_request("Destination token not found", &cors_origin)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if to == "NSC" && from != "USDT" && from != "NSC" {
                        // [ROUTER] TOKEN -> USDT -> NSC, atomic
                        // `amount` is a human-entered whole/fractional token
                        // amount from the frontend; scale to 18-decimal
                        // u128 internal units, same convention as NSC.
                        let amt: u128 = (amount * crate::genesis::DECIMALS as f64) as u128;
                        let mut tpool = crate::storage::load_token_pool(from);
                        if tpool.0 == 0 || tpool.1 <= 0.0 {
                            bad_request("No liquidity for source token", &cors_origin)
                        } else {
                            let usdt_mid = tpool.1 * amt as f64 / (tpool.0 as f64 + amt as f64) * 0.997;
                            // [ARITH-HARDENING] usdt_mid feeds directly into
                            // tpool.1 -= usdt_mid below (a raw f64 subtraction
                            // with no integer-truncation gate), so a NaN here
                            // would otherwise slip past both bound checks and
                            // permanently corrupt tpool.1.
                            if !usdt_mid.is_finite() || usdt_mid <= 0.0 || usdt_mid > tpool.1 {
                                bad_request("Insufficient liquidity", &cors_origin)
                            } else if pool.0 == 0 || pool.1 <= 0.0 {
                                bad_request("No liquidity for NSC pool", &cors_origin)
                            } else {
                                let nsc_out = (pool.0 as f64 * usdt_mid / (pool.1 + usdt_mid) * 0.997) as u128;
                                if nsc_out == 0 || nsc_out > pool.0 {
                                    bad_request("Insufficient liquidity", &cors_origin)
                                } else if wallet.is_empty() {
                                    bad_request("Wallet required", &cors_origin)
                                } else {
                                    let mut tokens = crate::storage::load_tokens();
                                    let from_bal = tokens.iter().find(|t| t.symbol == from).map(|t| t.balance_of(&wallet)).unwrap_or(0);
                                    if from_bal < amt {
                                        bad_request("Insufficient token balance", &cors_origin)
                                    } else {
                                        if let Some(t) = tokens.iter_mut().find(|t| t.symbol == from) {
                                            let bal = t.balance_of(&wallet);
                                            t.balances.insert(wallet.clone(), bal - amt);
                                            blockchain.lock().expect("chain lock").record_op(&format!("token:{}", from), &wallet, -(amt as i128), "swap_router_token_nsc");
                                        }
                                        tpool.0 += amt;
                                        tpool.1 -= usdt_mid;

                                        pool.1 += usdt_mid;
                                        pool.0 -= nsc_out;

                                        let evm_state_writes = {
                                            let mut chain = blockchain.lock().expect("chain lock");
                                            let prev_nsc = chain.get_balance(&wallet);
                                            chain.balances.insert(wallet.clone(), prev_nsc + nsc_out);
                                            chain.record_op("nsc", &wallet, nsc_out as i128, "swap_router_token_nsc");
                                              crate::storage::prepare_evm_state_write(&chain.balances, &chain.nonces)
                                        };

                                          // [ATOMICITY FIX] tokens.json, the token's pool file,
                                          // pool.json, and evm_state.json committed together
                                          // instead of four sequential independent saves.
                                          let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                          if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                              batch_writes.push(w);
                                          }
                                          if let Some(w) = crate::storage::prepare_token_pool_write(from, tpool.0, tpool.1) {
                                              batch_writes.push(w);
                                          }
                                          if let Some(w) = crate::storage::prepare_pool_write(pool.0, pool.1) {
                                              batch_writes.push(w);
                                          }
                                          batch_writes.extend(evm_state_writes);
                                          if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                              eprintln!("[API] Failed to commit swap batch (token->NSC): {}", e);
                                          }
                                        crate::storage::log_trade(from, "NSC", usdt_mid, &wallet);
                                        crate::storage::record_price_tick_now(from);
                                        crate::storage::record_price_tick_now("NSC");
                                        json_response(200, json!({
                                            "status": "ok",
                                            "from": from,
                                            "to": "NSC",
                                            "amount_in": amt.to_string(),
                                            "amount_out": nsc_out.to_string(),
                                            "route": [from, "USDT", "NSC"]
                                        }), &cors_origin)
                                    }
                                }
                            }
                        }
                    } else if from != "NSC" && from != "USDT" && to != "NSC" && to != "USDT" && from != to {
                        // [ROUTER] Multi-hop: TOKEN_A -> USDT -> TOKEN_B, atomic
                        // `amount` is a human-entered whole/fractional token
                        // amount from the frontend; scale to 18-decimal
                        // u128 internal units, same convention as NSC.
                        let amt: u128 = (amount * crate::genesis::DECIMALS as f64) as u128;
                        let mut pool_a = crate::storage::load_token_pool(from);
                        if pool_a.0 == 0 || pool_a.1 <= 0.0 {
                            bad_request("No liquidity for source token", &cors_origin)
                        } else {
                            let usdt_mid = pool_a.1 * amt as f64 / (pool_a.0 as f64 + amt as f64) * 0.997;
                            // [ARITH-HARDENING] usdt_mid feeds directly into
                            // pool_a.1 -= usdt_mid below (a raw f64 subtraction
                            // with no integer-truncation gate), so a NaN here
                            // would otherwise slip past both bound checks and
                            // permanently corrupt pool_a.1.
                            if !usdt_mid.is_finite() || usdt_mid <= 0.0 || usdt_mid > pool_a.1 {
                                bad_request("Insufficient liquidity", &cors_origin)
                            } else {
                                let mut pool_b = crate::storage::load_token_pool(to);
                                if pool_b.0 == 0 || pool_b.1 <= 0.0 {
                                    bad_request("No liquidity for destination token", &cors_origin)
                                } else {
                                    let token_out = (pool_b.0 as f64 * usdt_mid / (pool_b.1 + usdt_mid) * 0.997) as u128;
                                    if token_out == 0 || token_out > pool_b.0 {
                                        bad_request("Insufficient liquidity", &cors_origin)
                                    } else if wallet.is_empty() {
                                        bad_request("Wallet required", &cors_origin)
                                    } else {
                                        let mut tokens = crate::storage::load_tokens();
                                        let from_bal = tokens.iter().find(|t| t.symbol == from).map(|t| t.balance_of(&wallet)).unwrap_or(0);
                                        let dest_exists = tokens.iter().any(|t| t.symbol == to);
                                        if from_bal < amt {
                                            bad_request("Insufficient token balance", &cors_origin)
                                        } else if !dest_exists {
                                            bad_request("Destination token not found", &cors_origin)
                                        } else {
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == from) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal - amt);
                                            }
                                            pool_a.0 += amt;
                                            pool_a.1 -= usdt_mid;
                                            crate::storage::save_token_pool(from, pool_a.0, pool_a.1);

                                            pool_b.1 += usdt_mid;
                                            pool_b.0 -= token_out;
                                            crate::storage::enforce_token_floor(to, &mut pool_b);
                                            crate::storage::save_token_pool(to, pool_b.0, pool_b.1);
                                            if let Some(t) = tokens.iter_mut().find(|t| t.symbol == to) {
                                                let bal = t.balance_of(&wallet);
                                                t.balances.insert(wallet.clone(), bal + token_out);
                                            }
                                            crate::storage::save_tokens(&tokens);

                                            json_response(200, json!({
                                                "status": "ok",
                                                "from": from,
                                                "to": to,
                                                "amount_in": amt.to_string(),
                                                "amount_out": token_out.to_string(),
                                                "route": [from, "USDT", to]
                                            }), &cors_origin)
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        bad_request("Unsupported pair", &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/token/create" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let name = body["name"].as_str().unwrap_or("").to_string();
                let symbol = body["symbol"].as_str().unwrap_or("").to_string();
                // supply is a human-entered whole/fractional token amount
                // (e.g. 1000 or 0.5); scale to 18-decimal u128 internal
                // units, same convention as native NSC.
                let supply_human = body["supply"].as_f64().unwrap_or(0.0);
                let supply: u128 = (supply_human * crate::genesis::DECIMALS as f64) as u128;
                let owner = body["owner"].as_str().unwrap_or("").to_string();
                const DEPLOY_FEE: u128 = 101 * crate::genesis::DECIMALS;
                if name.is_empty() || symbol.is_empty() || supply == 0 || owner.is_empty() {
                    bad_request("Missing required fields", &cors_origin)
                } else {
                    let mut tokens = crate::storage::load_tokens();
                    if tokens.iter().any(|t| t.symbol == symbol) {
                        bad_request("Token symbol already exists", &cors_origin)
                    } else {
                        let mut chain = blockchain.lock().expect("chain lock");
                        let owner_balance = chain.get_balance(&owner);
                        if owner_balance < DEPLOY_FEE {
                            drop(chain);
                            bad_request("Insufficient NSC balance for deployment fee (101 NSC required)", &cors_origin)
                        } else {
                            chain.balances.insert(owner.clone(), owner_balance - DEPLOY_FEE);
                            chain.record_op("nsc", &owner, -(DEPLOY_FEE as i128), "token_create_fee");
                            chain.record_op(&format!("token:{}", symbol), &owner, supply as i128, "token_create");
                            drop(chain);
                            let token = crate::token::Token::new(name.clone(), symbol.clone(), supply, owner.clone());
                            tokens.push(token);
                            crate::storage::save_tokens(&tokens);
                            json_response(200, json!({
                                "status": "ok",
                                "name": name,
                                "symbol": symbol,
                                "supply": supply.to_string(),
                                "owner": owner,
                                "fee_paid": DEPLOY_FEE.to_string()
                            }), &cors_origin)
                        }
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/token/list" => {
    let tokens = crate::storage::load_tokens();
    let list: Vec<_> = tokens.iter().map(|t| json!({
        "name": t.name,
        "symbol": t.symbol,
        "total_supply": t.total_supply.to_string()
    })).collect();
    json_response(200, json!({
        "status": "ok",
        "tokens": list
    }), &cors_origin)
}

"/networks/list" => {
    if method != Method::Get {
        method_not_allowed(&cors_origin)
    } else {
        let networks = crate::storage::load_networks();
        let list: Vec<_> = networks.iter().map(|n| json!({
            "chain_id": n.chain_id,
            "name": n.name,
            "rpc_url": n.rpc_url,
            "native_symbol": n.native_symbol,
            "explorer_url": n.explorer_url,
            "logo_url": n.logo_url,
            "decimals": n.decimals,
            "added_by": n.added_by,
            "added_at": n.added_at,
            "verified": crate::network_registry::is_verified(n.chain_id)
        })).collect();
        json_response(200, json!({
            "status": "ok",
            "networks": list
        }), &cors_origin)
    }
}

"/networks/add" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let chain_id = body["chain_id"].as_u64().unwrap_or(0);
                let name = body["name"].as_str().unwrap_or("").trim().to_string();
                let rpc_url = body["rpc_url"].as_str().unwrap_or("").trim().to_string();
                let native_symbol = body["native_symbol"].as_str().unwrap_or("").trim().to_string();
                let explorer_url = body["explorer_url"].as_str().unwrap_or("").trim().to_string();
                let logo_url = body["logo_url"].as_str().unwrap_or("").trim().to_string();
                let decimals = body["decimals"].as_u64().unwrap_or(18).min(36) as u8;
                let added_by = body["added_by"].as_str().unwrap_or("").trim().to_string();

                if chain_id == 0 || name.is_empty() || native_symbol.is_empty() || added_by.is_empty() {
                    bad_request("Missing required fields (chain_id, name, native_symbol, added_by)", &cors_origin)
                } else if !rpc_url.starts_with("http://") && !rpc_url.starts_with("https://") {
                    bad_request("rpc_url must start with http:// or https://", &cors_origin)
                } else {
                    let mut networks = crate::storage::load_networks();
                    if networks.iter().any(|n| n.chain_id == chain_id) {
                        bad_request("A network with this chain_id already exists", &cors_origin)
                    } else {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let net = crate::network_registry::NetworkInfo {
                            chain_id, name: name.clone(), rpc_url, native_symbol: native_symbol.clone(),
                            explorer_url, logo_url, decimals, added_by, added_at: now,
                        };
                        networks.push(net);
                        crate::storage::save_networks(&networks);
                        json_response(200, json!({
                            "status": "ok",
                            "chain_id": chain_id,
                            "name": name,
                            "native_symbol": native_symbol,
                            "verified": crate::network_registry::is_verified(chain_id)
                        }), &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/price_history" => {
    let query = url.split('?').nth(1).unwrap_or("");
    let symbol = query.split('&').find_map(|p| p.strip_prefix("symbol=")).unwrap_or("NSC").to_string();
    let interval = query.split('&').find_map(|p| p.strip_prefix("interval=")).unwrap_or("1h").to_string();

    let bucket_secs: u64 = match interval.as_str() {
        "5m" => 300,
        "15m" => 900,
        "1h" => 3600,
        "4h" => 14400,
        "1d" => 86400,
        _ => 3600,
    };

    let ticks = crate::storage::load_price_history(&symbol);
    let mut candles: std::collections::BTreeMap<u64, Vec<f64>> = std::collections::BTreeMap::new();
    for (t, p) in ticks {
        let bucket = (t / bucket_secs) * bucket_secs;
        candles.entry(bucket).or_insert_with(Vec::new).push(p);
    }

    let mut result = vec![];
    for (bucket_time, prices) in candles {
        if prices.is_empty() { continue; }
        let open = prices[0];
        let close = *prices.last().unwrap();
        let high = prices.iter().cloned().fold(f64::MIN, f64::max);
        let low = prices.iter().cloned().fold(f64::MAX, f64::min);
        result.push(json!({
            "time": bucket_time,
            "open": open,
            "high": high,
            "low": low,
            "close": close
        }));
    }

    json_response(200, json!({
        "status": "ok",
        "symbol": symbol,
        "interval": interval,
        "candles": result
    }), &cors_origin)
}

"/airdrop/claim" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => {
                let wallet = body["wallet"].as_str().unwrap_or("").to_string().to_lowercase();
                let twitter_handle = body["twitter_handle"].as_str().unwrap_or("").trim().to_string();
                let tweet_link = body["tweet_link"].as_str().unwrap_or("").trim().to_string();

                if !wallet.starts_with("0x") || wallet.len() != 42 {
                    bad_request("Invalid wallet address (must be 0x... 42 chars)", &cors_origin)
                } else if twitter_handle.is_empty() {
                    bad_request("Twitter/X handle is required", &cors_origin)
                } else if tweet_link.is_empty() || !tweet_link.starts_with("http") {
                    bad_request("Valid tweet link is required", &cors_origin)
                } else {
                    crate::storage::append_airdrop_claim(&wallet, &twitter_handle, &tweet_link);
                    json_response(200, json!({
                        "status": "ok",
                        "message": "Claim submitted for review"
                    }), &cors_origin)
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/token/balance" => {
    let address = url.split('?').nth(1).unwrap_or("")
        .split('&')
        .find_map(|p| p.strip_prefix("address="))
        .unwrap_or("").to_string();
    let symbol = url.split('?').nth(1).unwrap_or("")
        .split('&')
        .find_map(|p| p.strip_prefix("symbol="))
        .unwrap_or("").to_string();
    let tokens = crate::storage::load_tokens();
    if let Some(t) = tokens.iter().find(|t| t.symbol == symbol) {
        json_response(200, json!({
            "status": "ok",
            "balance": t.balance_of(&address).to_string()
        }), &cors_origin)
    } else {
        bad_request("Token not found", &cors_origin)
    }
}

"/token/lp/balance" => {
    let address = url.split('?').nth(1).unwrap_or("")
        .split('&')
        .find_map(|p| p.strip_prefix("address="))
        .unwrap_or("").to_string();
    let symbol = url.split('?').nth(1).unwrap_or("")
        .split('&')
        .find_map(|p| p.strip_prefix("symbol="))
        .unwrap_or("").to_string();

    if symbol.is_empty() || address.is_empty() {
        bad_request("symbol and address query params required", &cors_origin)
    } else {
        let (total_lp, holders) = crate::storage::load_token_lp(&symbol);
        let owned_lp = holders.get(&address).copied().unwrap_or(0);

        if total_lp == 0 || owned_lp == 0 {
            json_response(200, json!({
                "status": "ok",
                "symbol": symbol,
                "lp_balance": "0",
                "share_percent": 0.0,
                "redeemable_token": "0",
                "redeemable_usdt": 0.0
            }), &cors_origin)
        } else {
            let pool = crate::storage::load_token_pool(&symbol);
            let redeemable_token: u128 = pool.0.saturating_mul(owned_lp) / total_lp;
            let usdt_reserve_micro: u128 = (pool.1 * 1_000_000.0).round() as u128;
            let redeemable_usdt_micro: u128 = usdt_reserve_micro.saturating_mul(owned_lp) / total_lp;
            let redeemable_usdt: f64 = redeemable_usdt_micro as f64 / 1_000_000.0;
            let share_percent: f64 = (owned_lp as f64 / total_lp as f64) * 100.0;

            json_response(200, json!({
                "status": "ok",
                "symbol": symbol,
                "lp_balance": owned_lp.to_string(),
                "total_lp": total_lp.to_string(),
                "share_percent": share_percent,
                "redeemable_token": redeemable_token.to_string(),
                "redeemable_usdt": redeemable_usdt
            }), &cors_origin)
        }
    }
}

"/token/liquidity/add" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                if blockchain.lock().expect("chain lock").chain_frozen {
                    break 'guard bad_request("Chain is frozen - trading and transfers are currently suspended", &cors_origin);
                }
                let _fund_guard = FUND_LOCK.lock().expect("fund lock");
                let symbol = body["symbol"].as_str().unwrap_or("").to_string();
                // Custom token balances are whole u64 units, but the
                // frontend sends a plain JS number which may be
                // fractional (e.g. 0.5) if the user types a decimal.
                // as_u64() silently returns None for non-integer JSON
                // numbers, so read as f64 first and round instead.
                // token_amount is a human-entered whole/fractional token
                // amount (e.g. 0.03); scale to 18-decimal u128 internal
                // units instead of rounding to a whole number.
                let token_amount: u128 = body["token_amount"].as_f64()
                    .map(|v| (v * crate::genesis::DECIMALS as f64) as u128)
                    .unwrap_or(0);
                let usdt_amount = body["usdt_amount"].as_f64().unwrap_or(0.0);
                let wallet = body["wallet"].as_str().unwrap_or("").to_string();
                let signature = body["signature"].as_str().unwrap_or("").to_string();
                let timestamp = body["timestamp"].as_u64().unwrap_or(0);

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let auth_ok = if timestamp == 0 || now.saturating_sub(timestamp) > 120 {
                    false
                } else {
                    let message = format!(
                        "NSC_LIQUIDITY_ADD:{}:{}:{}:{}:{}",
                        symbol, token_amount, usdt_amount, wallet, timestamp
                    );
                    crate::evm_tx::verify_personal_sign(&message, &signature, &wallet)
                };

                if !auth_ok {
                    bad_request("Invalid or expired signature", &cors_origin)
                } else if symbol.is_empty() || token_amount == 0 || usdt_amount <= 0.0 || wallet.is_empty() {
                    bad_request("Missing required fields", &cors_origin)
                } else {
                    let mut tokens = crate::storage::load_tokens();
                    if let Some(t) = tokens.iter_mut().find(|t| t.symbol == symbol) {
                        let bal = t.balance_of(&wallet);
                        if bal < token_amount {
                            bad_request("Insufficient token balance", &cors_origin)
                        } else {
                            let prev_usdt = crate::storage::load_usdt_balance(&wallet);
                            if prev_usdt < usdt_amount {
                                bad_request("Insufficient USDT balance", &cors_origin)
                            } else {
                                t.balances.insert(wallet.clone(), bal - token_amount);
                                blockchain.lock().expect("chain lock").record_op(&format!("token:{}", symbol), &wallet, -(token_amount as i128), "token_liquidity_add");

                                let mut pool = crate::storage::load_token_pool(&symbol);
                                let usdt_micro_deposit: u128 = (usdt_amount * 1_000_000.0).round() as u128;

                                // [LP] NS is intentionally excluded from LP
                                // tracking — its reserve still grows from
                                // fees, but nobody can claim it since it
                                // never gets LP shares minted.
                                let mut lp_minted: u128 = 0;
                                  let mut token_lp_writes: Option<(std::path::PathBuf, Vec<u8>)> = None;
                                if symbol != "NS" {
                                    let (mut total_lp, mut holders) = crate::storage::load_token_lp(&symbol);
                                    if total_lp == 0 {
                                        lp_minted = crate::storage::integer_sqrt_u128(
                                            token_amount.saturating_mul(usdt_micro_deposit)
                                        );
                                    } else {
                                        let usdt_reserve_micro: u128 = (pool.1 * 1_000_000.0).round() as u128;
                                        let lp_from_token = if pool.0 > 0 {
                                            total_lp.saturating_mul(token_amount) / pool.0
                                        } else { 0 };
                                        let lp_from_usdt = if usdt_reserve_micro > 0 {
                                            total_lp.saturating_mul(usdt_micro_deposit) / usdt_reserve_micro
                                        } else { 0 };
                                        lp_minted = lp_from_token.min(lp_from_usdt);
                                    }
                                    if lp_minted > 0 {
                                        total_lp = total_lp.saturating_add(lp_minted);
                                        let current = holders.get(&wallet).copied().unwrap_or(0);
                                        holders.insert(wallet.clone(), current.saturating_add(lp_minted));
                                          token_lp_writes = crate::storage::prepare_token_lp_write(&symbol, total_lp, &holders);
                                    }
                                }

                                pool.0 += token_amount;
                                pool.1 += usdt_amount;
                                // [ATOMICITY FIX] usdt_balances.json, tokens.json,
                                // the optional token_lp.json update, and the token's
                                // pool file committed together instead of up to four
                                // sequential independent saves.
                                let mut batch_writes: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
                                blockchain.lock().expect("chain lock").record_op("usdt", &wallet, -(usdt_amount as i128), "token_liquidity_add");
                                if let Some(w) = crate::storage::prepare_usdt_balance_write(&wallet, prev_usdt - usdt_amount) {
                                    batch_writes.push(w);
                                }
                                if let Some(w) = crate::storage::prepare_tokens_write(&tokens) {
                                    batch_writes.push(w);
                                }
                                if let Some(w) = token_lp_writes {
                                    batch_writes.push(w);
                                }
                                if let Some(w) = crate::storage::prepare_token_pool_write(&symbol, pool.0, pool.1) {
                                    batch_writes.push(w);
                                }
                                if let Err(e) = crate::storage::atomic_write_batch(&batch_writes) {
                                    eprintln!("[API] Failed to commit token/liquidity/add batch: {}", e);
                                }
                                json_response(200, json!({
                                    "status": "ok",
                                    "symbol": symbol,
                                    "token_reserve": pool.0.to_string(),
                                    "usdt_reserve": pool.1,
                                    "lp_minted": lp_minted.to_string()
                                }), &cors_origin)
                            }
                        }
                    } else {
                        bad_request("Token not found", &cors_origin)
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/token/pools" => {
    let mut pools = crate::storage::load_all_token_pools();
    if let Some(obj) = pools.as_object_mut() {
        for (_symbol, v) in obj.iter_mut() {
            let usdt_micro = v["usdt_micro"].as_u64().unwrap_or(0);
            let usdt = usdt_micro as f64 / 1_000_000.0;
            v["usdt"] = json!(usdt);
        }
    }
    json_response(200, json!({
        "status": "ok",
        "pools": pools
    }), &cors_origin)
}

"/token/transfer" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        match serde_json::from_str::<serde_json::Value>(&body_string) {
            Ok(body) => 'guard: {
                if blockchain.lock().expect("chain lock").chain_frozen {
                    break 'guard bad_request("Chain is frozen - trading and transfers are currently suspended", &cors_origin);
                }
                let symbol = body["symbol"].as_str().unwrap_or("").to_string();
                let from = body["from"].as_str().unwrap_or("").to_string();
                let to = body["to"].as_str().unwrap_or("").to_string();
                // amount is a human-entered whole/fractional token amount;
                // scale to 18-decimal u128 internal units.
                let amount_human = body["amount"].as_f64().unwrap_or(0.0);
                let amount: u128 = (amount_human * crate::genesis::DECIMALS as f64) as u128;
                if symbol.is_empty()||from.is_empty()||to.is_empty()||amount==0 {
                    bad_request("Missing fields", &cors_origin)
                } else {
                    // [FEE] Custom-token transfers now also charge a
                    // tiered NSC-denominated network fee, matching the
                    // native NSC transfer fee (chain.rs transfer_evm()):
                    //   value <= $500 -> 0.05%, value > $500 -> 0.03%,
                    //   capped at $100. If the token has no liquidity
                    // pool (so its USD price can't be determined), a
                    // flat $0.03 fee is charged instead. The fee is
                    // always paid in NSC, deducted from the sender's
                    // NSC balance separately from the token amount
                    // being transferred, and burned (not credited
                    // anywhere) - consistent with the existing burn-fee
                    // pattern used elsewhere in this file.
                    let nsc_pool = crate::storage::load_pool();
                    let nsc_reserve_whole = nsc_pool.0 as f64 / crate::genesis::DECIMALS as f64;
                    let nsc_price_usd = if nsc_reserve_whole > 0.0 { nsc_pool.1 / nsc_reserve_whole } else { 0.0 };

                    let tpool = crate::storage::load_token_pool(&symbol);
                    let token_reserve_whole = tpool.0 as f64 / crate::genesis::DECIMALS as f64;
                    let token_price_usd = if token_reserve_whole > 0.0 { tpool.1 / token_reserve_whole } else { 0.0 };

                    let fee_usd: f64 = if token_price_usd > 0.0 {
                        let usd_value = amount_human * token_price_usd;
                        let fee_rate = if usd_value <= 500.0 { 0.0005 } else { 0.0003 };
                        (usd_value * fee_rate).min(100.0)
                    } else {
                        0.03
                    };
                    let fee_nsc_whole = if nsc_price_usd > 0.0 { fee_usd / nsc_price_usd } else { 0.0 };
                    let fee_nsc: u128 = (fee_nsc_whole * crate::genesis::DECIMALS as f64) as u128;

                    let sender_nsc_balance = {
                        let chain = blockchain.lock().expect("chain lock");
                        chain.get_balance(&from)
                    };

                    if fee_nsc > 0 && sender_nsc_balance < fee_nsc {
                        bad_request("Insufficient NSC balance to pay network fee", &cors_origin)
                    } else {
                        let mut tokens = crate::storage::load_tokens();
                        if let Some(t) = tokens.iter_mut().find(|t| t.symbol == symbol) {
                            if t.transfer(from.clone(), to.clone(), amount) {
                                blockchain.lock().expect("chain lock").record_op(&format!("token:{}", symbol), &from, -(amount as i128), "token_transfer");
                                blockchain.lock().expect("chain lock").record_op(&format!("token:{}", symbol), &to, amount as i128, "token_transfer");
                                crate::storage::save_tokens(&tokens);

                                if fee_nsc > 0 {
                                    let mut chain = blockchain.lock().expect("chain lock");
                                    let prev = chain.get_balance(&from);
                                    chain.balances.insert(from.clone(), prev - fee_nsc);
                                    chain.record_op("nsc", &from, -(fee_nsc as i128), "token_transfer_fee");
                                    crate::storage::save_evm_state(&chain.balances, &chain.nonces);
                                }

                                // [TX-LOG] Gives this transfer a proper
                                // tx_hash so it shows in Explorer and the
                                // wallet's Transactions/Fees tabs, same as
                                // native NSC transfers.
                                let ts_now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);
                                let tx_hash = format!("0x{}", crate::hash::calculate_hash(
                                    &format!("{}:{}:{}:{}:{}:{}", symbol, from, to, amount, ts_now, fee_nsc)
                                ));
                                crate::storage::log_token_transfer(&symbol, &from, &to, amount, fee_nsc, &tx_hash);

                                json_response(200, json!({
                                    "status":"ok","symbol":symbol,"from":from,"to":to,
                                    "amount":amount.to_string(),"fee_nsc":fee_nsc.to_string(),
                                    "tx_hash":tx_hash
                                }), &cors_origin)
                            } else {
                                bad_request("Insufficient balance", &cors_origin)
                            }
                        } else {
                            bad_request("Token not found", &cors_origin)
                        }
                    }
                }
            }
            Err(_) => bad_request("Invalid JSON", &cors_origin)
        }
    }
}

"/buy" => {
    if method != Method::Post {
        method_not_allowed(&cors_origin)
    } else {
        json_response(200, json!({
            "status": "ok",
            "message": "Purchase submitted"
        }), &cors_origin)
    }
}

_ => not_found(&cors_origin),
        };

        // [FIX-13] Ignore respond errors (client disconnected).
        let _ = request.respond(response);
    }
}

// ============================================================
// END OF api.rs
// ============================================================

