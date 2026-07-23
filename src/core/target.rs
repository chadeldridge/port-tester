//! Parsing of user-supplied targets into scheme, host, port, and path components.
//!
//! The `host` argument accepted by the binaries may be a bare hostname or IP address, or
//! a URL that includes any combination of a scheme prefix (`http://` / `https://`), an
//! embedded `:port`, and a `/path`. [`Target::parse`] splits these apart so the binaries
//! can decide whether to run an HTTP test and how to address it.

use crate::core::error::*;
use hostname_validator::is_valid as is_valid_hostname;
use std::net::IpAddr;

/// The URL scheme used for an HTTP test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    /// Returns the lowercase scheme string, e.g. `"https"`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Scheme::Http => "http",
            Scheme::Https => "https",
        }
    }

    /// Returns the default port for the scheme (`80` for http, `443` for https).
    pub fn default_port(&self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A parsed target broken into its scheme, host, port, and path components.
///
/// Fields are private; use the getters and the `resolve_*` helpers to derive the values
/// needed for a connection attempt.
///
/// # Examples
///
/// ```
/// use port_tester::core::target::{Scheme, Target};
///
/// let t = Target::parse("https://example.com:8443/health").unwrap();
/// assert_eq!(t.scheme(), Some(Scheme::Https));
/// assert_eq!(t.host(), "example.com");
/// assert_eq!(t.port(), Some(8443));
/// assert_eq!(t.path(), "/health");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Target {
    scheme: Option<Scheme>,
    host: String,
    port: Option<u16>,
    path: String,
}

impl Target {
    /// Parse a raw target string into its components.
    ///
    /// The path defaults to `/` when the input does not contain one. The host is validated
    /// as either an IP address or a valid hostname.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is empty, the host is malformed, or an embedded port
    /// is not a valid non-zero `u16`.
    pub fn parse(input: &str) -> Result<Target> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(invalid("Host cannot be empty"));
        }

        // Detect and strip a scheme prefix (case-insensitive) while preserving the
        // original casing of the remainder.
        let (scheme, rest) = split_scheme(trimmed);

        // Split the authority from the path on the first '/'. The path keeps its leading
        // slash and defaults to "/" when absent.
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };

        if authority.is_empty() {
            return Err(invalid("Host cannot be empty"));
        }

        let (host, port) = split_host_port(authority)?;

        if host.is_empty() {
            return Err(invalid("Host cannot be empty"));
        }

        if host.parse::<IpAddr>().is_err() && !is_valid_hostname(host) {
            return Err(invalid(&format!("Invalid host format: {host}")));
        }

        Ok(Target {
            scheme,
            host: host.to_string(),
            port,
            path: path.to_string(),
        })
    }

    /// Returns the explicit scheme from the input, if one was present.
    pub fn scheme(&self) -> Option<Scheme> {
        self.scheme
    }

    /// Returns the bare host (hostname or IP) with any scheme, port, and path removed.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the port embedded in the input, if one was present.
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns the request path, defaulting to `/`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Resolve the effective [`Scheme`] for an HTTP test.
    ///
    /// Priority: `force_https` > explicit prefix > an explicitly-provided common HTTPS
    /// port (443 or an alternate `*443` port below 10000 such as 8443/9443) > http.
    /// `arg_port` is the port supplied via the `--port` argument, or [`None`] if it was
    /// left unset.
    pub fn resolve_scheme(&self, force_https: bool, arg_port: Option<u16>) -> Scheme {
        if force_https {
            return Scheme::Https;
        }
        if let Some(s) = self.scheme {
            return s;
        }
        match self.port.or(arg_port) {
            Some(p) if is_https_port(p) => Scheme::Https,
            _ => Scheme::Http,
        }
    }

    /// Resolve the effective port for an HTTP test.
    ///
    /// Priority: embedded port > `arg_port` > the scheme's default port.
    pub fn resolve_port(&self, scheme: Scheme, arg_port: Option<u16>) -> u16 {
        self.port
            .or(arg_port)
            .unwrap_or_else(|| scheme.default_port())
    }
}

/// Splits a leading `http://` or `https://` prefix (case-insensitive) from `input`,
/// returning the detected scheme and the remainder with its original casing intact.
fn split_scheme(input: &str) -> (Option<Scheme>, &str) {
    if let Some(rest) = strip_prefix_ci(input, "https://") {
        (Some(Scheme::Https), rest)
    } else if let Some(rest) = strip_prefix_ci(input, "http://") {
        (Some(Scheme::Http), rest)
    } else {
        (None, input)
    }
}

/// Case-insensitively strips an ASCII `prefix` from `input`, returning the remainder with
/// its original casing. Returns [`None`] if `input` does not start with `prefix`.
fn strip_prefix_ci<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    input
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &input[prefix.len()..])
}

