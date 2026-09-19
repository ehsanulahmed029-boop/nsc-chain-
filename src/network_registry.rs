// network_registry.rs — public, user-extensible registry of EVM-compatible
// networks shown in the wallet's Receive/Send UI. Anyone can add a network;
// "verified" is derived dynamically (CoinGecko platform lookup, or NSC's
// own chain id) rather than trusted from the submitter, so a malicious
// submitter can't self-badge their entry as verified.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub chain_id: u64,
    pub name: String,
    pub rpc_url: String,
    pub native_symbol: String,
    pub explorer_url: String,
    #[serde(default)]
    pub logo_url: String,
    #[serde(default = "default_decimals")]
    pub decimals: u8,
    pub added_by: String,
    pub added_at: u64,
}

fn default_decimals() -> u8 { 18 }

pub fn default_networks() -> Vec<NetworkInfo> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    vec![
        NetworkInfo {
            chain_id: 7788,
            name: "Nusacoin Mainnet".to_string(),
            rpc_url: "https://nusacoin.online/evm_rpc".to_string(),
            native_symbol: "NSC".to_string(),
            explorer_url: "https://nusacoin.online/explorer.html".to_string(),
            logo_url: "".to_string(),
            decimals: 18,
            added_by: "system".to_string(),
            added_at: now,
        },
        NetworkInfo {
            chain_id: 56,
            name: "BNB Smart Chain".to_string(),
            rpc_url: "https://bsc-dataseed.binance.org/".to_string(),
            native_symbol: "BNB".to_string(),
            explorer_url: "https://bscscan.com".to_string(),
            logo_url: "".to_string(),
            decimals: 18,
            added_by: "system".to_string(),
            added_at: now,
        },
        NetworkInfo {
            chain_id: 1,
            name: "Ethereum".to_string(),
            rpc_url: "https://cloudflare-eth.com".to_string(),
            native_symbol: "ETH".to_string(),
            explorer_url: "https://etherscan.io".to_string(),
            logo_url: "".to_string(),
            decimals: 18,
            added_by: "system".to_string(),
            added_at: now,
        },
        NetworkInfo {
            chain_id: 137,
            name: "Polygon".to_string(),
            rpc_url: "https://polygon-rpc.com".to_string(),
            native_symbol: "MATIC".to_string(),
            explorer_url: "https://polygonscan.com".to_string(),
            logo_url: "".to_string(),
            decimals: 18,
            added_by: "system".to_string(),
            added_at: now,
        },
    ]
}

pub fn is_verified(chain_id: u64) -> bool {
    if chain_id == 7788 {
        return true;
    }
    crate::coingecko_client::chain_id_to_cg_platform(chain_id).is_some()
}
