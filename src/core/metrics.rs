use chrono::Local;
use std::fmt::Write;

use crate::{Error, Verbosity};

/// Constant value to print for Success.
const STATUS_SUCCESS: &str = "ok";
/// Constant value to print for Failure.
const STATUS_FAILURE: &str = "fail";

/// Holds the status of a port open attempt.
///
/// `Failure` wraps an optional [`Error`] for cases where the failure was caused by a connection
/// error. Defaults to [`Status::Failure`]`(None)` via [`Default`].
///
/// # Examples
///
/// ```no_run
/// use port_tester::core::metrics::Status;
///
/// let success = Status::Success;
/// assert!(!success.is_err());
///
/// let failure = Status::default();
/// assert!(failure.is_err());
///
/// let from_bool = Status::new(false, None);
/// assert!(from_bool.is_err());
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Status {
    Success,
    Failure(Option<Error>),
}

/// Defaults to [`Status::Failure`]`(None)`.
///
/// A default failure with no error represents an uninitialized or unrecorded state. Explicit
/// success must always be constructed as [`Status::Success`].
///
/// # Examples
///
/// ```no_run
/// use port_tester::core::metrics::Status;
///
/// let status = Status::default();
/// assert!(status.is_err());
/// ```
impl Default for Status {
    fn default() -> Self {
        Status::Failure(None)
    }
}

/// Formats using [`Verbosity::Normal`]. Use [`Status::to_string_with_verbosity`] to control
/// output detail.
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
    /// Create a new [`Status`] from a boolean success flag and an optional [`Error`]. [`Error`] is
    /// only used for failure. Use [`None`] for success.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use port_tester::core::metrics::Status;
    ///
    /// let success = Status::new(true, None);
    /// assert!(!success.is_err());
    ///
    /// let failure = Status::new(false, None);
    /// assert!(failure.is_err());
    /// ```
    pub fn new(success: bool, error: Option<Error>) -> Self {
        match success {
            true => Status::Success,
            false => Status::Failure(error),
        }
    }

    /// Returns `true` if this status represents a failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use port_tester::core::metrics::Status;
    ///
    /// assert!(!Status::Success.is_err());
    /// assert!(Status::Failure(None).is_err());
    /// ```
    pub fn is_err(&self) -> bool {
        match self {
            Status::Success => false,
            Status::Failure(_) => true,
        }
    }

    /// Returns the string representation of this status for the given [`Verbosity`] level.
    ///
    /// [`Status::Success`] always returns `"ok"` regardless of verbosity. For [`Status::Failure`],
    /// the output depends on verbosity:
    /// [`Verbosity::Silent`]: empty string
    /// [`Verbosity::Quiet`]: `"fail"`
    /// All others: `"fail"` or `"fail: <error>"` if an error is present
    ///
    /// Prefer this over [`Display`] when the verbosity level is known, as [`Display`] always
    /// uses [`Verbosity::Normal`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use port_tester::core::metrics::Status;
    /// use port_tester::Verbosity;
    ///
    /// let success = Status::Success;
    /// assert_eq!(success.to_string_with_verbosity(&Verbosity::Normal), "ok");
    ///
    /// let failure = Status::Failure(None);
    /// assert_eq!(failure.to_string_with_verbosity(&Verbosity::Silent), "");
    /// assert_eq!(failure.to_string_with_verbosity(&Verbosity::Quiet), "fail");
    /// assert_eq!(failure.to_string_with_verbosity(&Verbosity::Normal), "fail");
    /// ```
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

/// Tracks connection attempt results, summary statistics, and the configured verbosity.
///
/// `Metrics` is the primary entry point for recording and reporting on connection attempts.
/// Each call to [`Metrics::record`] appends a [`MetricsResult`] and updates the internal
/// [`MetricsSummary`]. Use [`Metrics::report`] for a one-line summary or
/// [`Metrics::full_report`] for a per-attempt listing followed by the summary.
///
/// Create a new `Metrics` with [`Metrics::new`].
///
/// # Examples
///
/// ```no_run
/// use chrono::Local;
/// use port_tester::core::metrics::{Metrics, Status};
/// use port_tester::Verbosity;
///
/// let mut m = Metrics::new(&Verbosity::Normal);
/// let dur = chrono::TimeDelta::try_milliseconds(250).unwrap();
/// m.record(1, Local::now(), dur, Status::Success);
/// m.record(2, Local::now(), dur, Status::Failure(None));
/// println!("{}", m.report());
/// ```
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Metrics {
    results: Vec<MetricsResult>,
    summary: MetricsSummary,
    verbosity: Verbosity,
}

