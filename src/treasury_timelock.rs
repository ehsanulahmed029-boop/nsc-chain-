use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

#[derive(Debug)]
pub struct TimeLockedTransaction {

    pub amount: u64,

    pub unlock_time: u64,
}

pub struct TreasuryTimeLock;

impl TreasuryTimeLock {

    pub fn create(
        amount: u64,
        delay_seconds: u64,
    ) -> TimeLockedTransaction {

        let now =
            SystemTime::now()
            .duration_since(
                UNIX_EPOCH
            )
            .unwrap()
            .as_secs();

        TimeLockedTransaction {

            amount,

            unlock_time:
                now + delay_seconds,
        }
    }

    pub fn executable(
        tx: &TimeLockedTransaction,
    ) -> bool {

        let now =
            SystemTime::now()
            .duration_since(
                UNIX_EPOCH
            )
            .unwrap()
            .as_secs();

        now >= tx.unlock_time
    }

    pub fn show(
        tx: &TimeLockedTransaction,
    ) {

        println!(
            "\n===== TREASURY TIMELOCK ====="
        );

        println!(
            "Amount: {}",
            tx.amount
        );

        println!(
            "Unlock Time: {}",
            tx.unlock_time
        );

        println!(
            "Executable: {}",
            Self::executable(tx)
        );
    }
}
