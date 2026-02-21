use crate::Host;
use crate::core::error::*;

use std::net::TcpStream;

// Fully open and close the port and report any errors. Does not test any protocol information other
// than the ability to establish a TCP connection to the specified port.
pub fn connect(host: &Host, timeout: u64) -> Result<bool> {
    let result = TcpStream::connect_timeout(host.addr(), std::time::Duration::from_secs(timeout));

    match result {
        Ok(_) => {
            host.metrics.lock().unwrap().record(true);
            Ok(true)
        }
        Err(e) => {
            host.metrics.lock().unwrap().record(false);
            Err(Error::new(SourceError::Io(e)))
        }
    }
}
