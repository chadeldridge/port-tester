use chrono::{DateTime, Local, TimeDelta};

use crate::Host;
use crate::core::error::*;
use crate::core::metrics::Status;

use std::net::{SocketAddr, TcpStream};

/// Perform a port-open attempt against `addrs` and return its timing and outcome.
///
/// This does the blocking network work but does not touch a [`Host`], so callers can run
/// it without holding any lock and record the result separately. Each address is tried in
/// order until one connects; on total failure the last error is reported.
pub fn attempt(addrs: &[SocketAddr], timeout: u64) -> (DateTime<Local>, TimeDelta, Status) {
    let start = Local::now();
    let mut last_err = None;
    let mut success = false;

    // Attempt to connect to each resolved address until one succeeds.
    for addr in addrs {
        match TcpStream::connect_timeout(addr, std::time::Duration::from_secs(timeout)) {
            Ok(_) => {
                success = true;
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }

    let dur = Local::now() - start;
    let status = if success {
        Status::Success
    } else {
        Status::new(false, last_err.map(|e| Error::new(SourceError::Io(e))))
    };
    (start, dur, status)
}

// Fully open and close the port and report any errors. Does not test any protocol information other
// than the ability to establish a TCP connection to the specified port.
pub fn connect(seq: u32, host: &mut Host, timeout: u64) {
    let (start, dur, status) = attempt(host.addrs(), timeout);
    host.record(seq, start, dur, status);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg_attr(
        not(feature = "network-tests"),
        ignore = "requires network; enable with --features network-tests"
    )]
    fn test_connect_success() {
        let r = Host::new("8.8.8.8", 443);
        assert!(r.is_ok());

        let mut host = r.unwrap();
        connect(1, &mut host, 2);
        let m = host.metrics();
        let mr = m.result(1);
        assert!(mr.is_some());
        // Assert we did not get an error.
        assert!(!mr.unwrap().is_err());
    }

    #[test]
    fn test_connect_fail() {
        let r = Host::new("127.67.67.67", 443);
        assert!(r.is_ok());

        let mut host = r.unwrap();
        connect(1, &mut host, 1);
        let m = host.metrics();
        let mr = m.result(1);
        assert!(mr.is_some());
        assert!(mr.unwrap().is_err());
    }
}
