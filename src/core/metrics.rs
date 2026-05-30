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
        let _ = writeln!(report, "{}", self.report());
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
                    "{} {:.2}ms {}",
                    self.seq,
                    self.duration.num_milliseconds(),
                    self.status.to_string_with_verbosity(verbosity)
                ),
                2 => format!(
                    "{} {} {:.2}ms {}",
                    self.timestamp,
                    self.seq,
                    self.duration.num_milliseconds(),
                    self.status.to_string_with_verbosity(verbosity)
                ),
                _ => format!(
                    "start={} seq={} dur={:.2}ms status={}",
                    self.timestamp,
                    self.seq,
                    self.duration.num_milliseconds(),
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
    use crate::SourceError;

    use super::*;

    #[test]
    fn test_status() {
        // Status::Failure(None) from Default
        let s1 = Status::default();
        assert!(s1.is_err());
        assert_eq!(format!("{}", s1), "fail".to_string());
        assert_eq!(
            s1.to_string_with_verbosity(&Verbosity::Silent),
            "".to_string()
        );
        assert_eq!(
            s1.to_string_with_verbosity(&Verbosity::Quiet),
            "fail".to_string()
        );
        assert_eq!(
            s1.to_string_with_verbosity(&Verbosity::Verbose(2)),
            "fail".to_string()
        );

        // Status::Success
        let s2 = Status::Success;
        assert!(!s2.is_err());
        assert_eq!(format!("{}", s2), "ok".to_string());
        assert_eq!(
            s2.to_string_with_verbosity(&Verbosity::Silent),
            "ok".to_string()
        );
        assert_eq!(
            s2.to_string_with_verbosity(&Verbosity::Quiet),
            "ok".to_string()
        );
        assert_eq!(
            s2.to_string_with_verbosity(&Verbosity::Verbose(2)),
            "ok".to_string()
        );

        // Status::Failure(Error)
        let s3 = Status::Failure(Some(Error::new(SourceError::from("test error"))));
        assert!(s3.is_err());
        assert_eq!(format!("{}", s3), "fail: test error".to_string());
        assert_eq!(
            s3.to_string_with_verbosity(&Verbosity::Silent),
            "".to_string()
        );
        assert_eq!(
            s3.to_string_with_verbosity(&Verbosity::Quiet),
            "fail".to_string()
        );
        assert_eq!(
            s3.to_string_with_verbosity(&Verbosity::Verbose(2)),
            "fail: test error".to_string()
        );
    }

    #[test]
    fn test_metricsresult() {
        let dur = chrono::Duration::try_milliseconds(1234).unwrap();
        let start = Local::now() - dur;
        let mr = MetricsResult::new(1, start, dur, Status::Success);
        assert!(!mr.is_err());
        assert_eq!(mr.to_string_with_verbosity(&Verbosity::Normal), "1 ok");
        assert_eq!(mr.to_string_with_verbosity(&Verbosity::Quiet), "1 ok");
        assert_eq!(mr.to_string_with_verbosity(&Verbosity::Silent), "");
        assert_eq!(mr.to_string_with_verbosity(&Verbosity::Verbose(0)), "1 ok");
        assert_eq!(
            mr.to_string_with_verbosity(&Verbosity::Verbose(1)),
            "1 1234ms ok"
        );
        assert_eq!(
            mr.to_string_with_verbosity(&Verbosity::Verbose(2)),
            format!("{} 1 1234ms ok", start)
        );
        assert_eq!(
            mr.to_string_with_verbosity(&Verbosity::Verbose(3)),
            format!("start={} seq=1 dur=1234ms status=ok", start)
        );

        // Test with Failure(None).
        let mr = MetricsResult::new(1, start, dur, Status::Failure(None));
        assert!(mr.is_err());
        assert_eq!(mr.to_string_with_verbosity(&Verbosity::Normal), "1 fail");

        // Test with Failure(Some(Error))
        let mr = MetricsResult::new(
            1,
            start,
            dur,
            Status::Failure(Some(Error::new(SourceError::from("test error")))),
        );
        assert!(mr.is_err());
        assert_eq!(
            mr.to_string_with_verbosity(&Verbosity::Normal),
            "1 fail: test error"
        );
    }

    #[test]
    fn test_metricssummary() {
        let mut ms = MetricsSummary::default();
        assert_eq!(ms.attempts, 0);
        assert_eq!(ms.success, 0);
        assert_eq!(ms.failure, 0);

        ms.record(&Status::Success);
        assert_eq!(ms.attempts, 1);
        assert_eq!(ms.success, 1);
        assert_eq!(ms.failure, 0);

        ms.record(&Status::Failure(None));
        assert_eq!(ms.attempts, 2);
        assert_eq!(ms.success, 1);
        assert_eq!(ms.failure, 1);

        assert_eq!(ms.failure_rate(), 50.00);
        // Test report()
        assert_eq!(
            ms.report(),
            "attempts: 2, success: 1, fail: 1, failure rate: 50.00%".to_string()
        );
    }

    #[test]
    fn test_metrics() {
        let mut m = Metrics::new(&Verbosity::Normal);
        assert_eq!(m.verbosity(), Verbosity::Normal);
        let dur = chrono::Duration::try_milliseconds(1234).unwrap();
        m.record(1, Local::now() - dur, dur, Status::Success);
        m.record(
            2,
            Local::now() - dur,
            dur,
            Status::Failure(Some(Error::new(SourceError::from("test error")))),
        );
        assert_eq!(m.len(), 2);
        assert_eq!(m.iter().count(), 2);
        assert_eq!(m.attempts(), 2);
        assert_eq!(m.success(), 1);
        assert_eq!(m.failure(), 1);
        assert_eq!(m.failure_rate(), 50.00);

        // Test pulling back a result.
        assert!(m.result(2).unwrap().is_err());

        // Test report()
        assert_eq!(
            m.report(),
            "attempts: 2, success: 1, fail: 1, failure rate: 50.00%".to_string()
        );

        // Test report()
        assert_eq!(
            m.full_report(),
            "1 ok\n2 fail: test error\n\nattempts: 2, success: 1, fail: 1, failure rate: 50.00%\n"
                .to_string()
        );
    }
}
