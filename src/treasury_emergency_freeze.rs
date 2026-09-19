// WARNING: DISCONNECTED FROM REAL STATE (found 2026-08-09).
// This struct's `frozen` field is entirely separate from the
// REAL treasury freeze flag (Treasury.frozen in treasury.rs),
// which IS correctly checked by execute_spend(). freeze()/
// unfreeze() on THIS struct are never called anywhere, so its
// .show() output ("Frozen: false") is always wrong-by-omission —
// it reflects nothing about real treasury state. Do not assume
// this provides any active protection or accurate status.
pub struct TreasuryEmergencyFreeze {

    pub frozen: bool,
}

impl TreasuryEmergencyFreeze {

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
            "Treasury Frozen"
        );
    }

    pub fn unfreeze(
        &mut self,
    ) {

        self.frozen = false;

        println!(
            "Treasury Unfrozen"
        );
    }

    pub fn is_frozen(
        &self,
    ) -> bool {

        self.frozen
    }

    pub fn can_execute(
        &self,
    ) -> bool {

        !self.frozen
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== TREASURY EMERGENCY FREEZE ====="
        );

        println!(
            "Frozen: {}",
            self.frozen
        );

        println!(
            "Can Execute: {}",
            self.can_execute()
        );
    }
}