/// Splits an authority (`host`, `host:port`, or `[ipv6]:port`) into its host and optional
/// port. Unbracketed IPv6 literals are treated as a host with no port.
fn split_host_port(authority: &str) -> Result<(&str, Option<u16>)> {
    if let Some(rest) = authority.strip_prefix('[') {
        // Bracketed IPv6 literal: [addr] or [addr]:port.
        let end = rest
            .find(']')
            .ok_or_else(|| invalid("Invalid IPv6 host: missing ']'"))?;
        let host = &rest[..end];
        let after = &rest[end + 1..];
        let port = match after.strip_prefix(':') {
            Some(p) => Some(parse_port(p)?),
            None if after.is_empty() => None,
            None => return Err(invalid("Invalid characters after IPv6 host")),
        };
        return Ok((host, port));
    }

    // A bare IP address (including an unbracketed IPv6 literal) has no port component.
    if authority.parse::<IpAddr>().is_ok() {
        return Ok((authority, None));
    }

    match authority.rsplit_once(':') {
        Some((host, port)) => Ok((host, Some(parse_port(port)?))),
        None => Ok((authority, None)),
    }
}

/// Returns `true` if `port` is a common HTTPS port.
///
/// This is the standard `443` plus the alternate `*443` ports below 10000 that are
/// widely used for TLS services (e.g. 8443, 9443, 6443, 5443, 4443). It is used only as
/// a scheme hint and deliberately does not match HTTP alternates like 8080 or 8000.
fn is_https_port(port: u16) -> bool {
    port < 10_000 && port % 1000 == 443
}

/// Parses a non-zero `u16` port.
fn parse_port(s: &str) -> Result<u16> {
    s.parse::<u16>()
        .ok()
        .filter(|&p| p > 0)
        .ok_or_else(|| invalid(&format!("Invalid port: {s}")))
}

