// bsc_watcher.rs — Phase A: read-only BSC balance detector via Moralis Web3 Data API.
// Snapshot-based: each poll cycle fetches CURRENT token/native balances per known
// user address and sets (not increments) them in the token registry.
// Does NOT touch custody, native NSC balances, or send logic — display/detection only.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use serde::Deserialize;
use serde_json::Value;

use crate::storage;
use crate::token_registry::{TokenRegistry, DetectedToken};
use crate::coingecko_client;
use crate::coinmarketcap_client;

const CHAIN_ID: u64 = 56; // BSC
const MORALIS_CHAIN_PARAM: &str = "bsc";
const POLL_INTERVAL_SECS: u64 = 14400; // 4h — reduced from 10min to stay within Moralis free-tier 40,000 CU/day budget (~480 req/day at 84 req/cycle)
const PER_USER_DELAY_MS: u64 = 300;  // gentle pacing between users

#[derive(Debug, Deserialize)]
struct MoralisNativeBalance {
    balance: String,
}

#[derive(Debug, Deserialize)]
struct MoralisErc20Entry {
    token_address: String,
    symbol: Option<String>,
    name: Option<String>,
    logo: Option<String>,
    decimals: Option<Value>, // Moralis sometimes returns string, sometimes number
    balance: String,
    possible_spam: Option<bool>,
}

fn parse_decimals(v: &Option<Value>) -> u8 {
    match v {
        Some(Value::String(s)) => s.parse().unwrap_or(18),
        Some(Value::Number(n)) => n.as_u64().unwrap_or(18) as u8,
        _ => 18,
    }
}

fn moralis_api_key() -> Option<String> {
    std::env::var("MORALIS_API_KEY").ok().filter(|k| !k.is_empty())
}

async fn fetch_native_balance(client: &reqwest::Client, api_key: &str, address: &str) -> Option<u128> {
    let url = format!(
        "https://deep-index.moralis.io/api/v2.2/{}/balance?chain={}",
        address, MORALIS_CHAIN_PARAM
    );
    let resp = match client.get(&url).header("X-API-Key", api_key).send().await {
        Ok(r) => r,
        Err(e) => {
            println!("[NATIVE-DEBUG] request failed addr={} err={}", address, e);
            return None;
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        println!("[NATIVE-DEBUG] non-success addr={} status={} body={}", address, status, body);
        return None;
    }
    let body_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            println!("[NATIVE-DEBUG] read body failed addr={} err={}", address, e);
            return None;
        }
    };
    let parsed: MoralisNativeBalance = match serde_json::from_str(&body_text) {
        Ok(p) => p,
        Err(e) => {
            println!("[NATIVE-DEBUG] parse failed addr={} err={} body={}", address, e, body_text);
            return None;
        }
    };
    match parsed.balance.parse::<u128>() {
        Ok(b) => Some(b),
        Err(e) => {
            println!("[NATIVE-DEBUG] balance parse failed addr={} raw={} err={}", address, parsed.balance, e);
            None
        }
    }
}

async fn fetch_erc20_balances(client: &reqwest::Client, api_key: &str, address: &str) -> Option<Vec<MoralisErc20Entry>> {
    let url = format!(
        "https://deep-index.moralis.io/api/v2.2/{}/erc20?chain={}",
        address, MORALIS_CHAIN_PARAM
    );
    let resp = client.get(&url).header("X-API-Key", api_key).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<Vec<MoralisErc20Entry>>().await.ok()
}