impl Metrics {
    /// Create a new [`Metrics`] instance with the provided [`Verbosity`] level.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use port_tester::core::metrics::Metrics;
    /// use port_tester::Verbosity;
    ///
    /// let m = Metrics::new(&Verbosity::Normal);
    /// assert!(m.is_empty());
    /// ```
    pub fn new(verbose: &Verbosity) -> Self {
        Metrics {
            results: Vec::new(),
            summary: MetricsSummary::default(),
            verbosity: verbose.to_owned(),
        }
    }

    /// Returns the total number of recorded attempts.
    pub fn attempts(&self) -> u32 {
        self.summary.attempts()
    }

    /// Returns the number of successful attempts.
    pub fn success(&self) -> u32 {
        self.summary.success()
    }

    /// Returns the number of failed attempts.
    pub fn failure(&self) -> u32 {
        self.summary.failure()
    }

    /// Returns the failure rate as a percentage (0.0 – 100.0).
    pub fn failure_rate(&self) -> f64 {
        self.summary.failure_rate()
    }

    /// Returns the number of recorded results.
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Returns `true` if no results have been recorded.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Returns the configured [`Verbosity`] level.
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    /// Returns an iterator over the recorded [`MetricsResult`] entries in sequence order.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// use port_tester::core::metrics::{Metrics, Status};
    /// use port_tester::Verbosity;
    ///
    /// let mut m = Metrics::new(&Verbosity::Normal);
    /// let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();
    /// m.record(1, Local::now(), dur, Status::Success);
    /// assert_eq!(m.iter().count(), 1);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &MetricsResult> {
        self.results.iter()
    }

    /// Record a connection attempt, appending a [`MetricsResult`] and updating the
    /// [`MetricsSummary`].
    ///
    /// `seq` is the 1-based sequence number of this attempt. `timestamp` is the start time and
    /// `duration` is the time taken.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// use port_tester::core::metrics::{Metrics, Status};
    /// use port_tester::Verbosity;
    ///
    /// let mut m = Metrics::new(&Verbosity::Normal);
    /// let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();
    /// m.record(1, Local::now(), dur, Status::Success);
    /// assert_eq!(m.attempts(), 1);
    /// assert_eq!(m.success(), 1);
    /// ```
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

    /// Returns a reference to the [`MetricsResult`] for the given 1-based sequence number,
    /// or `None` if no result exists for that sequence number.
    ///
    /// Sequence numbers start at 1. Passing `0` returns the first result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// use port_tester::core::metrics::{Metrics, Status};
    /// use port_tester::Verbosity;
    ///
    /// let mut m = Metrics::new(&Verbosity::Normal);
    /// let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();
    /// m.record(1, Local::now(), dur, Status::Success);
    /// assert!(m.result(1).is_some());
    /// assert!(m.result(99).is_none());
    /// ```
    pub fn result(&self, seq: u32) -> Option<&MetricsResult> {
        let i = match seq {
            0 => 0,
            _ => seq - 1,
        };
        self.results.get(i as usize)
    }

    /// Returns a single-line summary report from the internal [`MetricsSummary`].
    ///
    /// Output format: `"attempts: N, success: N, fail: N, failure rate: N.NN%"`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// use port_tester::core::metrics::{Metrics, Status};
    /// use port_tester::Verbosity;
    ///
    /// let mut m = Metrics::new(&Verbosity::Normal);
    /// let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();
    /// m.record(1, Local::now(), dur, Status::Success);
    /// println!("{}", m.report());
    /// // Output: attempts: 1, success: 1, fail: 0, failure rate: 0.00%
    /// ```
    pub fn report(&self) -> String {
        self.summary.report()
    }

    /// Returns a multi-line report containing each recorded result followed by the summary.
    ///
    /// Each result is formatted using the stored [`Verbosity`] level. Results are separated
    /// from the summary by a blank line.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// use port_tester::core::metrics::{Metrics, Status};
    /// use port_tester::Verbosity;
    ///
    /// let mut m = Metrics::new(&Verbosity::Normal);
    /// let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();
    /// m.record(1, Local::now(), dur, Status::Success);
    /// m.record(2, Local::now(), dur, Status::Failure(None));
    /// println!("{}", m.full_report());
    /// // Output:
    /// // 1 ok
    /// // 2 fail
    /// //
    /// // attempts: 2, success: 1, fail: 1, failure rate: 50.00%
    /// ```
    pub fn full_report(&self) -> String {
        let mut report = String::new();
        for r in &self.results {
            // There's no reason why writeln! should fail to write to String so swallow the Result.
            let _ = writeln!(report, "{}", r.to_string_with_verbosity(&self.verbosity));
        }

        // Write an empty line as a separator.
        let _ = writeln!(report);
        let _ = writeln!(report, "{}", self.report());
        report
    }
}

