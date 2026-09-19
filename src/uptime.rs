use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct UptimeInfo {
    pub produced_blocks: u64,
    pub missed_blocks: u64,
}

#[derive(Debug)]
pub struct UptimeTracker {
    pub validators: HashMap<String, UptimeInfo>,
}

impl UptimeTracker {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        validator: String,
    ) {
        self.validators.insert(
            validator,
            UptimeInfo {
                produced_blocks: 0,
                missed_blocks: 0,
            },
        );
    }

    pub fn record_success(
        &mut self,
        validator: &str,
    ) {
        if let Some(v) =
            self.validators.get_mut(validator)
        {
            v.produced_blocks += 1;
        }
    }

    pub fn record_miss(
        &mut self,
        validator: &str,
    ) {
        if let Some(v) =
            self.validators.get_mut(validator)
        {
            v.missed_blocks += 1;
        }
    }

    pub fn uptime_percent(
        &self,
        validator: &str,
    ) -> f64 {

        if let Some(v) =
            self.validators.get(validator)
        {
            let total =
                v.produced_blocks
                + v.missed_blocks;

            if total == 0 {
                return 100.0;
            }

            return
                (v.produced_blocks as f64
                / total as f64)
                * 100.0;
        }

        0.0
    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== UPTIME ====="
        );

        for (addr, info)
            in &self.validators
        {
            let total =
                info.produced_blocks
                + info.missed_blocks;

            let uptime =
                if total == 0 {
                    100.0
                } else {
                    (info.produced_blocks as f64
                    / total as f64)
                    * 100.0
                };

            println!(
                "{} | produced={} | missed={} | uptime={:.2}%",
                addr,
                info.produced_blocks,
                info.missed_blocks,
                uptime
            );
        }
    }
}
