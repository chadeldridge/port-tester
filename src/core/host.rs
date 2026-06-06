//! Host management and address resolution.
//!
//! This module provides the [`Host`] struct, which represents a target for port testing.
//! It handles DNS resolution of hostnames to [`SocketAddr`] and maintains the
//! [`Metrics`] associated with connection attempts to that host.

use crate::core::error::*;
use crate::core::metrics::Metrics;
use crate::core::metrics::MetricsJSON;
use crate::core::metrics::Status;
use chrono::Local;
use dns_lookup::lookup_host;
use std::net::IpAddr;
use std::net::SocketAddr;

#[cfg(feature = "serde")]
use serde::Serialize;

/// Owned, serializable snapshot of a [`Host`] and its metrics.
///
/// Produced by [`Host::to_json`]. Use this for stable JSON serialization.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HostJSON {
    name: String,
    addr: SocketAddr,
    metrics: MetricsJSON,
}

impl HostJSON {
    /// Serializes this host and its metrics to a JSON string.
    #[cfg(feature = "serde")]
    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string(&self).map_err(|e| Error::new(crate::SourceError::SerdeJson(e)))
    }
}

/// Tracks connection information and metrics for port connection attempts to the user specified
/// host.
///
/// `Host` combines a target identity (hostname and address) with a [`Metrics`] instance
/// that tracks the history of connection attempts.
///
/// # Examples
///
/// ```no_run
/// use port_tester::core::host::Host;
///
/// let mut host = Host::new("example.com", 443).expect("Failed to resolve host");
/// assert_eq!(host.port(), 443);
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub struct Host {
    /// Remote hostname, like example.com, which will be resolved to an ip address.
    name: String,
    /// Remote ip:port to connect to.
    addr: SocketAddr,
    /// Internal metrics storage for connection attempts.
    metrics: Metrics,
}

/// Defaults to an unspecified IPv4 address (0.0.0.0) on port 0 with default metrics.
impl Default for Host {
    fn default() -> Self {
        let ip = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);
        Host {
            name: "".to_string(),
            addr: SocketAddr::new(ip, 0),
            metrics: Metrics::default(),
        }
    }
}

impl Host {
    /// Create a new [`Host`] by resolving the provided hostname and port.
    ///
    /// This performs a DNS lookup immediately. If the hostname cannot be resolved,
    /// an [`Error`] is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if the host string is invalid or if DNS resolution fails.
    pub fn new(host: &str, port: u16) -> Result<Self> {
        let addr = to_socket(host, port)?;
        Ok(Host {
            name: host.to_owned(),
            addr,
            metrics: Metrics::default(),
        })
    }

    /// Returns the original hostname or IP string provided during creation.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the resolved [`IpAddr`].
    pub fn ip(&self) -> IpAddr {
        self.addr.ip()
    }

    /// Returns the target port.
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Returns a reference to the resolved [`SocketAddr`].
    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    /// Returns a reference to the internal [`Metrics`].
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Returns a mutable reference to the internal [`Metrics`].
    pub fn metrics_mut(&mut self) -> &mut Metrics {
        &mut self.metrics
    }

    /// Record a connection attempt result into the host's metrics.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use port_tester::core::host::Host;
    /// # use port_tester::core::metrics::Status;
    /// # use chrono::Local;
    /// # let mut host = Host::new("127.0.0.1", 80).unwrap();
    /// let start = Local::now();
    /// let duration = chrono::TimeDelta::try_milliseconds(10).unwrap();
    /// host.record(1, start, duration, Status::Success);
    /// assert_eq!(host.metrics().attempts(), 1);
    /// ```
    pub fn record(
        &mut self,
        seq: u32,
        timestamp: chrono::DateTime<Local>,
        duration: chrono::TimeDelta,
        status: Status,
    ) {
        self.metrics.record(seq, timestamp, duration, status);
    }

    /// Returns an owned [`HostJSON`] snapshot of this host and its current metrics.
    /// This is useful for capturing state before serialization.
    pub fn to_json(&self) -> Result<HostJSON> {
        Ok(HostJSON {
            name: self.name.clone(),
            addr: self.addr,
            metrics: self.metrics.to_json(),
        })
    }

    /// Serializes this host and its metrics to a JSON string.
    #[cfg(feature = "serde")]
    pub fn to_json_string(&self) -> Result<String> {
        self.to_json()?.to_json_string()
    }
}

