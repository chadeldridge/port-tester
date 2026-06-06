use std::time::Duration;

use chrono::Local;
use ureq::Agent;

use crate::Host;
use crate::core::error::*;
use crate::core::metrics::Status;

/// Fully open and close the port and report any errors. Does not test any protocol information other
/// than the ability to establish a TCP connection to the specified port.
///
/// # Examples
///
/// ```
/// use port_tester::Host;
/// use port_tester::connectors::port_open::connect;
///
/// let r = Host::new("8.8.8.8", 443);
/// assert!(r.is_ok());
///
/// let mut host = r.unwrap();
/// connect(1, &mut host, 2);
/// let m = host.metrics();
/// let mr = m.result(1);
/// assert!(mr.is_some());
/// // Assert we did not get an error.
/// assert!(!mr.unwrap().is_err());
/// ```
pub fn connect(seq: u32, host: &mut Host, timeout: u64, success_codes: &[u16]) {
    let start = Local::now();
    let mut last_err = None;
    let mut success = false;

    // Attempt to connect to each resolved address until one succeeds.
    for addr in host.addrs() {
        let config = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(timeout)))
            .build();
        let agent: Agent = config.into();

        match agent.get(addr.to_string()).call() {
            Ok(res) => {
                if !success_codes.is_empty() && !success_codes.contains(&res.status().as_u16()) {
                    success = false;
                    last_err = Some(Error::new(SourceError::Msg(format!(
                        "status code not in accepted list: {}",
                        res.status()
                    ))));
                } else {
                    success = true;
                    break;
                }
            }
            Err(e) => last_err = Some(Error::new(SourceError::Ureq(e))),
        }
    }

    let dur = Local::now() - start;
    if success {
        host.record(seq, start, dur, Status::Success);
    } else {
        host.record(seq, start, dur, Status::new(false, last_err));
    }
}
