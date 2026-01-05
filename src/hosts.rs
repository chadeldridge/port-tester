use crate::args::Args;
use crate::metrics::Metrics;
use dns_lookup::lookup_host;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
pub struct Host {
    pub args: Args,
    pub address: SocketAddr,
    pub metrics: Arc<Mutex<Metrics>>,
}

impl Default for Host {
    fn default() -> Self {
        Host {
            args: Args::default(),
            address: SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0),
            metrics: Arc::new(Mutex::new(Metrics::default())),
        }
    }
}

impl Host {
    pub fn fortmat_address(&self) -> SocketAddr {
        let ip = match self.args.host.parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(_) => {
                let ips: Vec<IpAddr> = match lookup_host(&self.args.host) {
                    Ok(ips) => ips.collect(),
                    _ => {
                        eprintln!("Failed to resolve hostname: {}", self.args.host);
                        std::process::exit(1);
                    }
                };

                if ips.is_empty() {
                    eprintln!("No IP addresses found for hostname: {}", self.args.host);
                    std::process::exit(1);
                }

                ips[0]
            }
        };

        let socket = SocketAddr::new(ip, self.args.port);
        if !self.args.quiet && !self.args.silent {
            println!(
                "Connecting to {} ({}) on {}",
                self.args.host,
                socket.ip(),
                socket.port()
            );
        }
        socket
    }

    pub fn attempt_connect(&mut self) {
        let result = TcpStream::connect_timeout(
            &self.address,
            std::time::Duration::from_secs(self.args.timeout),
        );

        match result {
            Ok(_) => {
                if !self.args.silent {
                    println!("ok");
                }

                // If in silent mode and only one attempt, exit successfully so the user knows
                // the attempt succeeded.
                if self.args.silent && self.args.count == 1 {
                    std::process::exit(0);
                }
                self.metrics.lock().unwrap().record_attempt(true);
            }
            Err(e) => {
                if !self.args.silent {
                    if self.args.quiet {
                        println!("failed");
                    } else {
                        println!("failed: {}", e);
                    };
                }

                // If in silent mode and only one attempt, exit with error code so the user knows
                // the attempt failed.
                if self.args.silent && self.args.count == 1 {
                    std::process::exit(1);
                }
                self.metrics.lock().unwrap().record_attempt(false);
            }
        }
    }
}
