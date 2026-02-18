use crate::Metrics;
use crate::core::error::*;
use dns_lookup::lookup_host;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Host {
    /// Remote hostname, like example.com, which will be resolved to an ip address.
    name: String,
    /// Remote ip:port to connect to.
    addr: SocketAddr,
    pub metrics: Arc<Mutex<Metrics>>,
}

impl Default for Host {
    fn default() -> Self {
        let ip = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);
        Host {
            name: "".to_string(),
            addr: SocketAddr::new(ip, 0),
            metrics: Arc::new(Mutex::new(Metrics::default())),
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
