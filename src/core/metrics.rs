use chrono::Local;
use std::fmt::Write;

use crate::{Error, Verbosity};

const STATUS_SUCCESS: &str = "ok";
const STATUS_FAILURE: &str = "fail";

#[derive(Debug)]
#[non_exhaustive]
pub enum Status {
    Success,
    Failure(Option<Error>),
}

impl Default for Status {
    fn default() -> Self {
        Status::Failure(None)
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_with_verbosity(&Verbosity::Normal).as_str()
        )
    }
}

impl Status {
    pub fn new(success: bool, error: Option<Error>) -> Self {
        match success {
            true => Status::Success,
            false => Status::Failure(error),
        }
    }

    pub fn is_err(&self) -> bool {
        match self {
            Status::Success => false,
            Status::Failure(_) => true,
        }
    }

    pub fn to_string_with_verbosity(&self, verbosity: &Verbosity) -> String {
        match self {
            Status::Success => STATUS_SUCCESS.to_string(),
            Status::Failure(err) => match verbosity {
                Verbosity::Silent => "".to_string(),
                Verbosity::Quiet => STATUS_FAILURE.to_string(),
                _ => match err {
                    Some(e) => format!("{}: {}", STATUS_FAILURE, e),
                    None => STATUS_FAILURE.to_string(),
                },
            },
        }
    }
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Metrics {
    results: Vec<MetricsResult>,
    summary: MetricsSummary,
    verbosity: Verbosity,
}

impl Metrics {
    pub fn new(verbose: &Verbosity) -> Self {
        Metrics {
            results: Vec::new(),
            summary: MetricsSummary::default(),
            verbosity: verbose.to_owned(),
        }
    }

    pub fn attempts(&self) -> u32 {
        self.summary.attempts()
    }

    pub fn success(&self) -> u32 {
        self.summary.success()
    }

    pub fn failure(&self) -> u32 {
        self.summary.failure()
    }

    pub fn failure_rate(&self) -> f64 {
        self.summary.failure_rate()
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    pub fn iter(&self) -> impl Iterator<Item = &MetricsResult> {
        self.results.iter()
    }

    pub fn record(
        &mut self,
        seq: u32,
        timestamp: chrono::DateTime<Local>,
        duration: chrono::TimeDelta,
        status: Status,
    ) {
        self.summary.record(&status);
        let result = MetricsResult::new(seq, timestamp, duration, status);
        self.results.push(result);
    }

    pub fn result(&self, seq: u32) -> Option<&MetricsResult> {
        let i = match seq {
            0 => 0,
            _ => seq - 1,
        };
        self.results.get(i as usize)
    }

    pub fn report(&self) -> String {
        self.summary.report()
    }

    pub fn full_report(&self) -> String {
        let mut report = String::new();
        for r in &self.results {
            // There's no reason why writelin! should fail to write to String so swallow the Result.
            let _ = writeln!(report, "{}", r.to_string_with_verbosity(&self.verbosity));
        }

        // Write an empty line as a separator.
        let _ = writeln!(report);
        report
    }
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct MetricsResult {
    seq: u32,
    timestamp: chrono::DateTime<Local>,
    duration: chrono::TimeDelta,
    status: Status,
}

impl std::fmt::Display for MetricsResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_with_verbosity(&Verbosity::Normal).as_str()
        )
    }
}

impl MetricsResult {
    pub fn new(
        seq: u32,
        timestamp: chrono::DateTime<Local>,
        duration: chrono::TimeDelta,
        status: Status,
    ) -> Self {
        MetricsResult {
            seq,
            timestamp,
            duration,
            status,
        }
    }

    pub fn seq(&self) -> u32 {
        self.seq
    }

    pub fn timestamp(&self) -> chrono::DateTime<Local> {
        self.timestamp
    }

