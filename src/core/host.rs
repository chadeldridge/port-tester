use crate::core::Metrics;
use crate::core::error::*;
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
/// Produced by [`Host::to_json`].
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug)]
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

#[derive(Debug)]
pub struct Host {
    /// Remote hostname, like example.com, which will be resolved to an ip address.
    name: String,
    /// Remote ip:port to connect to.
    addr: SocketAddr,
    pub metrics: Metrics,
}

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
    pub fn new(host: &str, port: u16) -> Result<Self> {
        let mut h = Host::default();
        let socket = to_socket(host, port)?;
        h.name = host.into();
        h.addr = socket;
        Ok(h)
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn ip(&self) -> IpAddr {
        self.addr.ip()
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

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
