//! The address a request really comes from.
//!
//! Behind Cloudflare every TCP connection comes from a Cloudflare edge
//! server, and the visitor's address is in `CF-Connecting-IP`. That header
//! is only believed when the connection does come from a trusted proxy;
//! otherwise anybody reaching the server directly could pick any address and
//! dodge the rate limits.

use axum::http::HeaderMap;
use ipnet::IpNet;
use std::net::{IpAddr, Ipv6Addr};

pub fn resolve(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNet]) -> IpAddr {
    let peer = canonical(peer);
    if !trusted.iter().any(|net| net.contains(&peer)) {
        return peer;
    }
    headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<IpAddr>().ok())
        .map(canonical)
        .unwrap_or(peer)
}

/// IPv4-mapped IPv6 addresses as IPv4.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        v4 => v4,
    }
}

/// The key a client is rate limited and identified by: its IPv4 address, or
/// its IPv6 /64, since one subscriber usually gets a whole /64.
pub fn limit_key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => {
            let s = v6.segments();
            IpAddr::V6(Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::CLOUDFLARE_RANGES;

    fn trusted() -> Vec<IpNet> {
        CLOUDFLARE_RANGES.iter().map(|r| r.parse().unwrap()).collect()
    }

    fn headers(cf: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("cf-connecting-ip", cf.parse().unwrap());
        h
    }

    #[test]
    fn believes_cloudflare() {
        let ip = resolve("162.158.1.2".parse().unwrap(), &headers("203.0.113.9"), &trusted());
        assert_eq!(ip, "203.0.113.9".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn ignores_spoofed_header_from_elsewhere() {
        let ip = resolve("198.51.100.7".parse().unwrap(), &headers("203.0.113.9"), &trusted());
        assert_eq!(ip, "198.51.100.7".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn garbage_header_falls_back_to_peer() {
        let ip = resolve("162.158.1.2".parse().unwrap(), &headers("not-an-ip"), &trusted());
        assert_eq!(ip, "162.158.1.2".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn mapped_v4_and_v6_prefix() {
        let ip = resolve("::ffff:162.158.1.2".parse().unwrap(), &headers("2001:db8:1:2:3:4:5:6"), &trusted());
        assert_eq!(limit_key(ip), "2001:db8:1:2::".parse::<IpAddr>().unwrap());
    }
}
