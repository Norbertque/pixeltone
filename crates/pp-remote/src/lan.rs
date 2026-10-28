//! Who may talk to the remote server, and which URLs a phone may hand it.

use std::net::{IpAddr, UdpSocket};

use axum::http::Uri;

/// The address other devices on the network reach this PC at (no packet is sent).
pub fn lan_address() -> Option<String> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("192.0.2.1", 9)).ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

/// IPv4-mapped IPv6 addresses (a dual-stack socket's view of an IPv4 peer) as IPv4.
pub fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        v4 => v4,
    }
}

/// Addresses on the local network or this computer: the only ones the server answers.
pub fn is_local(ip: IpAddr) -> bool {
    match canonical(ip) {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            v6.is_loopback() || first & 0xffc0 == 0xfe80 || first & 0xfe00 == 0xfc00
        }
    }
}

/// Whether `url` is plain HTTP on the phone that sent it (`peer`). A phone hands over its
/// own files, served by itself, and nothing else: the PC never fetches anything on a
/// phone's behalf from elsewhere.
pub fn served_by(url: &str, peer: IpAddr) -> bool {
    let Ok(uri) = url.parse::<Uri>() else {
        return false;
    };
    let host = uri
        .host()
        .map(|h| h.trim_start_matches('[').trim_end_matches(']'));
    uri.scheme_str() == Some("http")
        && host
            .and_then(|h| h.parse::<IpAddr>().ok())
            .is_some_and(|h| canonical(h) == canonical(peer))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn only_local_addresses_are_served() {
        for s in [
            "192.168.1.20",
            "10.0.0.5",
            "172.16.4.1",
            "127.0.0.1",
            "169.254.3.3",
            "::1",
            "fe80::1",
            "fd12::7",
            "::ffff:192.168.1.20",
        ] {
            assert!(is_local(ip(s)), "{s}");
        }
        for s in ["8.8.8.8", "172.32.0.1", "2001:db8::1", "::ffff:1.1.1.1"] {
            assert!(!is_local(ip(s)), "{s}");
        }
    }

    #[test]
    fn urls_must_come_from_the_phone_itself() {
        let phone = ip("192.168.1.20");
        assert!(served_by("http://192.168.1.20:8080/file/1?auth=x", phone));
        assert!(served_by(
            "http://192.168.1.20/file/1",
            ip("::ffff:192.168.1.20")
        ));
        assert!(!served_by("http://192.168.1.21:8080/file/1", phone));
        assert!(
            !served_by("https://192.168.1.20/file/1", phone),
            "plain HTTP only"
        );
        assert!(
            !served_by("http://phone.local/file/1", phone),
            "no names to resolve"
        );
        assert!(!served_by("file:///C:/Windows/win.ini", phone));
        assert!(!served_by("not a url", phone));
        assert!(served_by("http://[fe80::1]:9000/file/1", ip("fe80::1")));
    }
}
