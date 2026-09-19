#[derive(Debug)]
pub struct TreasuryProtection {

    pub expected_balance: u128,
}

impl TreasuryProtection {

    pub fn new(
        expected_balance: u128,
    ) -> Self {

        Self {
            expected_balance,
        }
    }

    pub fn verify(
        &self,
        current_balance: u128,
    ) -> bool {

        current_balance
            == self.expected_balance
    }

    pub fn detect_tampering(
        &self,
        current_balance: u128,
    ) {

        if self.verify(
            current_balance
        ) {

            println!(
                "Treasury Integrity Verified"
            );
        } else {

            println!(
                "TREASURY TAMPERING DETECTED"
            );

            println!(
                "Expected: {}",
                self.expected_balance
            );

            println!(
                "Current: {}",
                current_balance
            );
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== TREASURY PROTECTION ====="
        );

        println!(
            "Protected Balance: {}",
            self.expected_balance
        );
    }
}
