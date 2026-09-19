use serde::Serialize;

#[derive(Serialize)]
pub struct ChainInfo {
    pub height: usize,
    pub difficulty: usize,
}

#[derive(Serialize)]
pub struct SupplyInfo {
    pub max_supply: u64,
    pub circulating: u64,
}

#[derive(Serialize)]
pub struct PoolInfo {
    pub nsc_reserve: u64,
    pub usdt_reserve: u64,
    pub lp_supply: u64,
}
