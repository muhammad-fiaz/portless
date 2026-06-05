//! Networking helpers (port allocation, browser-blocked list, etc).

use crate::common::{Error, Result};
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use tokio::net::TcpListener as TokioTcpListener;

/// Well-known ports that browsers refuse to connect to.
///
/// This is a conservative subset covering the most commonly blocked ports
/// across modern browsers (Chrome, Firefox, Safari, Edge).
pub const BROWSER_BLOCKED_PORTS: &[u16] = &[
    1, 7, 9, 11, 13, 15, 17, 19, 20, 21, 22, 23, 25, 37, 42, 43, 53, 69, 77, 79, 87, 95, 101, 102,
    103, 104, 109, 110, 111, 113, 115, 117, 119, 123, 135, 137, 139, 143, 161, 179, 389, 427, 465,
    512, 513, 514, 515, 526, 530, 531, 532, 540, 548, 554, 556, 563, 587, 601, 636, 989, 990, 993,
    995, 1719, 1720, 1723, 2049, 3659, 4045, 5060, 5061, 6000, 6566, 6665, 6666, 6667, 6668, 6669,
    6679, 6697, 10080,
];

/// Returns true if a port is in the browser-blocked list.
pub fn is_browser_blocked(port: u16) -> bool {
    BROWSER_BLOCKED_PORTS.contains(&port)
}

/// Find a free TCP port in the given (inclusive) range, skipping browser-blocked
/// ports. Returns the first free port found.
pub fn find_free_port(start: u16, end: u16) -> Result<u16> {
    if start >= end {
        return Err(Error::NoFreePort(start, end));
    }
    for port in start..end {
        if is_browser_blocked(port) {
            continue;
        }
        if is_port_free(port) {
            return Ok(port);
        }
    }
    Err(Error::NoFreePort(start, end))
}

/// Find a free port asynchronously using Tokio.
pub async fn find_free_port_async(start: u16, end: u16) -> Result<u16> {
    if start >= end {
        return Err(Error::NoFreePort(start, end));
    }
    for port in start..end {
        if is_browser_blocked(port) {
            continue;
        }
        if is_port_free_async(port).await {
            return Ok(port);
        }
    }
    Err(Error::NoFreePort(start, end))
}

/// Returns true if a port can be bound locally.
pub fn is_port_free(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Async version of `is_port_free`.
pub async fn is_port_free_async(port: u16) -> bool {
    TokioTcpListener::bind(("127.0.0.1", port)).await.is_ok()
}

/// Try to bind a specific port and return a Tokio listener (with SO_REUSEADDR).
pub async fn bind_tcp<A: ToSocketAddrs>(addr: A) -> Result<TokioTcpListener> {
    let std_listener = std::net::TcpListener::bind(addr)?;
    std_listener.set_nonblocking(true)?;
    let listener = TokioTcpListener::from_std(std_listener)?;
    Ok(listener)
}

/// Format a `SocketAddr` for log output (omits scope id for IPv6).
pub fn format_addr(addr: &SocketAddr) -> String {
    match addr {
        SocketAddr::V4(v4) => format!("{}:{}", v4.ip(), v4.port()),
        SocketAddr::V6(v6) => format!("[{}]:{}", v6.ip(), v6.port()),
    }
}

/// Detect the loopback address (always `127.0.0.1`).
pub fn loopback_v4() -> std::net::Ipv4Addr {
    std::net::Ipv4Addr::new(127, 0, 0, 1)
}

/// Detect a non-loopback IPv4 address from local interfaces.
pub fn detect_local_ipv4() -> Option<std::net::Ipv4Addr> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    match addr.ip() {
        std::net::IpAddr::V4(v4) => Some(v4),
        std::net::IpAddr::V6(_) => None,
    }
}

/// Synchronously probe whether a TCP socket is accepting connections.
///
/// Returns `Ok(true)` if the port is open, `Ok(false)` if the connection was
/// refused or timed out, and `Err(_)` for unexpected I/O errors.
pub fn tcp_probe(addr: std::net::SocketAddr, timeout: std::time::Duration) -> Result<bool> {
    use std::net::TcpStream;
    let _ = timeout;
    // A fast path: try to connect. If it succeeds, the port is open. If the
    // OS reports "connection refused" the port is closed. Anything else is an
    // error.
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_s) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(false),
        Err(e) => Err(Error::Network(format!("probe {addr}: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_blocked_contains_known() {
        assert!(is_browser_blocked(6666));
        assert!(is_browser_blocked(6667));
        assert!(!is_browser_blocked(3000));
        assert!(!is_browser_blocked(8080));
    }

    #[test]
    fn find_free_port_basic() {
        let p = find_free_port(50_000, 50_100).unwrap();
        assert!((50_000..50_100).contains(&p));
        assert!(!is_browser_blocked(p));
    }

    #[test]
    fn find_free_port_invalid_range() {
        let r = find_free_port(100, 50);
        assert!(matches!(r, Err(Error::NoFreePort(100, 50))));
    }

    #[test]
    fn loopback_addr() {
        assert_eq!(loopback_v4().to_string(), "127.0.0.1");
    }
}
