#[derive(Debug, Default)]
pub struct Metrics {
    pub attempts: u32,
    pub success: u32,
    pub failure: u32,
}

impl Metrics {
    /// Record a connection attempt. `success` indicates whether the attempt was successful.
    pub fn record(&mut self, success: bool) {
        self.attempts += 1;
        if success {
            self.success += 1;
        } else {
            self.failure += 1;
        }
    }

    /// Print a report of the collected metrics.
    /// Output Format: "<attempts> attempts, success: <successes>, fail: <failures>, failure rate: <failure_rate>%"
    pub fn report(&self) {
        let failure_rate = if self.attempts > 0 {
            (self.failure as f64 / self.attempts as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "{} attempts, success: {}, fail: {}, failure rate: {:.2}%",
            self.attempts, self.success, self.failure, failure_rate
        );
    }
}
