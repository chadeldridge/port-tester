//! HTTP GET connectivity test.
//!
//! Unlike [`port_open`](crate::connectors::port_open), which only verifies that a TCP
//! connection can be established, this connector issues an HTTP `GET` and treats the
//! response (or the lack of one) as the success signal. The request is made against the
//! target's hostname so that TLS SNI, name-based virtual hosting, and the `Host` header
//! all resolve correctly. `ureq` sends a standard HTTP/1.1 request with `Host`,
//! `User-Agent`, and `Accept` headers, which is accepted by both modern and legacy servers.

use std::collections::BTreeSet;
use std::time::Duration;

use chrono::Local;
use ureq::Agent;
use ureq::tls::TlsConfig;

use crate::Host;
use crate::core::error::*;
use crate::core::metrics::Status;
use crate::core::target::Scheme;

/// Policy deciding which HTTP status codes count as a successful test.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HttpSuccess {
    /// Any completed HTTP response is a success, regardless of status code.
    #[default]
    Any,
    /// Only the listed status codes are a success.
    Codes(BTreeSet<u16>),
}

impl HttpSuccess {
    /// Build a policy from the CLI flags.
    ///
    /// `restrict_success` seeds the accepted set with the 2xx/3xx range (`200..=399`).
    /// Each code in `codes` is added to the set. Duplicate codes are dropped by the
    /// underlying [`BTreeSet`]. When neither is supplied, the policy is [`HttpSuccess::Any`].
    ///
    /// # Examples
    ///
    /// ```
    /// use port_tester::connectors::http::HttpSuccess;
    ///
    /// assert_eq!(HttpSuccess::from_flags(false, &[]), HttpSuccess::Any);
    ///
    /// let policy = HttpSuccess::from_flags(true, &[418]);
    /// assert!(policy.accepts(200));
    /// assert!(policy.accepts(418));
    /// assert!(!policy.accepts(500));
    /// ```
    pub fn from_flags(restrict_success: bool, codes: &[u16]) -> Self {
        if !restrict_success && codes.is_empty() {
            return HttpSuccess::Any;
        }
        let mut set = BTreeSet::new();
        if restrict_success {
            set.extend(200u16..=399);
        }
        set.extend(codes.iter().copied());
        HttpSuccess::Codes(set)
    }

    /// Returns `true` if `code` is accepted by this policy.
    pub fn accepts(&self, code: u16) -> bool {
        match self {
            HttpSuccess::Any => true,
            HttpSuccess::Codes(set) => set.contains(&code),
        }
    }
}

/// Configuration for a single HTTP GET test.
#[derive(Clone, Debug)]
pub struct HttpConfig {
    /// Scheme to request with.
    pub scheme: Scheme,
    /// Request path, e.g. `/` or `/health`.
    pub path: String,
    /// Which status codes count as success.
    pub success: HttpSuccess,
    /// Per-attempt timeout in seconds.
    pub timeout: u64,
    /// Skip TLS certificate verification (expired/invalid certs, hostname mismatch, etc.).
    pub insecure: bool,
}

/// Issue an HTTP `GET` against `host` and record the result into its metrics.
///
/// The request targets `host.name()` (the original hostname), using the port stored on the
/// [`Host`]. A response whose status code is accepted by [`HttpConfig::success`] records a
/// [`Status::Success`]; any other status, or a connection/TLS/timeout error, records a
/// [`Status::Failure`].
///
/// # Examples
///
/// ```no_run
/// use port_tester::Host;
/// use port_tester::Scheme;
/// use port_tester::connectors::http::{connect, HttpConfig, HttpSuccess};
///
/// let mut host = Host::new("google.com", 443).unwrap();
/// let cfg = HttpConfig {
///     scheme: Scheme::Https,
///     path: "/".to_string(),
///     success: HttpSuccess::Any,
///     timeout: 5,
///     insecure: false,
/// };
/// connect(1, &mut host, &cfg);
/// assert!(!host.metrics().result(1).unwrap().is_err());
/// ```
pub fn connect(seq: u32, host: &mut Host, cfg: &HttpConfig) {
    let start = Local::now();

    let url = build_url(host.name(), host.port(), cfg.scheme, &cfg.path);

    // Disable ureq's default behavior of turning 4xx/5xx into errors so that the success
    // policy alone decides the outcome. Any completed response then returns `Ok`.
    let config = Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(cfg.timeout)))
        .http_status_as_error(false)
        .tls_config(
            TlsConfig::builder()
                .disable_verification(cfg.insecure)
                .build(),
        )
        .build();
    let agent: Agent = config.into();

    let status = match agent.get(&url).call() {
        Ok(res) => {
            let code = res.status().as_u16();
            if cfg.success.accepts(code) {
                Status::Success
            } else {
                Status::new(
                    false,
                    Some(Error::new(SourceError::Msg(format!(
                        "unexpected HTTP status: {code}"
                    )))),
                )
            }
        }
        Err(e) => Status::new(false, Some(Error::new(SourceError::Ureq(e)))),
    };

    let dur = Local::now() - start;
    host.record(seq, start, dur, status);
}

