//! IP address and CIDR utilities

use std::net::IpAddr;

/// Normalize an IP address string, handling shorthand formats
///
/// Supports:
/// - Standard IPv4: "192.168.1.1"
/// - Standard IPv6: "`2001:db8::1`"
/// - Shorthand IPv4: "1.1" -> "1.0.0.1", "1.2.3" -> "1.2.0.3"
/// - CIDR notation: "8.8.8.0/24" (returns as-is for CIDR queries)
pub fn normalize_ip(input: &str) -> Option<String> {
    let input = input.trim();

    // Handle CIDR notation - return as-is if it's a valid CIDR
    if input.contains('/') {
        // Validate CIDR format
        if let Some((ip_part, prefix_part)) = input.split_once('/')
            // Prefix must be plain digits (`parse` would also accept `+8`)
            && !prefix_part.is_empty()
            && prefix_part.bytes().all(|b| b.is_ascii_digit())
            && let Ok(prefix) = prefix_part.parse::<u8>()
            && let Some(normalized_ip) = normalize_ip(ip_part)
        {
            let max_prefix = if normalized_ip.contains(':') { 128 } else { 32 };
            if prefix <= max_prefix {
                return Some(format!("{normalized_ip}/{prefix}"));
            }
        }
        return None;
    }

    // Handle IPv6 - return as-is if valid
    if input.contains(':') {
        if input.parse::<IpAddr>().is_ok() {
            return Some(input.to_string());
        }
        return None;
    }

    // Handle IPv4 (including shorthand)
    let parts: Vec<&str> = input.split('.').collect();

    // Validate all parts are valid numbers (0-255)
    let mut nums: Vec<u8> = Vec::new();
    for part in &parts {
        match part.parse::<u64>() {
            Ok(n) if n <= 255 => nums.push(n as u8),
            Ok(n) if parts.len() == 1 && u32::try_from(n).is_ok() => {
                // Single large number - convert to IP
                // e.g., 16843009 -> 1.1.1.1
                let n = n as u32;
                return Some(format!(
                    "{}.{}.{}.{}",
                    (n >> 24) & 0xFF,
                    (n >> 16) & 0xFF,
                    (n >> 8) & 0xFF,
                    n & 0xFF
                ));
            }
            _ => return None,
        }
    }

    // Expand shorthand to full IPv4
    match nums.len() {
        1 => Some(format!("0.0.0.{}", nums[0])),
        2 => Some(format!("{}.0.0.{}", nums[0], nums[1])),
        3 => Some(format!("{}.{}.0.{}", nums[0], nums[1], nums[2])),
        4 => Some(format!("{}.{}.{}.{}", nums[0], nums[1], nums[2], nums[3])),
        _ => None,
    }
}

/// Check if a string looks like an IP address or CIDR
pub fn is_ip_like(input: &str) -> bool {
    let input = input.trim();

    // Contains colon -> likely IPv6
    if input.contains(':') {
        return true;
    }

    // Check for CIDR notation
    let ip_part = if let Some((ip, _)) = input.split_once('/') {
        ip
    } else {
        input
    };

    // All characters are digits or dots -> likely IPv4
    ip_part.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Check if the input is a CIDR notation
pub fn is_cidr(input: &str) -> bool {
    if let Some((ip_part, prefix_part)) = input.split_once('/') {
        // Check prefix is valid
        if let Ok(prefix) = prefix_part.parse::<u8>() {
            // Normalize and validate IP
            if let Some(normalized) = normalize_ip(ip_part) {
                let max_prefix = if normalized.contains(':') { 128 } else { 32 };
                return prefix <= max_prefix;
            }
        }
    }
    false
}

/// Whether `ip` is a publicly routable unicast address.
///
/// Rejects loopback, private (RFC 1918 / ULA), link-local (incl. cloud
/// metadata 169.254.169.254), CGNAT, unspecified, broadcast, multicast,
/// documentation and reserved ranges.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || a == 0
                || a >= 240
                || (a == 100 && (64..128).contains(&b))
                || (a == 198 && (b == 18 || b == 19)))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00 // unique local fc00::/7
                || (first & 0xffc0) == 0xfe80 // link-local fe80::/10
                || (first == 0x2001 && v6.segments()[1] == 0x0db8)) // documentation
        }
    }
}

/// Extract the IP part from a CIDR notation, or return as-is if not CIDR
pub fn extract_ip_from_cidr(input: &str) -> String {
    if let Some((ip_part, _)) = input.split_once('/') {
        normalize_ip(ip_part).unwrap_or_else(|| ip_part.to_string())
    } else {
        normalize_ip(input).unwrap_or_else(|| input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_public_ip() {
        for ip in ["1.1.1.1", "8.8.8.8", "2606:4700::1111"] {
            assert!(is_public_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "::",
            "fd00:ec2::254",
            "fe80::1",
            "::ffff:127.0.0.1",
            "2001:db8::1",
        ] {
            assert!(!is_public_ip(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn test_normalize_standard_ipv4() {
        assert_eq!(normalize_ip("192.168.1.1"), Some("192.168.1.1".to_string()));
        assert_eq!(normalize_ip("8.8.8.8"), Some("8.8.8.8".to_string()));
    }

    #[test]
    fn test_normalize_shorthand_ipv4() {
        assert_eq!(normalize_ip("1.1"), Some("1.0.0.1".to_string()));
        assert_eq!(normalize_ip("1.2.3"), Some("1.2.0.3".to_string()));
        assert_eq!(normalize_ip("1"), Some("0.0.0.1".to_string()));
    }

    #[test]
    fn test_normalize_cidr() {
        assert_eq!(normalize_ip("8.8.8.0/24"), Some("8.8.8.0/24".to_string()));
        assert_eq!(normalize_ip("1.1/16"), Some("1.0.0.1/16".to_string()));
    }

    #[test]
    fn test_normalize_ipv6() {
        assert_eq!(normalize_ip("2001:db8::1"), Some("2001:db8::1".to_string()));
        assert_eq!(normalize_ip("::1"), Some("::1".to_string()));
    }

    #[test]
    fn test_is_ip_like() {
        assert!(is_ip_like("192.168.1.1"));
        assert!(is_ip_like("1.1"));
        assert!(is_ip_like("8.8.8.0/24"));
        assert!(is_ip_like("2001:db8::1"));
        assert!(!is_ip_like("example.com"));
        assert!(!is_ip_like("google"));
    }

    #[test]
    fn test_is_cidr() {
        assert!(is_cidr("8.8.8.0/24"));
        assert!(is_cidr("1.1/16"));
        assert!(is_cidr("2001:db8::/32"));
        assert!(!is_cidr("8.8.8.8"));
        assert!(!is_cidr("example.com"));
    }
}