    pub fn duration(&self) -> chrono::TimeDelta {
        self.duration
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn is_err(&self) -> bool {
        match self.status {
            Status::Success => false,
            Status::Failure(_) => true,
        }
    }

    pub fn to_string_with_verbosity(&self, verbosity: &Verbosity) -> String {
        match *verbosity {
            Verbosity::Verbose(n) => match n {
                // Same as Verbosity::Normal.
                0 => format!(
                    "{} {}",
                    self.seq,
                    self.status.to_string_with_verbosity(verbosity)
                ),
                1 => format!(
                    "{} {:.2}s {}",
                    self.seq,
                    self.duration.as_seconds_f32(),
                    self.status.to_string_with_verbosity(verbosity)
                ),
                2 => format!(
                    "{} {} {:.2}s {}",
                    self.timestamp,
                    self.seq,
                    self.duration.as_seconds_f32(),
                    self.status.to_string_with_verbosity(verbosity)
                ),
                _ => format!(
                    "start={} seq={} dur={:.2}s status={}",
                    self.timestamp,
                    self.seq,
                    self.duration.as_seconds_f32(),
                    self.status.to_string_with_verbosity(verbosity)
                ),
            },
            Verbosity::Normal => format!(
                "{} {}",
                self.seq,
                self.status.to_string_with_verbosity(verbosity)
            ),
            Verbosity::Quiet => format!(
                "{} {}",
                self.seq,
                self.status.to_string_with_verbosity(verbosity)
            ),
            Verbosity::Silent => "".to_string(),
        }
    }
}

/// A metrics store to track attempt successes and failures.
///
/// Create a new MetricsSummary object with [`MetricsSummary::default`].
/// Record an attempt with [`MetricsSummary::record`].
/// Generate a report with [`MetricsSummary::report`].
///
/// # Examples
///
/// ```no_run
/// use pt::core::metrics::{MetricsSummary, Status};
///
/// let mut ms = MetricsSummary::default();
/// ms.record(&Status::Success);
/// ms.record(&Status::new(false, None));
/// ms.report();
/// ```
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub struct MetricsSummary {
    attempts: u32,
    success: u32,
    failure: u32,
}

impl MetricsSummary {
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn success(&self) -> u32 {
        self.success
    }

    pub fn failure(&self) -> u32 {
        self.failure
    }

    pub fn failure_rate(&self) -> f64 {
        if self.attempts > 0 {
            (self.failure as f64 / self.attempts as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Record a connection attempt.
    /// `success` = true increments [`Metrics.success`] by 1.
    /// `success` = false increments [`Metrics.failure`] by 1.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pt::core::metrics::{MetricsSummary, Status};
    ///
    /// let mut ms = MetricsSummary::default();
    /// ms.record(&Status::Success);
    /// ms.record(&Status::new(false, None));
    /// assert_eq!(ms.success(), 1);
    /// assert_eq!(ms.failure(), 1);
    /// assert_eq!(ms.attempts(), 2);
    /// ```
    pub fn record(&mut self, status: &Status) {
        self.attempts += 1;
        match status {
            Status::Success => self.success += 1,
            Status::Failure(..) => self.failure += 1,
        }
    }

    /// Print a report of the collected metrics.
    /// Output Format: "<count> attempts, success: <successes>, fail: <failures>, failure rate: <failure_rate>%"
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pt::core::metrics::{MetricsSummary, Status};
    ///
    /// let mut ms = MetricsSummary::default();
    /// ms.record(&Status::Success);
    /// ms.record(&Status::Success);
    /// ms.report();
    /// ```
    /// Output: attempts: 2, success: 2, fail: 0, failure rate: 0.00%
    pub fn report(&self) -> String {
        format!(
            "attempts: {}, success: {}, fail: {}, failure rate: {:.2}%",
            self.attempts,
            self.success,
            self.failure,
            self.failure_rate()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metricssummary_record() {
        let mut m = MetricsSummary::default();
        assert_eq!(m.attempts, 0);
        assert_eq!(m.success, 0);
        assert_eq!(m.failure, 0);

        m.record(&Status::Success);
        assert_eq!(m.attempts, 1);
        assert_eq!(m.success, 1);
        assert_eq!(m.failure, 0);

        m.record(&Status::Failure(None));
        assert_eq!(m.attempts, 2);
        assert_eq!(m.success, 1);
        assert_eq!(m.failure, 1);
    }

    #[test]
    fn test_metricssummary_report() {
        let mut m = MetricsSummary::default();
        m.record(&Status::Success);
        m.record(&Status::Failure(None));
        let output = m.report();

        assert_eq!(
            output,
            "attempts: 2, success: 1, fail: 1, failure rate: 50.00%".to_string()
        );
    }
}
