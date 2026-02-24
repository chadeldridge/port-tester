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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_connect_success() {
        let r = Host::new("8.8.8.8", 443);
        assert!(r.is_ok());

        let host = r.unwrap();
        let a = connect(&host, 2);
        assert!(a.unwrap());
    }

    #[test]
    fn test_connect_fail() {
        let r = Host::new("127.67.67.67", 443);
        assert!(r.is_ok());

        let host = r.unwrap();
        let a = connect(&host, 1);
        assert!(a.is_err());
    }
}
