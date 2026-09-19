#[derive(Debug, Clone)]
pub struct Trade {
    pub trade_type: String,
    pub amount_in: u128,
    pub amount_out: u128,
}
