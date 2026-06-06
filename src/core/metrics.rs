//! Collection and reporting of port connection metrics.
//!
//! This module provides the infrastructure for tracking the results of connection attempts.
//! It includes:
//! - [`Metrics`]: The primary container for a sequence of attempt results.
//! - [`Status`]: An enum representing success or specific failure conditions.
//! - [`MetricsSummary`]: Aggregated statistics (success rate, attempt count).

use chrono::Local;
use std::fmt::Write;

use crate::core::error::Result;
use crate::{Error, Verbosity};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Constant value to print for Success.
const STATUS_SUCCESS: &str = "ok";
/// Constant value to print for Failure.
const STATUS_FAILURE: &str = "fail";

/// Holds the status of a port open attempt.
///
/// [`Status::Failure`] wraps an optional [`Error`] for cases where the failure was caused by a
/// specific system or network error.
///
/// # Examples
///
/// ```
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
/// A default failure with no error represents an uninitialized or unknown state.
/// Successful attempts must be explicitly constructed as [`Status::Success`].
///
/// # Examples
///
/// ```
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
    /// ```
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
    /// ```
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
    /// Prefer this over the [`std::fmt::Display`] implementation when the verbosity level is known, as it always
    /// uses [`Verbosity::Normal`].
    ///
    /// # Examples
    ///
    /// ```
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

/// Serializable representation of a single connection attempt result.
///
/// Constructed via [`MetricsJSON`] from [`Metrics::to_json`]. All fields use plain types so no
/// lifetime or `Clone` bound is needed on the internal [`MetricsResult`] or [`Status`] types.
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct MetricsResultJSON {
    /// The 1-based sequence number of the attempt.
    seq: u32,
    /// RFC 3339 formatted start time.
    timestamp: String,
    /// Time taken in milliseconds.
    duration_ms: i64,
    /// String representation of the result (e.g., "ok" or "fail: connection refused").
    status: String,
}

impl MetricsResultJSON {
    /// Returns the 1-based sequence number of this attempt.
    pub fn seq(&self) -> u32 {
        self.seq
    }

    /// Returns the RFC 3339 timestamp string for when this attempt started.
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// Returns the duration of this attempt in milliseconds.
    pub fn duration_ms(&self) -> i64 {
        self.duration_ms
    }

    /// Returns the status string (`"ok"` or `"fail"` / `"fail: <error>"`).
    pub fn status(&self) -> &str {
        &self.status
    }
}

impl From<&MetricsResult> for MetricsResultJSON {
    fn from(r: &MetricsResult) -> Self {
        MetricsResultJSON {
            seq: r.seq,
            timestamp: r.timestamp.to_rfc3339(),
            duration_ms: r.duration.num_milliseconds(),
            status: r.status.to_string(),
        }
    }
}

/// Owned, serializable snapshot of all metrics for a connection session.
///
/// Produced by [`Metrics::to_json`]. Flattens the summary counters alongside the per-attempt
/// results so the JSON output is self-contained.
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct MetricsJSON {
    /// List of all individual attempt results.
    results: Vec<MetricsResultJSON>,
    /// Total count of attempts recorded.
    attempts: u32,
    /// Count of successful attempts.
    success: u32,
    /// Count of failed attempts.
    failure: u32,
    /// Calculated failure rate (0.0 - 100.0).
    failure_rate: f64,
}

impl MetricsJSON {
    /// Returns a slice of the per-attempt results.
    pub fn results(&self) -> &[MetricsResultJSON] {
        &self.results
    }

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
    pub fn failure_rate(&self) -> f64 {
        self.failure_rate
    }

    /// Serializes the current metrics to a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if serialization fails.
    #[cfg(feature = "serde")]
    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string(&self).map_err(|e| Error::new(crate::SourceError::SerdeJson(e)))
    }
}

