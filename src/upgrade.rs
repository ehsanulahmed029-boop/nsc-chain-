#[derive(Debug, Clone)]
pub struct UpgradeProposal {

    pub id: u64,
    pub name: String,
    pub activation_epoch: u64,
    pub executed: bool,

}

pub struct UpgradeManager {

    pub upgrades: Vec<UpgradeProposal>,

}

impl UpgradeManager {

    pub fn new() -> Self {

        Self {
            upgrades: Vec::new(),
        }

    }

    pub fn schedule_upgrade(
        &mut self,
        id: u64,
        name: String,
        activation_epoch: u64,
    ) {

        self.upgrades.push(
            UpgradeProposal {
                id,
                name,
                activation_epoch,
                executed: false,
            }
        );

    }

    pub fn process_epoch(
        &mut self,
        current_epoch: u64,
    ) {

        for upgrade
            in &mut self.upgrades
        {

            if !upgrade.executed
                &&
                current_epoch
                >=
                upgrade.activation_epoch
            {

                upgrade.executed = true;

                println!(
                    "Upgrade Activated: {}",
                    upgrade.name
                );

            }

        }

    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== SCHEDULED UPGRADES ====="
        );

        for upgrade
            in &self.upgrades
        {

            println!(
                "[{}] {} | epoch={} | executed={}",
                upgrade.id,
                upgrade.name,
                upgrade.activation_epoch,
                upgrade.executed
            );

        }

    }
}
