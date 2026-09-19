// token_registry.rs — "Any Coin, Any Network" Phase A: detection + display only, no custody/send
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedToken {
    pub chain_id: u64,
    pub contract_address: String, // lowercase; "" for native coin
    pub symbol: String,
    pub name: String,
    pub logo_url: String,
    pub decimals: u8,
    pub verified: bool,
    pub first_seen_at: u64,
    #[serde(default)]
    pub price_usd: f64,
    #[serde(default)]
    pub last_checked_at: u64,
}

impl DetectedToken {
    pub fn registry_key(chain_id: u64, contract_address: &str) -> String {
        format!("{}:{}", chain_id, contract_address.to_lowercase())
    }

    pub fn unverified(chain_id: u64, contract_address: &str) -> Self {
        let addr = contract_address.to_lowercase();
        let short = if addr.len() >= 10 {
            format!("{}...{}", &addr[0..6], &addr[addr.len()-4..])
        } else {
            addr.clone()
        };
        DetectedToken {
            chain_id,
            contract_address: addr,
            symbol: "UNVERIFIED".to_string(),
            name: short,
            logo_url: "".to_string(),
            decimals: 18,
            verified: false,
            first_seen_at: now_ts(),
            last_checked_at: now_ts(),
            price_usd: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTokenBalance {
    pub user_address: String, // lowercase
    pub chain_id: u64,
    pub contract_address: String, // "" for native
    pub balance: String, // u128 stored as string — NEVER put raw u128 into json!()
    pub last_updated_at: u64,
}

impl UserTokenBalance {
    pub fn key(user_address: &str, chain_id: u64, contract_address: &str) -> String {
        format!("{}:{}:{}", user_address.to_lowercase(), chain_id, contract_address.to_lowercase())
    }

    pub fn balance_u128(&self) -> u128 {
        self.balance.parse().unwrap_or(0)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TokenRegistryState {
    pub tokens: HashMap<String, DetectedToken>,
    pub balances: HashMap<String, UserTokenBalance>,
}

pub struct TokenRegistry {
    state: RwLock<TokenRegistryState>,
    file_path: String,
}

fn now_ts() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

impl TokenRegistry {
    pub fn new(file_path: &str) -> Self {
        let state = Self::load_from_disk(file_path).unwrap_or_default();
        TokenRegistry { state: RwLock::new(state), file_path: file_path.to_string() }
    }

    fn load_from_disk(path: &str) -> Option<TokenRegistryState> {
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save_to_disk(&self) -> std::io::Result<()> {
        let state = self.state.read().unwrap();
        let json = serde_json::to_string_pretty(&*state)?;
        fs::write(&self.file_path, json)
    }

    pub fn get_token(&self, chain_id: u64, contract_address: &str) -> Option<DetectedToken> {
        let key = DetectedToken::registry_key(chain_id, contract_address);
        self.state.read().unwrap().tokens.get(&key).cloned()
    }

    pub fn insert_token(&self, token: DetectedToken) -> std::io::Result<()> {
        let key = DetectedToken::registry_key(token.chain_id, &token.contract_address);
        { self.state.write().unwrap().tokens.insert(key, token); }
        self.save_to_disk()
    }

    /// Get or create (as unverified) — used when CoinGecko lookup fails
    pub fn get_or_insert_unverified(&self, chain_id: u64, contract_address: &str) -> DetectedToken {
        if let Some(t) = self.get_token(chain_id, contract_address) {
            return t;
        }
        let t = DetectedToken::unverified(chain_id, contract_address);
        let _ = self.insert_token(t.clone());
        t
    }

    pub fn set_balance(&self, user_address: &str, chain_id: u64, contract_address: &str, amount: u128) -> std::io::Result<()> {
        let key = UserTokenBalance::key(user_address, chain_id, contract_address);
        {
            let mut state = self.state.write().unwrap();
            let entry = state.balances.entry(key).or_insert_with(|| UserTokenBalance {
                user_address: user_address.to_lowercase(),
                chain_id,
                contract_address: contract_address.to_lowercase(),
                balance: "0".to_string(),
                last_updated_at: now_ts(),
            });
            entry.balance = amount.to_string();
            entry.last_updated_at = now_ts();
        }
        self.save_to_disk()
    }

    /// Adds `amount` to an existing balance (does not overwrite) — used by watchers.
    pub fn credit_balance(&self, user_address: &str, chain_id: u64, contract_address: &str, amount: u128) -> std::io::Result<()> {
        let key = UserTokenBalance::key(user_address, chain_id, contract_address);
        {
            let mut state = self.state.write().unwrap();
            let entry = state.balances.entry(key).or_insert_with(|| UserTokenBalance {
                user_address: user_address.to_lowercase(),
                chain_id,
                contract_address: contract_address.to_lowercase(),
                balance: "0".to_string(),
                last_updated_at: now_ts(),
            });
            let current: u128 = entry.balance.parse().unwrap_or(0);
            entry.balance = (current + amount).to_string();
            entry.last_updated_at = now_ts();
        }
        self.save_to_disk()
    }

    pub fn get_user_tokens(&self, user_address: &str) -> Vec<(DetectedToken, UserTokenBalance)> {
        let state = self.state.read().unwrap();
        let user_lower = user_address.to_lowercase();
        state.balances.values()
            .filter(|b| b.user_address == user_lower && b.balance_u128() > 0)
            .filter_map(|b| {
                let tkey = DetectedToken::registry_key(b.chain_id, &b.contract_address);
                state.tokens.get(&tkey).map(|t| (t.clone(), b.clone()))
            })
            .collect()
    }

    /// Updates the cached USD price for an existing known token. No-op if the
    /// token isn't in the registry yet (price refresh runs after detection).
    pub fn update_token_price(&self, chain_id: u64, contract_address: &str, price_usd: f64) -> std::io::Result<()> {
        let key = DetectedToken::registry_key(chain_id, contract_address);
        {
            let mut state = self.state.write().unwrap();
            if let Some(t) = state.tokens.get_mut(&key) {
                t.price_usd = price_usd;
            }
        }
        self.save_to_disk()
    }

    /// Returns every distinct known token (across all users) — used by the
    /// price-refresh pass so we fetch each token's price once per cycle,
    /// not once per user holding it.
    pub fn list_all_tokens(&self) -> Vec<DetectedToken> {
        self.state.read().unwrap().tokens.values().cloned().collect()
    }
}