/// Tracks connection attempt results, summary statistics, and the configured verbosity.
///
/// `Metrics` is used for recording and reporting on connection attempts.
/// Each call to [`Metrics::record`] appends a [`MetricsResult`] and updates the internal
/// [`MetricsSummary`].
///
/// Use [`Metrics::report`] for a one-line summary string or [`Metrics::full_report`]
/// for a detailed multi-line listing.
///
/// # Examples
///
/// ```
/// use chrono::Local;
/// use port_tester::core::metrics::{Metrics, Status, MetricsSummary};
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
    /// ```
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
    /// ```
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
    /// ```
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
    /// or `None` if the sequence number is 0 or no result exists for that sequence number.
    ///
    /// Sequence numbers are 1-based, following conventions similar to `ping`.
    ///
    /// # Examples
    ///
    /// ```
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
        seq.checked_sub(1)
            .and_then(|i| self.results.get(i as usize))
    }

    /// Returns a single-line summary report from the internal [`MetricsSummary`].
    ///
    /// Output format: `"attempts: N, success: N, fail: N, failure rate: N.NN%"`
    ///
    /// # Examples
    ///
    /// ```
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
    /// ```
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

    /// Returns an owned [`MetricsJSON`] snapshot of the current metrics state.
    pub fn to_json(&self) -> MetricsJSON {
        MetricsJSON {
            results: self.results.iter().map(MetricsResultJSON::from).collect(),
            attempts: self.summary.attempts,
            success: self.summary.success,
            failure: self.summary.failure,
            failure_rate: self.summary.failure_rate(),
        }
    }

    /// Serializes the current metrics to a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if JSON serialization fails.
    #[cfg(feature = "serde")]
    pub fn to_json_string(&self) -> Result<String> {
        self.to_json().to_json_string()
    }
}

/// Stores the metrics for a single connection attempt.
///
/// `MetricsResult` is created automatically by [`Metrics::record`] and is not typically
/// constructed directly. Access recorded results via [`Metrics::iter`] or [`Metrics::result`].
///
/// # Examples
///
/// ```
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
    /// ```
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
    /// ```
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
    /// - `Verbose(0)`: same as [`Verbosity::Normal`]
    /// - `Verbose(1)`: `"<seq> <duration>ms <status>"`
    /// - `Verbose(2)`: `"<timestamp> <seq> <duration>ms <status>"`
    /// - `Verbose(3+)`: `"start=<timestamp> seq=<seq> dur=<duration>ms status=<status>"`
    ///
    /// # Examples
    ///
    /// ```
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
                    "{} {}ms {}",
                    self.seq,
                    self.duration.num_milliseconds(),
                    self.status.to_string_with_verbosity(verbosity)
                ),
                2 => format!(
                    "{} {} {}ms {}",
                    self.timestamp,
                    self.seq,
                    self.duration.num_milliseconds(),
                    self.status.to_string_with_verbosity(verbosity)
                ),
                _ => format!(
                    "start={} seq={} dur={}ms status={}",
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
/// ```
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
    /// ```
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
    /// ```
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
        let dur = chrono::TimeDelta::try_milliseconds(1234).unwrap();
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
    #[cfg(feature = "serde")]
    fn test_metricsresultjson() {
        let dur = chrono::TimeDelta::try_milliseconds(1234).unwrap();
        let start = Local::now() - dur;
        let mr = MetricsResult::new(1, start, dur, Status::Success);
        let mr_json = MetricsResultJSON::from(&mr);
        assert_eq!(mr_json.seq(), 1);
        assert_eq!(mr_json.timestamp(), start.to_rfc3339());
        assert_eq!(mr_json.duration_ms(), 1234);
        assert_eq!(mr_json.status(), "ok");
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
        let dur = chrono::TimeDelta::try_milliseconds(1234).unwrap();
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

    #[test]
    #[cfg(feature = "serde")]
    fn test_metricsjson() {
        let mut m = Metrics::new(&Verbosity::Normal);
        assert_eq!(m.verbosity(), Verbosity::Normal);
        let dur = chrono::TimeDelta::try_milliseconds(1234).unwrap();
        m.record(1, Local::now() - dur, dur, Status::Success);
        m.record(
            2,
            Local::now() - dur,
            dur,
            Status::Failure(Some(Error::new(SourceError::from("test error")))),
        );
        let m_json = m.to_json();
        assert_eq!(m_json.attempts(), 2);
        assert_eq!(m_json.success(), 1);
        assert_eq!(m_json.failure(), 1);
        assert_eq!(m_json.failure_rate(), 50.00);

        // Test pulling back a result.
        let m_string = m_json.to_json_string();
        assert!(m_string.is_ok());
        assert_ne!(m_string.unwrap(), "".to_string());
    }
}
