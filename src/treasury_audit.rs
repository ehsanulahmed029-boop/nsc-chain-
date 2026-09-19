#[derive(Debug, Clone)]
pub struct AuditRecord {

    pub tx_id: String,

    pub amount: u64,

    pub direction: String,
}

pub struct TreasuryAudit {

    pub logs:
        Vec<AuditRecord>,
}

impl TreasuryAudit {

    pub fn new() -> Self {

        Self {
            logs: Vec::new(),
        }
    }

    pub fn record(
        &mut self,
        tx_id: String,
        amount: u64,
        direction: String,
    ) {

        self.logs.push(
            AuditRecord {

                tx_id,

                amount,

                direction,
            }
        );
    }

    pub fn detect_suspicious(
        &self,
        threshold: u64,
    ) {

        println!(
            "\n===== SUSPICIOUS ACTIVITY ====="
        );

        for log in &self.logs {

            if log.amount > threshold {

                println!(
                    "Suspicious TX: {} amount={}",
                    log.tx_id,
                    log.amount
                );
            }
        }
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== TREASURY AUDIT LOG ====="
        );

        for log in &self.logs {

            println!(
                "{} | {} | {}",
                log.tx_id,
                log.amount,
                log.direction
            );
        }

        println!(
            "Total Records: {}",
            self.logs.len()
        );
    }
}
