#[derive(Debug)]
pub struct RecoveryWallet {
    pub seed_phrase: String,
}

impl RecoveryWallet {
    pub fn recover(
        phrase: String,
    ) -> Self {
        Self {
            seed_phrase: phrase,
        }
    }

    pub fn verify(&self) -> bool {
        self.seed_phrase
            .split_whitespace()
            .count() >= 12
    }
}