/// Resolves a hostname or IP string and a port into a [`SocketAddr`].
///
/// If `host` is a valid IP address, it is used directly. Otherwise, a DNS lookup
/// is performed and the first resolved address is used.
///
/// # Errors
///
/// Returns an error if DNS lookup fails or if no IP addresses are found for the hostname.
pub fn to_socket(host: &str, port: u16) -> Result<SocketAddr> {
    let ip = match host.parse::<IpAddr>() {
        Ok(ip) => ip,
        Err(_) => {
            let ips: Vec<IpAddr> = match lookup_host(host) {
                Ok(ips) => ips.collect(),
                Err(e) => {
                    return Err(Error::new(SourceError::Io(e))
                        .set_context(&format!("Hostname lookup failed: {}", host))
                        .set_code(CODE_OPTIONS_ERROR));
                }
            };

            if ips.is_empty() {
                return Err(Error::new(SourceError::Msg(format!(
                    "No IP addresses found for hostname: {}",
                    host
                )))
                .set_code(CODE_RUNTIME_ERROR));
            }

            ips[0]
        }
    };

    Ok(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default() {
        let d = Host::default();
        assert_eq!(d.name, "".to_string());
        assert_eq!(
            d.addr,
            SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0)
        );
        assert_eq!(d.metrics.attempts(), 0);
    }

    #[test]
    fn test_new() {
        let r = Host::new("8.8.8.8", 80);
        assert!(r.is_ok());

        let d = r.unwrap();
        assert_eq!(d.name, "8.8.8.8".to_string());
        assert_eq!(
            d.addr,
            SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)), 80)
        );
        assert_eq!(d.metrics.attempts(), 0);
    }

    #[test]
    fn test_new_hostname() {
        // localhost is generally safe to resolve in test environments.
        let r = Host::new("localhost", 80);
        assert!(r.is_ok());
        let h = r.unwrap();
        assert_eq!(h.name(), "localhost");
        assert!(h.ip().is_loopback());
        assert_eq!(h.port(), 80);
    }

    #[test]
    fn test_new_invalid_host() {
        // Test a hostname that is highly unlikely to resolve.
        let r = Host::new("this.is.an.invalid.hostname.example.com", 80);
        assert!(r.is_err());
    }

    #[test]
    fn test_getters() {
        let h = Host::new("127.0.0.1", 443).unwrap();
        assert_eq!(h.name(), "127.0.0.1");
        assert_eq!(h.port(), 443);
        assert_eq!(h.ip(), IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(h.addr().port(), 443);
    }

    #[test]
    fn test_record() {
        let mut h = Host::new("127.0.0.1", 80).unwrap();
        let start = Local::now();
        let dur = chrono::TimeDelta::try_milliseconds(100).unwrap();

        h.record(1, start, dur, Status::Success);
        assert_eq!(h.metrics().attempts(), 1);
        assert_eq!(h.metrics().success(), 1);

        h.record(2, start, dur, Status::Failure(None));
        assert_eq!(h.metrics().attempts(), 2);
        assert_eq!(h.metrics().failure(), 1);
    }

    #[test]
    fn test_metrics_mut() {
        let mut h = Host::new("127.0.0.1", 80).unwrap();
        {
            let m = h.metrics_mut();
            m.record(1, Local::now(), chrono::TimeDelta::zero(), Status::Success);
        }
        assert_eq!(h.metrics().attempts(), 1);
    }

    #[test]
    fn test_to_socket_logic() {
        // Test IP parsing path
        let addr = to_socket("1.1.1.1", 53).unwrap();
        assert_eq!(addr.ip(), IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)));
        assert_eq!(addr.port(), 53);

        // Test DNS lookup path
        let addr = to_socket("localhost", 80);
        assert!(addr.is_ok());

        // Test invalid IP
        let addr = to_socket("999.999.999.999", 80);
        assert!(addr.is_err());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_hostjson() {
        let r = Host::new("8.8.8.8", 80);
        assert!(r.is_ok());

        let h = r.unwrap();
        let h_json_res = h.to_json();
        assert!(h_json_res.is_ok());
        let h_json = h_json_res.unwrap();
        assert_eq!(h_json.name, "8.8.8.8".to_string());
        assert_eq!(
            h_json.addr,
            SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)), 80)
        );
        assert_eq!(h_json.metrics.attempts(), 0);
        let h_json_string = h_json.to_json_string();
        assert!(h_json_string.is_ok());
        assert_ne!(h_json_string.unwrap(), "".to_string());
    }
}
