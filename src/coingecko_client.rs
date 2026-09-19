// coingecko_client.rs — contract-address lookup for auto-detected tokens (Phase A, read-only)
use serde::Deserialize;
use crate::token_registry::DetectedToken;

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Maps our internal chain_id to CoinGecko's "platform" slug
pub fn chain_id_to_cg_platform(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 => Some("ethereum"),
        56 => Some("binance-smart-chain"),
        137 => Some("polygon-pos"),
        42161 => Some("arbitrum-one"),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct CgContractResponse {
    id: String,
    symbol: String,
    name: String,
    image: Option<CgImage>,
    detail_platforms: Option<std::collections::HashMap<String, CgPlatformDetail>>,
}

#[derive(Debug, Deserialize)]
struct CgImage {
    large: Option<String>,
    small: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CgPlatformDetail {
    decimal_place: Option<u8>,
}

/// Looks up a token by contract address on CoinGecko.
/// Returns None on any failure (network error, not found, rate limit, parse error) —
/// caller should fall back to get_or_insert_unverified().
pub async fn lookup_token_by_contract(chain_id: u64, contract_address: &str) -> Option<DetectedToken> {
    let platform = chain_id_to_cg_platform(chain_id)?;
    let addr = contract_address.to_lowercase();
    let url = format!(
        "https://api.coingecko.com/api/v3/coins/{}/contract/{}",
        platform, addr
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None; // 404 = not listed, 429 = rate limited — both fall back to unverified
    }

    let parsed: CgContractResponse = resp.json().await.ok()?;

    let logo_url = parsed
        .image
        .as_ref()
        .and_then(|i| i.large.clone().or_else(|| i.small.clone()))
        .unwrap_or_default();

    let decimals = parsed
        .detail_platforms
        .as_ref()
        .and_then(|m| m.get(platform))
        .and_then(|p| p.decimal_place)
        .unwrap_or(18);

    Some(DetectedToken {
        chain_id,
        contract_address: addr,
        symbol: parsed.symbol.to_uppercase(),
        name: parsed.name,
        logo_url,
        decimals,
        verified: true,
        first_seen_at: now_ts(),
        last_checked_at: now_ts(),
        price_usd: 0.0,
    })
}

/// Known native coins per chain — no API call needed, avoids burning rate limit.
pub fn native_coin_info(chain_id: u64) -> Option<DetectedToken> {
    let (symbol, name, logo) = match chain_id {
        1 => ("ETH", "Ethereum", "https://assets.coingecko.com/coins/images/279/large/ethereum.png"),
        56 => ("BNB", "BNB", "https://assets.coingecko.com/coins/images/825/large/bnb-icon2_2x.png"),
        137 => ("MATIC", "Polygon", "https://assets.coingecko.com/coins/images/4713/large/polygon.png"),
        42161 => ("ETH", "Ethereum (Arbitrum)", "https://assets.coingecko.com/coins/images/279/large/ethereum.png"),
        _ => return None,
    };
    Some(DetectedToken {
        chain_id,
        contract_address: "".to_string(),
        symbol: symbol.to_string(),
        name: name.to_string(),
        logo_url: logo.to_string(),
        decimals: 18,
        verified: true,
        first_seen_at: now_ts(),
        last_checked_at: now_ts(),
        price_usd: 0.0,
    })
}

/// Maps our internal chain_id to CoinGecko's native-coin id (for /simple/price).
pub fn chain_id_to_cg_native_id(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 => Some("ethereum"),
        56 => Some("binancecoin"),
        137 => Some("matic-network"),
        42161 => Some("ethereum"),
        _ => None,
    }
}

/// Fetches the current USD price of a chain's native coin (e.g. BNB on BSC).
/// Returns None on any failure — caller should leave the cached price unchanged.
pub async fn fetch_native_price_usd(chain_id: u64) -> Option<f64> {
    let cg_id = chain_id_to_cg_native_id(chain_id)?;
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
        cg_id
    );
    let client = match reqwest::Client::builder()
        .user_agent("nusacoin-price-fetcher/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build() {
        Ok(c) => c,
        Err(e) => { eprintln!("[PRICE-DEBUG] client build failed: {}", e); return None; }
    };
    let resp = match client.get(&url).header("Accept", "application/json").send().await {
        Ok(r) => r,
        Err(e) => { eprintln!("[PRICE-DEBUG] native request error ({}): {}", cg_id, e); return None; }
    };
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        eprintln!("[PRICE-DEBUG] native non-success status={} id={} body={}", status, cg_id, body);
        return None;
    }
    let raw = match resp.text().await {
        Ok(t) => t,
        Err(e) => { eprintln!("[PRICE-DEBUG] native body read error: {}", e); return None; }
    };
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => { eprintln!("[PRICE-DEBUG] native JSON parse error: {} raw={}", e, raw); return None; }
    };
    let price = parsed[cg_id]["usd"].as_f64();
    if price.is_none() {
        eprintln!("[PRICE-DEBUG] native price field missing id={} raw={}", cg_id, raw);
    }
    price
}

/// Fetches the current USD price of an ERC20 token by contract address.
/// Returns None on any failure (not listed, rate limited, network error) —
/// caller should leave the cached price unchanged rather than zero it out.
pub async fn fetch_token_price_usd(chain_id: u64, contract_address: &str) -> Option<f64> {
    let platform = chain_id_to_cg_platform(chain_id)?;
    let addr = contract_address.to_lowercase();
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/token_price/{}?contract_addresses={}&vs_currencies=usd",
        platform, addr
    );
    let client = match reqwest::Client::builder()
        .user_agent("nusacoin-price-fetcher/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build() {
        Ok(c) => c,
        Err(e) => { eprintln!("[PRICE-DEBUG] client build failed: {}", e); return None; }
    };
    let resp = match client.get(&url).header("Accept", "application/json").send().await {
        Ok(r) => r,
        Err(e) => { eprintln!("[PRICE-DEBUG] token request error ({}): {}", addr, e); return None; }
    };
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        eprintln!("[PRICE-DEBUG] token non-success status={} addr={} body={}", status, addr, body);
        return None;
    }
    let raw = match resp.text().await {
        Ok(t) => t,
        Err(e) => { eprintln!("[PRICE-DEBUG] token body read error: {}", e); return None; }
    };
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => { eprintln!("[PRICE-DEBUG] token JSON parse error: {} raw={}", e, raw); return None; }
    };
    let price = parsed[&addr]["usd"].as_f64();
    if price.is_none() {
        eprintln!("[PRICE-DEBUG] token price field missing addr={} raw={}", addr, raw);
    }
    price
}
