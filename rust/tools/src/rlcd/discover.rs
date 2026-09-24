//! RLCD-001 — scoped discovery of the port a pidfile's live pid serves on.
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! Regression (2026-09-24): a spawn/reuse can land on a port DIFFERENT from
//! the configured url (scan-skip, or a server started out-of-band) and the
//! pidfile records the pid but NOT the port. Consumers that only probe the
//! configured url then fail open while the pidfile's own server sits
//! healthy on a different loopback port. This module resolves the listen
//! ports owned by a given pid (socket inodes from `/proc/<pid>/fd` +
//! `/proc/net/tcp`, Linux) and probes each for the RLCD-ready `/health`
//! shape, returning the first healthy `(port, model)`.
//!
//! Connect-only scope: this NEVER spawns anything — it only lets the
//! supervisor re-anchor to a server that is already serving.

use std::collections::HashSet;
use std::time::Duration;

use crate::rlcd::health::health_check;

/// Per-candidate /health probe budget (short — discovery runs inside
/// bounded wait loops).
const DISCOVERY_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// The LISTEN ports on 127.0.0.1 owned by `pid` (Linux). Other platforms /
/// an unreadable `/proc/<pid>/fd`: empty set (discovery is a no-op and the
/// old bounded-wait behavior stands).
#[must_use]
pub fn socket_inodes_for_pid(pid: u32) -> HashSet<u64> {
    #[cfg(target_os = "linux")]
    {
        collect_socket_inodes(pid)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        HashSet::new()
    }
}

/// Scan `/proc/net/tcp` for 127.0.0.1 LISTEN sockets whose inode is in
/// `inodes`.
#[must_use]
pub fn loopback_listen_ports_for_inodes(content: &str, inodes: &HashSet<u64>) -> Vec<u16> {
    let mut ports = Vec::new();
    for line in content.lines().skip(1) {
        // "  sl  local_address rem_address st tx:rx tr:tm->when retrnsmt
        //  uid timeout inode ..." — 0-based fields: 0=sl, 1=local_address,
        // 2=rem_address, 3=st, 9=inode.
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (Some(local), Some(state), Some(inode_field)) =
            (fields.get(1), fields.get(3), fields.get(9))
        else {
            continue;
        };
        if *state != "0A" {
            continue; // LISTEN only
        }
        let Some(inode) = inode_field.parse::<u64>().ok() else {
            continue;
        };
        if !inodes.contains(&inode) {
            continue;
        }
        let Some((ip, port_hex)) = local.rsplit_once(':') else {
            continue;
        };
        // local_address is a little-endian 32-bit hex IP + ':' + hex port.
        // 127.0.0.1 (0x7F000001) prints as "0100007F" → bytes [1,0,0,0x7F].
        if ip_bytes_le(ip) != Some([1, 0, 0, 127]) {
            continue;
        }
        if let Ok(port) = u16::from_str_radix(port_hex, 16) {
            if port > 0 && !ports.contains(&port) {
                ports.push(port);
            }
        }
    }
    ports
}

/// The socket inodes held by `pid` (`/proc/<pid>/fd/*` → `socket:[N]`).
fn collect_socket_inodes(pid: u32) -> HashSet<u64> {
    let mut inodes = HashSet::new();
    let Ok(entries) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
        return inodes;
    };
    for entry in entries.flatten() {
        if let Ok(target) = std::fs::read_link(entry.path()) {
            if let Some(inode) = target
                .to_str()
                .and_then(|s| s.strip_prefix("socket:["))
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|s| s.parse::<u64>().ok())
            {
                inodes.insert(inode);
            }
        }
    }
    inodes
}

/// Decode a little-endian 32-bit hex IP (as printed in `/proc/net/tcp`).
fn ip_bytes_le(hex_ip: &str) -> Option<[u8; 4]> {
    let mut out = [0u8; 4];
    for (i, word) in hex_ip.as_bytes().chunks(2).enumerate() {
        if i >= 4 || word.len() != 2 {
            return None;
        }
        out[i] = u8::from_str_radix(std::str::from_utf8(word).ok()?, 16).ok()?;
    }
    Some(out)
}

