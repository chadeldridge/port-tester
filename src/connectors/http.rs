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

use chrono::{DateTime, Local, TimeDelta};
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
    /// let policy = HttpSuccess::from_flags(false, &[]);
    /// assert_eq!(policy, HttpSuccess::Any);
    /// assert!(policy.accepts(200));
    /// assert!(policy.accepts(418));
    /// assert!(policy.accepts(500));
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
        let mut p = if restrict_success {
            HttpSuccess::new_success_or_redir()
        } else {
            HttpSuccess::Codes(BTreeSet::new())
        };
        p.add_codes(codes);
        p
    }

    /// Returns a policy that only accepts HTTP status codes in the 200-399 range.
    pub fn new_success_or_redir() -> Self {
        let mut set = BTreeSet::new();
        set.extend(200u16..=399);
        HttpSuccess::Codes(set)
    }

    /// Adds the given status codes to the set of accepted codes.
    ///
    /// This only ever widens the accepted set. On [`HttpSuccess::Any`] the call is a
    /// deliberate no-op, because `Any` already accepts every status code.
    pub fn add_codes(&mut self, codes: &[u16]) {
        match self {
            HttpSuccess::Codes(set) => set.extend(codes.iter().copied()),
            // `Any` already accepts every code; adding specific codes changes nothing.
            HttpSuccess::Any => {}
        }
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
    /// Scheme to request with, e.g. `http` or `https`.
    pub scheme: Scheme,
    /// Request path, e.g. `/` or `/health`.
    pub path: String,
    /// Which status codes count as success.
    pub success: HttpSuccess,
    /// Per-attempt timeout in seconds.
    pub timeout: u64,
    /// Skip TLS certificate verification (expired/invalid certs, hostname mismatch, etc.).
    pub insecure: bool,
    /// Maximum number of redirects to follow. `0` disables redirect following.
    pub max_redirects: u32,
}

/// ureq's default maximum number of redirects.
///
/// Used when the user requests redirect following (`--location`) without an explicit
/// `--max-redirs`. Mirrors ureq's own default.
pub const DEFAULT_MAX_REDIRECTS: u32 = 10;

/// Resolve the effective redirect cap from the `--location` / `--max-redirs` flags.
///
/// Following is enabled by either flag; `--max-redirs` sets the cap, and `--location`
/// alone uses [`DEFAULT_MAX_REDIRECTS`]. When neither is given, redirects are not followed
/// (`0`).
pub fn resolve_max_redirects(location: bool, max_redirs: Option<u32>) -> u32 {
    if location || max_redirs.is_some() {
        max_redirs.unwrap_or(DEFAULT_MAX_REDIRECTS)
    } else {
        0
    }
}

/// Build a reusable [`Agent`] for the given HTTP configuration.
///
/// Construct this once per run and pass it to [`connect`] so repeated attempts reuse the
/// same TLS setup and connection pool instead of rebuilding them on every attempt.
///
/// The agent is configured so that:
/// - 4xx/5xx responses are returned as responses, not errors (`http_status_as_error(false)`),
///   leaving the success/failure decision entirely to [`HttpConfig::success`];
/// - redirects are followed up to [`HttpConfig::max_redirects`] (`0`, the default, means the
///   test reports the status of the exact endpoint requested rather than wherever it
///   redirects to);
/// - TLS certificate verification is skipped when [`HttpConfig::insecure`] is set.
pub fn build_agent(cfg: &HttpConfig) -> Agent {
    let config = Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(cfg.timeout)))
        .http_status_as_error(false)
        .max_redirects(cfg.max_redirects)
        .tls_config(
            TlsConfig::builder()
                .disable_verification(cfg.insecure)
                .build(),
        )
        .build();
    config.into()
}

/// Issue an HTTP `GET` against `host` using `agent` and record the result into its metrics.
///
/// The request is addressed by hostname (`host.name()`) and the port stored on the [`Host`];
/// DNS resolution, connection, and redirect handling are delegated to `agent`. A response
/// whose status code is accepted by [`HttpConfig::success`] records a [`Status::Success`];
/// any other status, or a connection/TLS/timeout error, records a [`Status::Failure`].
///
/// # Examples
///
/// ```no_run
/// use port_tester::Host;
/// use port_tester::Scheme;
/// use port_tester::connectors::http::{build_agent, connect, HttpConfig, HttpSuccess};
///
/// let mut host = Host::new("google.com", 443).unwrap();
/// let cfg = HttpConfig {
///     scheme: Scheme::Https,
///     path: "/".to_string(),
///     success: HttpSuccess::Any,
///     timeout: 5,
///     insecure: false,
///     max_redirects: 0,
/// };
/// let agent = build_agent(&cfg);
/// connect(1, &mut host, &cfg, &agent);
/// assert!(!host.metrics().result(1).unwrap().is_err());
/// ```
pub fn connect(seq: u32, host: &mut Host, cfg: &HttpConfig, agent: &Agent) {
    let (start, dur, status) = attempt(host.name(), host.port(), cfg, agent);
    host.record(seq, start, dur, status);
}