/// Stores the metrics for a single connection attempt.
///
/// `MetricsResult` is created automatically by [`Metrics::record`] and is not typically
/// constructed directly. Access recorded results via [`Metrics::iter`] or [`Metrics::result`].
///
/// # Examples
///
/// ```no_run
/// use chrono::Local;
/// use port_tester::core::metrics::{MetricsResult, Status};
///
/// let dur = chrono::TimeDelta::try_milliseconds(150).unwrap();
/// let mr = MetricsResult::new(1, Local::now(), dur, Status::Success);
/// assert!(!mr.is_err());
/// assert_eq!(mr.seq(), 1);
/// ```
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct MetricsResult {
    seq: u32,
    timestamp: chrono::DateTime<Local>,
    duration: chrono::TimeDelta,
    status: Status,
}

/// Formats using [`Verbosity::Normal`]. Use [`MetricsResult::to_string_with_verbosity`] to
/// control output detail.
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
    /// Create a new [`MetricsResult`] for the given sequence number, timestamp, duration,
    /// and status.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// use port_tester::core::metrics::{MetricsResult, Status};
    ///
    /// let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();
    /// let mr = MetricsResult::new(1, Local::now(), dur, Status::Success);
    /// assert_eq!(mr.seq(), 1);
    /// ```
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

    /// Returns the 1-based sequence number of this attempt.
    pub fn seq(&self) -> u32 {
        self.seq
    }

    /// Returns the timestamp when this attempt started.
    pub fn timestamp(&self) -> chrono::DateTime<Local> {
        self.timestamp
    }

    /// Returns the duration of this attempt.
    pub fn duration(&self) -> chrono::TimeDelta {
        self.duration
    }

    /// Returns a reference to the [`Status`] of this attempt.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Returns `true` if this result's status represents a failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// use port_tester::core::metrics::{MetricsResult, Status};
    ///
    /// let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();
    /// let mr = MetricsResult::new(1, Local::now(), dur, Status::Failure(None));
    /// assert!(mr.is_err());
    /// ```
    pub fn is_err(&self) -> bool {
        match self.status {
            Status::Success => false,
            Status::Failure(_) => true,
        }
    }

    /// Returns the string representation of this result for the given [`Verbosity`] level.
    ///
    /// Output varies by verbosity:
    /// - [`Verbosity::Silent`]: empty string
    /// - [`Verbosity::Quiet`] and [`Verbosity::Normal`]: `"<seq> <status>"`
    /// - [`Verbosity::Verbose(0)`]: same as [`Verbosity::Normal`]
    /// - [`Verbosity::Verbose(1)`]: `"<seq> <duration>ms <status>"`
    /// - [`Verbosity::Verbose(2)`]: `"<timestamp> <seq> <duration>ms <status>"`
    /// - [`Verbosity::Verbose(3+)`]: `"start=<timestamp> seq=<seq> dur=<duration>ms status=<status>"`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chrono::Local;
    /// use port_tester::core::metrics::{MetricsResult, Status};
    /// use port_tester::Verbosity;
    ///
    /// let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();
    /// let mr = MetricsResult::new(1, Local::now(), dur, Status::Success);
    /// assert_eq!(mr.to_string_with_verbosity(&Verbosity::Normal), "1 ok");
    /// assert_eq!(mr.to_string_with_verbosity(&Verbosity::Silent), "");
    /// assert_eq!(mr.to_string_with_verbosity(&Verbosity::Verbose(1)), "1 100ms ok");
    /// ```
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
/// Create a new `MetricsSummary` with [`MetricsSummary::default`].
/// Record an attempt with [`MetricsSummary::record`].
/// Generate a report with [`MetricsSummary::report`].
///
/// # Examples
///
/// ```no_run
/// use port_tester::core::metrics::{MetricsSummary, Status};
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
    /// Returns the total number of recorded attempts.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Returns the number of successful attempts.
    pub fn success(&self) -> u32 {
        self.success
    }

    /// Returns the number of failed attempts.
    pub fn failure(&self) -> u32 {
        self.failure
    }

    /// Returns the failure rate as a percentage (0.0 – 100.0).
    ///
    /// Returns `0.0` when no attempts have been recorded.
    pub fn failure_rate(&self) -> f64 {
        if self.attempts > 0 {
            (self.failure as f64 / self.attempts as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Record a connection attempt, incrementing `success` or `failure` accordingly.
    ///
    /// [`Status::Success`] increments the success counter. [`Status::Failure`] increments the
    /// failure counter. Both increment the total attempts counter.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use port_tester::core::metrics::{MetricsSummary, Status};
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

    /// Returns a single-line summary of the collected metrics.
    ///
    /// Output format: `"attempts: N, success: N, fail: N, failure rate: N.NN%"`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use port_tester::core::metrics::{MetricsSummary, Status};
    ///
    /// let mut ms = MetricsSummary::default();
    /// ms.record(&Status::Success);
    /// ms.record(&Status::Success);
    /// println!("{}", ms.report());
    /// ```
    /// Output: `attempts: 2, success: 2, fail: 0, failure rate: 0.00%`
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