/// Builds an options error carrying `msg`.
fn invalid(msg: &str) -> Error {
    Error::new(SourceError::Msg(msg.to_string())).set_code(CODE_OPTIONS_ERROR)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_scheme_helpers() {
        assert_eq!(Scheme::Http.as_str(), "http");
        assert_eq!(Scheme::Https.as_str(), "https");
        assert_eq!(Scheme::Http.default_port(), 80);
        assert_eq!(Scheme::Https.default_port(), 443);
        assert_eq!(Scheme::Https.to_string(), "https");
    }

    #[test]
    fn test_parse_bare_host() {
        let t = Target::parse("google.com").unwrap();
        assert_eq!(t.scheme(), None);
        assert_eq!(t.host(), "google.com");
        assert_eq!(t.port(), None);
        assert_eq!(t.path(), "/");
    }

    #[test]
    fn test_parse_bare_ip() {
        let t = Target::parse("1.1.1.1").unwrap();
        assert_eq!(t.host(), "1.1.1.1");
        assert_eq!(t.port(), None);
        assert_eq!(t.path(), "/");
    }

    #[test]
    fn test_parse_http_prefix() {
        let t = Target::parse("http://google.com").unwrap();
        assert_eq!(t.scheme(), Some(Scheme::Http));
        assert_eq!(t.host(), "google.com");
        assert_eq!(t.port(), None);
        assert_eq!(t.path(), "/");
    }

    #[test]
    fn test_parse_https_prefix_case_insensitive() {
        let t = Target::parse("HTTPS://Google.com/Path").unwrap();
        assert_eq!(t.scheme(), Some(Scheme::Https));
        // Host casing is preserved from the original input.
        assert_eq!(t.host(), "Google.com");
        assert_eq!(t.path(), "/Path");
    }

    #[test]
    fn test_parse_embedded_port() {
        let t = Target::parse("google.com:8080").unwrap();
        assert_eq!(t.host(), "google.com");
        assert_eq!(t.port(), Some(8080));
        assert_eq!(t.path(), "/");
    }

    #[test]
    fn test_parse_port_and_path() {
        let t = Target::parse("http://google.com:8080/status/health").unwrap();
        assert_eq!(t.scheme(), Some(Scheme::Http));
        assert_eq!(t.host(), "google.com");
        assert_eq!(t.port(), Some(8080));
        assert_eq!(t.path(), "/status/health");
    }

    #[test]
    fn test_parse_trailing_slash_path() {
        let t = Target::parse("google.com/").unwrap();
        assert_eq!(t.host(), "google.com");
        assert_eq!(t.path(), "/");
    }

    #[test]
    fn test_parse_path_without_scheme() {
        let t = Target::parse("google.com/search").unwrap();
        assert_eq!(t.scheme(), None);
        assert_eq!(t.host(), "google.com");
        assert_eq!(t.path(), "/search");
    }

    #[test]
    fn test_parse_bracketed_ipv6_with_port() {
        let t = Target::parse("https://[2001:4860:4860::8888]:8443/dns").unwrap();
        assert_eq!(t.scheme(), Some(Scheme::Https));
        assert_eq!(t.host(), "2001:4860:4860::8888");
        assert_eq!(t.port(), Some(8443));
        assert_eq!(t.path(), "/dns");
    }

    #[test]
    fn test_parse_bracketed_ipv6_without_port() {
        let t = Target::parse("[::1]/").unwrap();
        assert_eq!(t.host(), "::1");
        assert_eq!(t.port(), None);
    }

    #[test]
    fn test_parse_bare_ipv6_no_port() {
        // An unbracketed IPv6 literal is treated as a host with no port.
        let t = Target::parse("2001:4860:4860::8888").unwrap();
        assert_eq!(t.host(), "2001:4860:4860::8888");
        assert_eq!(t.port(), None);
    }

    #[test]
    fn test_parse_errors() {
        assert!(Target::parse("").is_err());
        assert!(Target::parse("   ").is_err());
        assert!(Target::parse("http://").is_err());
        assert!(Target::parse("-bad.com").is_err());
        assert!(Target::parse("google.com:0").is_err());
        assert!(Target::parse("google.com:99999").is_err());
        assert!(Target::parse("google.com:abc").is_err());
        assert!(Target::parse("[::1").is_err());
    }

    #[test]
    fn test_parse_error_code() {
        let err = Target::parse("").unwrap_err();
        assert_eq!(err.code(), Some(CODE_OPTIONS_ERROR));
    }

    #[test]
    fn test_resolve_scheme_force_https() {
        let t = Target::parse("google.com").unwrap();
        assert_eq!(t.resolve_scheme(true, None), Scheme::Https);
        // force_https wins even over an explicit http:// prefix.
        let t = Target::parse("http://google.com").unwrap();
        assert_eq!(t.resolve_scheme(true, None), Scheme::Https);
    }

    #[test]
    fn test_resolve_scheme_prefix() {
        let t = Target::parse("http://google.com").unwrap();
        assert_eq!(t.resolve_scheme(false, None), Scheme::Http);
        let t = Target::parse("https://google.com").unwrap();
        assert_eq!(t.resolve_scheme(false, None), Scheme::Https);
    }

    #[test]
    fn test_resolve_scheme_port_indicator() {
        // Embedded port 443 implies https.
        let t = Target::parse("google.com:443").unwrap();
        assert_eq!(t.resolve_scheme(false, None), Scheme::Https);
        // --port 443 implies https.
        let t = Target::parse("google.com").unwrap();
        assert_eq!(t.resolve_scheme(false, Some(443)), Scheme::Https);
        // Port 80 implies http.
        let t = Target::parse("google.com:80").unwrap();
        assert_eq!(t.resolve_scheme(false, None), Scheme::Http);
    }

    #[test]
    fn test_resolve_scheme_alternate_https_ports() {
        // Common alternate `*443` ports below 10000 imply https, from either source.
        for port in [1443, 4443, 5443, 6443, 8443, 9443] {
            let t = Target::parse(&format!("google.com:{port}")).unwrap();
            assert_eq!(t.resolve_scheme(false, None), Scheme::Https, "port {port}");
            let t = Target::parse("google.com").unwrap();
            assert_eq!(
                t.resolve_scheme(false, Some(port)),
                Scheme::Https,
                "arg port {port}"
            );
        }
    }

    #[test]
    fn test_is_https_port() {
        assert!(is_https_port(443));
        assert!(is_https_port(8443));
        assert!(is_https_port(9443));
        assert!(is_https_port(6443));
        // Not an HTTPS-shaped port.
        assert!(!is_https_port(80));
        assert!(!is_https_port(8080));
        assert!(!is_https_port(8000));
        assert!(!is_https_port(1234));
        // `*443` at or above 10000 is not treated as an indicator.
        assert!(!is_https_port(10443));
        assert!(!is_https_port(18443));
    }

    #[test]
    fn test_resolve_scheme_default_http() {
        let t = Target::parse("google.com").unwrap();
        assert_eq!(t.resolve_scheme(false, None), Scheme::Http);
        // A non-indicator port does not force https.
        let t = Target::parse("google.com:8080").unwrap();
        assert_eq!(t.resolve_scheme(false, None), Scheme::Http);
        // An alternate `*443` port at/above 10000 is not an indicator.
        let t = Target::parse("google.com:10443").unwrap();
        assert_eq!(t.resolve_scheme(false, None), Scheme::Http);
    }

    #[test]
    fn test_resolve_port() {
        // Embedded port wins.
        let t = Target::parse("google.com:8080").unwrap();
        assert_eq!(t.resolve_port(Scheme::Https, Some(9000)), 8080);
        // Then the arg port.
        let t = Target::parse("google.com").unwrap();
        assert_eq!(t.resolve_port(Scheme::Https, Some(9000)), 9000);
        // Then the scheme default.
        let t = Target::parse("google.com").unwrap();
        assert_eq!(t.resolve_port(Scheme::Https, None), 443);
        assert_eq!(t.resolve_port(Scheme::Http, None), 80);
    }
}