/// Issue an HTTP `GET` and return its timing and outcome without touching a [`Host`].
///
/// This does the blocking request but records nothing, so callers can run it without
/// holding any lock and record the result separately. `host` is the bare hostname/IP and
/// `port` the target port (used to build the request URL). A response whose status code is
/// accepted by [`HttpConfig::success`] yields a [`Status::Success`]; any other status, or a
/// connection/TLS/timeout error, yields a [`Status::Failure`].
pub fn attempt(
    host: &str,
    port: u16,
    cfg: &HttpConfig,
    agent: &Agent,
) -> (DateTime<Local>, TimeDelta, Status) {
    let start = Local::now();

    let url = build_url(host, port, cfg.scheme, &cfg.path);

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
    (start, dur, status)
}

/// Builds the request URL, bracketing IPv6 literals and omitting the port when it matches
/// the scheme default.
fn build_url(host: &str, port: u16, scheme: Scheme, path: &str) -> String {
    // An IPv6 literal contains ':' and must be bracketed to form a valid URL authority.
    let auth = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    if port == scheme.default_port() {
        format!("{scheme}://{auth}{path}")
    } else {
        format!("{scheme}://{auth}:{port}{path}")
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
    fn test_new_success_or_redirect() {
        let policy = HttpSuccess::new_success_or_redir();
        assert!(policy.accepts(200));
        assert!(policy.accepts(301));
        assert!(!policy.accepts(404));
    }

    #[test]
    fn test_add_codes() {
        let mut policy = HttpSuccess::new_success_or_redir();
        policy.add_codes(&[404, 500]);
        assert!(policy.accepts(200));
        assert!(policy.accepts(301));
        assert!(policy.accepts(404));
        assert!(policy.accepts(500));
        assert!(!policy.accepts(418));
    }

    #[test]
    fn test_add_codes_on_any_is_noop() {
        let mut policy = HttpSuccess::Any;
        policy.add_codes(&[404, 500]);
        // `Any` already accepts every code, so it stays `Any` and keeps accepting all.
        assert_eq!(policy, HttpSuccess::Any);
        assert!(policy.accepts(200));
        assert!(policy.accepts(404));
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
    fn test_resolve_max_redirects() {
        // Neither flag: no following.
        assert_eq!(resolve_max_redirects(false, None), 0);
        // --location alone: ureq's default.
        assert_eq!(resolve_max_redirects(true, None), DEFAULT_MAX_REDIRECTS);
        // --max-redirs sets the cap and implies following, with or without --location.
        assert_eq!(resolve_max_redirects(false, Some(3)), 3);
        assert_eq!(resolve_max_redirects(true, Some(5)), 5);
        // An explicit --max-redirs 0 disables following even if --location is present.
        assert_eq!(resolve_max_redirects(true, Some(0)), 0);
    }

    #[test]
    fn test_build_url_brackets_ipv6() {
        assert_eq!(build_url("::1", 443, Scheme::Https, "/"), "https://[::1]/");
        assert_eq!(
            build_url("2001:db8::1", 8443, Scheme::Https, "/health"),
            "https://[2001:db8::1]:8443/health"
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
            max_redirects: 0,
        };
        let agent = build_agent(&cfg);
        connect(1, &mut host, &cfg, &agent);
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
            max_redirects: 0,
        };
        let agent = build_agent(&cfg);
        connect(1, &mut host, &cfg, &agent);
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
            max_redirects: 0,
        };
        let agent = build_agent(&cfg);
        connect(1, &mut host, &cfg, &agent);
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
            max_redirects: 0,
        };
        let agent = build_agent(&cfg);
        connect(1, &mut host, &cfg, &agent);
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
            max_redirects: 0,
        };
        let agent = build_agent(&cfg);
        connect(1, &mut host, &cfg, &agent);
        assert!(!host.metrics().result(1).unwrap().is_err());
    }

    #[test]
    #[cfg_attr(
        not(feature = "network-tests"),
        ignore = "requires network; enable with --features network-tests"
    )]
    fn test_connect_does_not_follow_redirects() {
        // Plain HTTP to google.com on port 80 responds with a 3xx redirect to https. With
        // redirect-following disabled we observe that 3xx directly; if it were followed we
        // would instead see the final 2xx, so accepting only 3xx proves no redirect occurred.
        let mut host = Host::new("google.com", 80).unwrap();
        let cfg = HttpConfig {
            scheme: Scheme::Http,
            path: "/".to_string(),
            success: HttpSuccess::Codes((300u16..=399).collect()),
            timeout: 10,
            insecure: false,
            max_redirects: 0,
        };
        let agent = build_agent(&cfg);
        connect(1, &mut host, &cfg, &agent);
        assert!(!host.metrics().result(1).unwrap().is_err());
    }

    #[test]
    #[cfg_attr(
        not(feature = "network-tests"),
        ignore = "requires network; enable with --features network-tests"
    )]
    fn test_connect_follows_redirects_when_configured() {
        // With redirects enabled, the http->https redirect from google.com:80 is followed to
        // the final 2xx, so accepting only 200 succeeds (the opposite of the no-follow test).
        let mut host = Host::new("google.com", 80).unwrap();
        let mut codes = BTreeSet::new();
        codes.insert(200u16);
        let cfg = HttpConfig {
            scheme: Scheme::Http,
            path: "/".to_string(),
            success: HttpSuccess::Codes(codes),
            timeout: 10,
            insecure: false,
            max_redirects: DEFAULT_MAX_REDIRECTS,
        };
        let agent = build_agent(&cfg);
        connect(1, &mut host, &cfg, &agent);
        assert!(!host.metrics().result(1).unwrap().is_err());
    }
}
