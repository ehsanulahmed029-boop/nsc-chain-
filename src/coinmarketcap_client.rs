// coinmarketcap_client.rs — fallback price source for tokens CoinGecko
// doesn't have. Added 2026-08-17.
//
// Verified-working flow (the documented `/v2/cryptocurrency/info?address=`
// contract lookup returned a 400 "Invalid value for 'slug': 'null'" error
// even for well-known tokens like FLOKI — confirmed broken/misdocumented
// for this account tier, NOT a missing-listing issue). The flow below was
// live-tested and confirmed working on 2026-08-17:
//   1. GET /v1/cryptocurrency/map?symbol={symbol} — returns ALL tokens
//      sharing that symbol across every chain (e.g. 8 different "FLOKI"
//      tokens on different chains/platforms).
//   2. Filter results by platform.slug == "bnb" (BSC) AND
//      token_address matching our contract address, to pick the right one.
//   3. GET /v3/cryptocurrency/quotes/latest?id={id} — get the USD price.
//
// Costs ~2 API credits per token looked up this way (Basic plan: 15,000
// credits/month) — used only as a fallback after CoinGecko returns None,
// not on every refresh cycle for every token, to conserve quota.

use serde::Deserialize;

fn api_key() -> Option<String> {
    std::env::var("CMC_API_KEY").ok().filter(|k| !k.is_empty())
}

#[derive(Debug, Deserialize)]
struct MapPlatform {
    slug: Option<String>,
    token_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MapEntry {
    id: u64,
    platform: Option<MapPlatform>,
}

#[derive(Debug, Deserialize)]
struct MapResponse {
    data: Option<Vec<MapEntry>>,
}

#[derive(Debug, Deserialize)]
struct QuoteUsdEntry {
    symbol: String,
    price: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct QuoteDataEntry {
    quote: Option<Vec<QuoteUsdEntry>>,
}

#[derive(Debug, Deserialize)]
struct QuoteResponse {
    data: Option<Vec<QuoteDataEntry>>,
}

/// Maps our internal chain_id to CMC's platform slug.
/// Only BSC is wired up currently, matching this codebase's current
/// scope (see coingecko_client's chain_id_to_cg_platform for the
/// equivalent broader mapping if more chains are added later).
fn chain_id_to_cmc_platform_slug(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        56 => Some("bnb"),
        _ => None,
    }
}

/// Looks up a token's live USD price on CoinMarketCap by symbol +
/// contract address (see module doc for why this two-step flow is
/// used instead of the documented-but-broken address lookup).
/// Returns None on any failure (no API key, network error, not
/// found, ambiguous match, rate limit, parse error) — caller should
/// treat this the same as "no price available."
pub async fn fetch_token_price_usd(chain_id: u64, contract_address: &str, symbol: &str) -> Option<f64> {
    let key = api_key()?;
    let platform_slug = chain_id_to_cmc_platform_slug(chain_id)?;
    if symbol.is_empty() {
        return None;
    }
    let addr = contract_address.to_lowercase();

    let client = match reqwest::Client::builder()
        .user_agent("nusacoin-price-fetcher/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build() {
        Ok(c) => c,
        Err(e) => { eprintln!("[CMC-DEBUG] client build failed: {}", e); return None; }
    };

    // ── Step 1: resolve CMC id via symbol map, filtered by platform + address ──
    let map_url = format!(
        "https://pro-api.coinmarketcap.com/v1/cryptocurrency/map?symbol={}",
        symbol
    );
    let map_resp = match client.get(&map_url)
        .header("X-CMC_PRO_API_KEY", &key)
        .header("Accept", "application/json")
        .send().await {
        Ok(r) => r,
        Err(e) => { eprintln!("[CMC-DEBUG] map request error ({}): {}", symbol, e); return None; }
    };
    if !map_resp.status().is_success() {
        let status = map_resp.status();
        let body = map_resp.text().await.unwrap_or_default();
        eprintln!("[CMC-DEBUG] map non-success status={} symbol={} body={}", status, symbol, body);
        return None;
    }
    let map_raw = match map_resp.text().await {
        Ok(t) => t,
        Err(e) => { eprintln!("[CMC-DEBUG] map body read error: {}", e); return None; }
    };
    let map_parsed: MapResponse = match serde_json::from_str(&map_raw) {
        Ok(v) => v,
        Err(e) => { eprintln!("[CMC-DEBUG] map JSON parse error: {} raw={}", e, map_raw); return None; }
    };
    let entries = map_parsed.data?;
    let matched = entries.iter().find(|e| {
        e.platform.as_ref().map_or(false, |p| {
            p.slug.as_deref() == Some(platform_slug)
                && p.token_address.as_deref().map(|a| a.to_lowercase()) == Some(addr.clone())
        })
    });
    let cmc_id = match matched {
        Some(e) => e.id,
        None => {
            eprintln!("[CMC-DEBUG] no matching entry for symbol={} addr={} platform={}", symbol, addr, platform_slug);
            return None;
        }
    };

    // ── Step 2: fetch USD quote by id ──
    let quote_url = format!(
        "https://pro-api.coinmarketcap.com/v3/cryptocurrency/quotes/latest?id={}",
        cmc_id
    );
    let quote_resp = match client.get(&quote_url)
        .header("X-CMC_PRO_API_KEY", &key)
        .header("Accept", "application/json")
        .send().await {
        Ok(r) => r,
        Err(e) => { eprintln!("[CMC-DEBUG] quote request error (id={}): {}", cmc_id, e); return None; }
    };
    if !quote_resp.status().is_success() {
        let status = quote_resp.status();
        let body = quote_resp.text().await.unwrap_or_default();
        eprintln!("[CMC-DEBUG] quote non-success status={} id={} body={}", status, cmc_id, body);
        return None;
    }
    let quote_raw = match quote_resp.text().await {
        Ok(t) => t,
        Err(e) => { eprintln!("[CMC-DEBUG] quote body read error: {}", e); return None; }
    };
    let quote_parsed: QuoteResponse = match serde_json::from_str(&quote_raw) {
        Ok(v) => v,
        Err(e) => { eprintln!("[CMC-DEBUG] quote JSON parse error: {} raw={}", e, quote_raw); return None; }
    };
    let data_entry = quote_parsed.data?.into_iter().next()?;
    let usd_quote = data_entry.quote?.into_iter().find(|q| q.symbol == "USD")?;
    usd_quote.price
}
