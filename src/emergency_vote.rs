use std::collections::HashSet;

#[derive(Debug)]
pub struct EmergencyProposal {

    pub title: String,

    pub approvals:
        HashSet<String>,

    pub rejections:
        HashSet<String>,
}

impl EmergencyProposal {

    pub fn new(
        title: String,
    ) -> Self {

        Self {
            title,
            approvals:
                HashSet::new(),
            rejections:
                HashSet::new(),
        }
    }

    pub fn approve(
        &mut self,
        validator: String,
    ) {

        self.approvals.insert(
            validator
        );
    }

    pub fn reject(
        &mut self,
        validator: String,
    ) {

        self.rejections.insert(
            validator
        );
    }

    pub fn approved(
        &self,
        total_members: usize,
    ) -> bool {

        self.approvals.len()
            * 3
            >= total_members * 2
    }

    pub fn rejected(
        &self,
        total_members: usize,
    ) -> bool {

        self.rejections.len()
            * 3
            >= total_members * 2
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== EMERGENCY PROPOSAL ====="
        );

        println!(
            "Title: {}",
            self.title
        );

        println!(
            "Approvals: {}",
            self.approvals.len()
        );

        println!(
            "Rejections: {}",
            self.rejections.len()
        );
    }
}