/// Probe the 127.0.0.1 LISTEN ports owned by `pid` concurrently for a
/// healthy RLCD server; returns the first `(port, model)` hit. `None` when
/// nothing healthy answers (or on non-Linux platforms).
pub async fn discover_healthy_server_for_pid(pid: u32) -> Option<(u16, String)> {
    let inodes = socket_inodes_for_pid(pid);
    let Ok(content) = std::fs::read_to_string("/proc/net/tcp") else {
        return None;
    };
    let ports = loopback_listen_ports_for_inodes(&content, &inodes);
    let mut join = tokio::task::JoinSet::new();
    for port in ports {
        let url = format!("http://127.0.0.1:{port}");
        join.spawn(async move {
            match health_check(&url, DISCOVERY_PROBE_TIMEOUT).await {
                Ok(model) => Some((port, model)),
                Err(_) => None,
            }
        });
    }
    while let Some(result) = join.join_next().await {
        if let Ok(Some(hit)) = result {
            return Some(hit);
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The /proc/net/tcp parser pins the loopback LISTEN ports filtered by
    /// socket inode (state, remote-only, non-loopback and foreign-inode
    /// rows are ignored). Field layout: 0=sl, 1=local_address,
    /// 2=rem_address, 3=st, 4=tx:rx, 5=tr:tm, 6=retrnsmt, 7=uid, 8=timeout,
    /// 9=inode.
    #[test]
    fn parse_pins_loopback_listen_ports_by_inode() {
        // 0x1F40 = 8000 (LISTEN 127.0.0.1, inode 12345 — kept), 0x1F94 =
        // 8084 (state 06 TIME_WAIT — ignored), 0x232B on 0.0.0.0 (ignored),
        // 0x0050 = 80 (state 01 ESTABLISHED — ignored), 0x3388 = 13192
        // (LISTEN 127.0.0.1, inode 777 — kept only when in the set).
        let content = "  sl  local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode
             0: 0100007F:1F40 00000000:0000 0A 00000000:00000000 00:00000000 00000000 0 0 12345 1
             1: 0100007F:1F94 00000000:0000 06 00000000:00000000 00:00000000 00000000 0 0 0 1
             2: 00000000:232B 00000000:0000 0A 00000000:00000000 00:00000000 00000000 0 0 0 1
             3: 0100007F:0050 0100007F:9E5D 01 00000000:00000000 00:00000000 00000000 1000 0 0 1
             4: 0100007F:3388 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 777 1";
        let only_8000: HashSet<u64> = [12345u64].into_iter().collect();
        assert_eq!(
            loopback_listen_ports_for_inodes(content, &only_8000),
            vec![8000]
        );
        let both: HashSet<u64> = [12345u64, 777].into_iter().collect();
        assert_eq!(
            loopback_listen_ports_for_inodes(content, &both),
            vec![8000, 13192]
        );
        let none: HashSet<u64> = [4242u64].into_iter().collect();
        assert!(loopback_listen_ports_for_inodes(content, &none).is_empty());
    }

    /// Discovery finds a real loopback LISTEN socket owned by this process
    /// (the socket-inode ↔ /proc/net/tcp join on a live listener).
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn discovery_finds_own_listener_port() {
        let listener = tokio::net::TcpListener::bind(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::LOCALHOST,
            0,
        ))
        .await
        .expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let inodes = socket_inodes_for_pid(std::process::id() as u32);
        assert!(
            !inodes.is_empty(),
            "the process's own LISTEN socket inode must be visible in /proc/fd"
        );
        let content = std::fs::read_to_string("/proc/net/tcp").expect("read /proc/net/tcp");
        let ports = loopback_listen_ports_for_inodes(&content, &inodes);
        assert!(
            ports.contains(&port),
            "the own listener port {port} must be among the pid's loopback LISTEN ports: {ports:?}"
        );
    }
}