/// Builds the request URL, omitting the port when it matches the scheme default.
fn build_url(host: &str, port: u16, scheme: Scheme, path: &str) -> String {
    if port == scheme.default_port() {
        format!("{scheme}://{host}{path}")
    } else {
        format!("{scheme}://{host}:{port}{path}")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_http_success_default_any() {
        assert_eq!(HttpSuccess::default(), HttpSuccess::Any);
        assert_eq!(HttpSuccess::from_flags(false, &[]), HttpSuccess::Any);
        assert!(HttpSuccess::Any.accepts(200));
        assert!(HttpSuccess::Any.accepts(404));
        assert!(HttpSuccess::Any.accepts(500));
    }

    #[test]
    fn test_http_success_restrict() {
        let policy = HttpSuccess::from_flags(true, &[]);
        assert!(policy.accepts(200));
        assert!(policy.accepts(301));
        assert!(policy.accepts(399));
        assert!(!policy.accepts(199));
        assert!(!policy.accepts(400));
        assert!(!policy.accepts(500));
    }

    #[test]
    fn test_http_success_codes_only() {
        let policy = HttpSuccess::from_flags(false, &[404, 500]);
        assert!(policy.accepts(404));
        assert!(policy.accepts(500));
        assert!(!policy.accepts(200));
    }

    #[test]
    fn test_http_success_union() {
        let policy = HttpSuccess::from_flags(true, &[404, 500]);
        assert!(policy.accepts(200));
        assert!(policy.accepts(404));
        assert!(policy.accepts(500));
        assert!(!policy.accepts(418));
    }

    #[test]
    fn test_http_success_dedup() {
        // 200 and 250 already fall in the 2xx/3xx range, so the set must not grow.
        let policy = HttpSuccess::from_flags(true, &[200, 250, 200]);
        match policy {
            HttpSuccess::Codes(set) => assert_eq!(set.len(), 200),
            HttpSuccess::Any => panic!("expected Codes"),
        }
    }

    #[test]
    fn test_build_url_default_port_omitted() {
        assert_eq!(
            build_url("google.com", 443, Scheme::Https, "/"),
            "https://google.com/"
        );
        assert_eq!(
            build_url("google.com", 80, Scheme::Http, "/health"),
            "http://google.com/health"
        );
    }

    #[test]
    fn test_build_url_non_default_port() {
        assert_eq!(
            build_url("google.com", 8443, Scheme::Https, "/"),
            "https://google.com:8443/"
        );
    }

    #[test]
    #[cfg_attr(
        not(feature = "network-tests"),
        ignore = "requires network; enable with --features network-tests"
    )]
    fn test_connect_success() {
        let mut host = Host::new("google.com", 443).unwrap();
        let cfg = HttpConfig {
            scheme: Scheme::Https,
            path: "/".to_string(),
            success: HttpSuccess::Any,
            timeout: 10,
            insecure: false,
        };
        connect(1, &mut host, &cfg);
        let mr = host.metrics().result(1).unwrap();
        assert!(!mr.is_err());
    }

    #[test]
    #[cfg_attr(
        not(feature = "network-tests"),
        ignore = "requires network; enable with --features network-tests"
    )]
    fn test_connect_unexpected_status_fails() {
        // google.com will not return 599, so a code-restricted policy must fail.
        let mut host = Host::new("google.com", 443).unwrap();
        let mut codes = BTreeSet::new();
        codes.insert(599u16);
        let cfg = HttpConfig {
            scheme: Scheme::Https,
            path: "/".to_string(),
            success: HttpSuccess::Codes(codes),
            timeout: 10,
            insecure: false,
        };
        connect(1, &mut host, &cfg);
        let mr = host.metrics().result(1).unwrap();
        assert!(mr.is_err());
    }

    #[test]
    fn test_connect_connection_error_fails() {
        // An unroutable address should fail to connect within the timeout.
        let mut host = Host::new("127.67.67.67", 443).unwrap();
        let cfg = HttpConfig {
            scheme: Scheme::Https,
            path: "/".to_string(),
            success: HttpSuccess::Any,
            timeout: 1,
            insecure: false,
        };
        connect(1, &mut host, &cfg);
        let mr = host.metrics().result(1).unwrap();
        assert!(mr.is_err());
    }

    #[test]
    #[cfg_attr(
        not(feature = "network-tests"),
        ignore = "requires network; enable with --features network-tests"
    )]
    fn test_connect_secure_rejects_bad_cert() {
        // A host with an expired certificate must fail when verification is enabled.
        let mut host = Host::new("expired.badssl.com", 443).unwrap();
        let cfg = HttpConfig {
            scheme: Scheme::Https,
            path: "/".to_string(),
            success: HttpSuccess::Any,
            timeout: 10,
            insecure: false,
        };
        connect(1, &mut host, &cfg);
        assert!(host.metrics().result(1).unwrap().is_err());
    }

    #[test]
    #[cfg_attr(
        not(feature = "network-tests"),
        ignore = "requires network; enable with --features network-tests"
    )]
    fn test_connect_insecure_accepts_bad_cert() {
        // The same expired-certificate host succeeds once verification is disabled.
        let mut host = Host::new("expired.badssl.com", 443).unwrap();
        let cfg = HttpConfig {
            scheme: Scheme::Https,
            path: "/".to_string(),
            success: HttpSuccess::Any,
            timeout: 10,
            insecure: true,
        };
        connect(1, &mut host, &cfg);
        assert!(!host.metrics().result(1).unwrap().is_err());
    }
}
