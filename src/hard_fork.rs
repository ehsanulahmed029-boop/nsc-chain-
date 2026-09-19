#[derive(Debug, Clone)]
pub struct HardFork {

    pub id: u64,
    pub name: String,
    pub activation_height: u64,
    pub activated: bool,

}

pub struct HardForkManager {

    pub forks: Vec<HardFork>,

}

impl HardForkManager {

    pub fn new() -> Self {

        Self {
            forks: Vec::new(),
        }

    }

    pub fn schedule_fork(
        &mut self,
        id: u64,
        name: String,
        activation_height: u64,
    ) {

        self.forks.push(
            HardFork {
                id,
                name,
                activation_height,
                activated: false,
            }
        );

    }

    pub fn process_height(
        &mut self,
        current_height: u64,
    ) {

        for fork
            in &mut self.forks
        {

            if !fork.activated
                &&
                current_height
                >=
                fork.activation_height
            {

                fork.activated = true;

                println!(
                    "Hard Fork Activated: {}",
                    fork.name
                );

            }

        }

    }

    pub fn active_forks(
        &self,
    ) -> usize {

        self.forks
            .iter()
            .filter(
                |f|
                f.activated
            )
            .count()

    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== HARD FORKS ====="
        );

        for fork
            in &self.forks
        {

            println!(
                "[{}] {} | height={} | active={}",
                fork.id,
                fork.name,
                fork.activation_height,
                fork.activated
            );

        }

    }

}
