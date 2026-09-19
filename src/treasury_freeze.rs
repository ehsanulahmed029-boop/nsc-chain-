#[derive(Debug)]
// ⚠️ DEAD-BY-DESIGN + MISLEADING NAME (Batch 2 audit, 2026-08-10):
// This struct is NOT the real treasury freeze mechanism. The REAL freeze
// flag is `Treasury.frozen` in treasury.rs, which IS correctly checked in
// execute_spend(). This TreasuryFreeze struct is a disconnected duplicate:
// its can_spend() is never called anywhere, so it enforces nothing. Do not
// use this struct to build freeze functionality — use Treasury.freeze()
// in treasury.rs instead. See audit notes.

pub struct TreasuryFreeze {

    pub frozen: bool,
}

impl TreasuryFreeze {

    pub fn new() -> Self {

        Self {
            frozen: false,
        }
    }

    pub fn freeze(
        &mut self,
    ) {

        self.frozen = true;

        println!(
            "TREASURY FROZEN"
        );
    }

    pub fn unfreeze(
        &mut self,
    ) {

        self.frozen = false;

        println!(
            "TREASURY UNFROZEN"
        );
    }

    pub fn can_spend(
        &self,
    ) -> bool {

        !self.frozen
    }

    pub fn status(
        &self,
    ) {

        println!(
            "\n===== TREASURY STATUS ====="
        );

        println!(
            "Frozen: {}",
            self.frozen
        );
    }
}
