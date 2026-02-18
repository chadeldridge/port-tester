#[derive(Debug, Default)]
pub struct Metrics {
    pub attempts: u32,
    pub successes: u32,
    pub failures: u32,
}

impl Metrics {
    /// Record a connection attempt. `success` indicates whether the attempt was successful.
    pub fn record_attempt(&mut self, success: bool) {
        self.attempts += 1;
        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
    }

    /// Print a report of the collected metrics.
    /// Output Format: "<attempts> attempts, success: <successes>, fail: <failures>, failure rate: <failure_rate>%"
    pub fn report(&self) {
        let failure_rate = if self.attempts > 0 {
            (self.failures as f64 / self.attempts as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "{} attempts, success: {}, fail: {}, failure rate: {:.2}%",
            self.attempts, self.successes, self.failures, failure_rate
        );
    }
}
