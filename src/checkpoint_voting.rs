use std::collections::HashMap;

pub struct CheckpointVoting {

    pub approvals:
        HashMap<u64, Vec<String>>,

    pub rejections:
        HashMap<u64, Vec<String>>,
}

impl CheckpointVoting {

    pub fn new() -> Self {

        Self {

            approvals:
                HashMap::new(),

            rejections:
                HashMap::new(),
        }
    }

    pub fn approve(
        &mut self,
        height: u64,
        validator: String,
    ) {

        self.approvals
            .entry(height)
            .or_insert(Vec::new())
            .push(validator);
    }

    pub fn reject(
        &mut self,
        height: u64,
        validator: String,
    ) {

        self.rejections
            .entry(height)
            .or_insert(Vec::new())
            .push(validator);
    }

    pub fn approval_count(
        &self,
        height: u64,
    ) -> usize {

        self.approvals
            .get(&height)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn rejection_count(
        &self,
        height: u64,
    ) -> usize {

        self.rejections
            .get(&height)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn show(
        &self,
        height: u64,
    ) {

        println!(
            "\n===== CHECKPOINT VOTING ====="
        );

        println!(
            "Height: {}",
            height
        );

        println!(
            "Approvals: {}",
            self.approval_count(
                height
            )
        );

        println!(
            "Rejections: {}",
            self.rejection_count(
                height
            )
        );
    }
}