/// Resolves token metadata: prefers CoinGecko (verified=true) for the "verified" badge
/// per user's decision (auto-approve if CoinGecko-listed), falls back to Moralis-provided
/// name/symbol/decimals marked unverified otherwise. Skips tokens Moralis flags as spam.
const RECHECK_INTERVAL_SECS: u64 = 21600; // 6h cooldown before re-querying CoinGecko for a still-unverified token

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn resolve_token(
    registry: &Arc<TokenRegistry>,
    contract_address: &str,
    moralis_symbol: &str,
    moralis_name: &str,
    moralis_decimals: u8,
) -> DetectedToken {
    // [2026-08-17] Hardcoded pin: official Binance-Peg BSC-USD (USDT)
    // contract. This is the canonical, universally-used USDT contract
    // on BSC (same one PancakeSwap etc. use) — not a scam/vanity
    // token, so it shouldn't sit in "unverified" limbo waiting on a
    // third-party API lookup that may be rate-limited. Always marked
    // verified, and if a live price fetch hasn't populated price_usd
    // yet, defaults to $1.00 (safe assumption for a USD-pegged
    // stablecoin — the price-refresh pass will still overwrite this
    // with a live CoinGecko/CMC quote when one succeeds).
    const BSC_USDT_ADDRESS: &str = "0x55d398326f99059ff775485246999027b3197955";
    if contract_address.to_lowercase() == BSC_USDT_ADDRESS {
        let existing_price = registry.get_token(CHAIN_ID, contract_address)
            .map(|t| t.price_usd)
            .filter(|p| *p > 0.0)
            .unwrap_or(1.0);
        let token = DetectedToken {
            chain_id: CHAIN_ID,
            contract_address: BSC_USDT_ADDRESS.to_string(),
            symbol: "USDT".to_string(),
            name: "Tether USD".to_string(),
            logo_url: String::new(),
            decimals: 18,
            verified: true,
            first_seen_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            price_usd: existing_price,
            last_checked_at: now_ts(),
        };
        let _ = registry.insert_token(token.clone());
        return token;
    }
    if let Some(existing) = registry.get_token(CHAIN_ID, contract_address) {
        if existing.verified {
            return existing;
        }
        let elapsed = now_ts().saturating_sub(existing.last_checked_at);
        if elapsed < RECHECK_INTERVAL_SECS {
            return existing;
        }
        // cooldown expired — fall through and retry CoinGecko below
    }
    if let Some(mut found) = coingecko_client::lookup_token_by_contract(CHAIN_ID, contract_address).await {
        found.last_checked_at = now_ts();
        let _ = registry.insert_token(found.clone());
        return found;
    }
    // Not on CoinGecko — mark unverified but keep Moralis-reported symbol/name/decimals
    // (still labeled "unverified" for trust purposes, per user's decision).
    let token = DetectedToken {
        chain_id: CHAIN_ID,
        contract_address: contract_address.to_lowercase(),
        symbol: if moralis_symbol.is_empty() { "UNVERIFIED".to_string() } else { moralis_symbol.to_string() },
        name: if moralis_name.is_empty() { contract_address.to_string() } else { moralis_name.to_string() },
        logo_url: String::new(),
        decimals: moralis_decimals,
        verified: false,
        first_seen_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        price_usd: 0.0,
        last_checked_at: now_ts(),
    };
    let _ = registry.insert_token(token.clone());
    token
}

pub async fn start_bsc_watcher(registry: Arc<TokenRegistry>) {
    let api_key = match moralis_api_key() {
        Some(k) => k,
        None => {
            eprintln!("[BSC-WATCHER] MORALIS_API_KEY not set — watcher disabled.");
            return;
        }
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    println!("[BSC-WATCHER] started (Moralis snapshot mode, chain={})", MORALIS_CHAIN_PARAM);

    loop {
        let user_addresses: HashSet<String> = {
            let (balances, _) = storage::load_evm_state();
            balances.keys().map(|a| a.to_lowercase()).collect()
        };

        for addr in &user_addresses {
            // Native BNB balance
            if let Some(native_amount) = fetch_native_balance(&client, &api_key, addr).await {
                if let Some(native_token) = coingecko_client::native_coin_info(CHAIN_ID) {
                    let _ = registry.insert_token(native_token);
                }
                let _ = registry.set_balance(addr, CHAIN_ID, "", native_amount);
            }

            // ERC20 token balances
            if let Some(entries) = fetch_erc20_balances(&client, &api_key, addr).await {
                for entry in entries {
                    if entry.possible_spam.unwrap_or(false) {
                        continue; // skip Moralis-flagged spam tokens entirely
                    }
                    let amount: u128 = entry.balance.parse().unwrap_or(0);
                    if amount == 0 {
                        continue;
                    }
                    let contract = entry.token_address.to_lowercase();
                    let decimals = parse_decimals(&entry.decimals);
                    let token = resolve_token(
                        &registry,
                        &contract,
                        entry.symbol.as_deref().unwrap_or(""),
                        entry.name.as_deref().unwrap_or(""),
                        decimals,
                    ).await;
                    let _ = registry.set_balance(addr, CHAIN_ID, &contract, amount);
                    println!("[BSC-WATCHER] {} = {} {} (verified={})", addr, amount, token.symbol, token.verified);
                }
            }

            tokio::time::sleep(Duration::from_millis(PER_USER_DELAY_MS)).await;
        }

        // ── Price refresh pass (once per distinct token per cycle). ──
        // [2026-08-17] Previously skipped unverified tokens entirely
        // (they'd never even attempt a CoinGecko lookup). Now: try
        // CoinGecko for every token regardless of verified status —
        // "verified" only means "CoinGecko had metadata for it", not
        // "has a price", so the two were needlessly conflated. If
        // CoinGecko has no listing, fall back to CoinMarketCap (see
        // coinmarketcap_client.rs for why a symbol+platform-filtered
        // map lookup is used instead of a direct address lookup).
        // Many small/vanity tokens will still legitimately have no
        // price anywhere — that's an honest $0.00, not a bug.
        let all_tokens = registry.list_all_tokens();
        for token in &all_tokens {
            let price = if token.contract_address.is_empty() {
                coingecko_client::fetch_native_price_usd(token.chain_id).await
            } else {
                let cg_price = coingecko_client::fetch_token_price_usd(token.chain_id, &token.contract_address).await;
                if cg_price.is_some() {
                    cg_price
                } else {
                    coinmarketcap_client::fetch_token_price_usd(token.chain_id, &token.contract_address, &token.symbol).await
                }
            };
            if let Some(p) = price {
                let _ = registry.update_token_price(token.chain_id, &token.contract_address, p);
            }
            tokio::time::sleep(Duration::from_millis(6000)).await;
        }

        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}
