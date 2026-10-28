//! Render the dedicated node's IPv4 forwarding policy as nftables transactions.
//!
//! These functions only build text; [`crate::service::install`] checks table
//! ownership and validates it with `nft` before applying it. The table's forward
//! chain drops traffic unless a rule accepts a connection marked by this gateway.
//! It therefore assumes the node is dedicated to this forwarding service.
//!
//! NFLOG sends packet copies to userspace. It does not enqueue forwarding
//! decisions in NFQUEUE, and collection failures cannot withhold a verdict.

use std::net::SocketAddrV4;

/// IPv4 table name managed exclusively by this service.
pub const TABLE_NAME: &str = "rokbattles_nat_gateway";

/// Conntrack mark identifying connections selected by the gateway (`ROKB`).
pub const CT_MARK: u32 = 0x524f_4b42;

/// NFLOG group bound by [`crate::capture::Capture`].
pub const NFLOG_GROUP: u16 = 3101;

/// Schema marker required before [`crate::service::install`] replaces a table.
pub const SCHEMA_IDENTITY: &str = "rokbattles-nat-gateway-v1";

/// Render a transaction that creates the gateway table and its chains.
///
/// An unspecified bind address selects all local IPv4 destinations on the bind
/// port. An explicit address narrows that selection. New SYNs receive a conntrack
/// mark, DNAT, and masquerading; established marked connections are accepted
/// independently of the current upstream address.
///
/// This function does not inspect existing rules or enable IP forwarding. Use
/// [`render_replace`] only after verifying ownership of an existing table.
#[must_use]
pub fn render_create(bind_addr: SocketAddrV4, upstream_addr: SocketAddrV4) -> String {
    render("add", bind_addr, upstream_addr)
}

/// Render a transaction that flushes and rebuilds the gateway table's contents.
///
/// The caller must first validate the table's [`SCHEMA_IDENTITY`]. No other table
/// or conntrack state is flushed. Existing connections retain their translations
/// and marks, even if `upstream_addr` selects a new target for future SYNs.
#[must_use]
pub fn render_replace(bind_addr: SocketAddrV4, upstream_addr: SocketAddrV4) -> String {
    render("flush", bind_addr, upstream_addr)
}

fn render(action: &str, bind_addr: SocketAddrV4, upstream_addr: SocketAddrV4) -> String {
    let bind_port = bind_addr.port();
    let upstream_ip = upstream_addr.ip();
    let upstream_port = upstream_addr.port();
    let mark = format!("0x{CT_MARK:08x}");
    // The FIB check restricts DNAT to traffic addressed to this host, including
    // the wildcard-bind case; it must not capture arbitrary transit traffic.
    let destination = if bind_addr.ip().is_unspecified() {
        format!("fib daddr type local tcp dport {bind_port}")
    } else {
        format!("fib daddr type local ip daddr {} tcp dport {bind_port}", bind_addr.ip())
    };

    format!(
        "{action} table ip {TABLE_NAME}\n\
         add chain ip {TABLE_NAME} schema_identity {{ comment \"{SCHEMA_IDENTITY}\"; }}\n\
         add chain ip {TABLE_NAME} prerouting {{ type nat hook prerouting priority dstnat; policy accept; }}\n\
         add chain ip {TABLE_NAME} postrouting {{ type nat hook postrouting priority srcnat; policy accept; }}\n\
         add chain ip {TABLE_NAME} forward {{ type filter hook forward priority filter; policy drop; }}\n\
         add rule ip {TABLE_NAME} prerouting {destination} ct state new tcp flags & (fin | syn | rst | ack) == syn ct mark set {mark} counter dnat to {upstream_ip}:{upstream_port}\n\
         add rule ip {TABLE_NAME} postrouting ct mark {mark} ct status dnat counter masquerade\n\
         add rule ip {TABLE_NAME} forward meta l4proto icmp ct mark {mark} ct status dnat ct state related icmp type {{ destination-unreachable, time-exceeded, parameter-problem }} counter accept\n\
         add rule ip {TABLE_NAME} forward meta l4proto tcp ct mark {mark} ct status dnat ct state established,related counter log group {NFLOG_GROUP} snaplen 65535 queue-threshold 1 accept\n\
         add rule ip {TABLE_NAME} forward meta l4proto tcp ct mark {mark} ct status dnat ct state new tcp flags & (fin | syn | rst | ack) == syn ip daddr {upstream_ip} tcp dport {upstream_port} counter log group {NFLOG_GROUP} snaplen 65535 queue-threshold 1 accept\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> String {
        render_create(
            "192.0.2.10:3101".parse().expect("test bind address"),
            "198.51.100.20:3201".parse().expect("test upstream address"),
        )
    }

    #[test]
    fn rules_scope_new_connections_to_the_configured_tuple() {
        let rules = rules();

        assert!(rules.contains(
            "fib daddr type local ip daddr 192.0.2.10 tcp dport 3101 ct state new tcp flags & (fin | syn | rst | ack) == syn ct mark set 0x524f4b42 counter dnat to 198.51.100.20:3201"
        ));
        assert!(rules.contains(
            "ct mark 0x524f4b42 ct status dnat ct state new tcp flags & (fin | syn | rst | ack) == syn ip daddr 198.51.100.20 tcp dport 3201 counter log group 3101"
        ));
    }

    #[test]
    fn rules_fail_closed_and_only_nat_owned_connections() {
        let rules = rules();

        assert!(rules.contains(
            "chain ip rokbattles_nat_gateway forward { type filter hook forward priority filter; policy drop; }"
        ));
        assert!(rules.contains("postrouting ct mark 0x524f4b42 ct status dnat counter masquerade"));
        assert!(!rules.contains("flush ruleset"));
    }

    #[test]
    fn reply_capture_is_passive_and_exactly_scoped() {
        let rules = rules();
        let logging_rules =
            rules.lines().filter(|line| line.contains(" log group ")).collect::<Vec<_>>();

        assert_eq!(logging_rules.len(), 2);
        assert!(logging_rules[0].contains(
            "meta l4proto tcp ct mark 0x524f4b42 ct status dnat ct state established,related"
        ));
        assert!(logging_rules[0].contains("log group 3101 snaplen 65535 queue-threshold 1 accept"));
        assert!(!rules.contains("queue num"), "NFQUEUE would put userspace in the forwarding path");
    }

    #[test]
    fn established_acceptance_survives_upstream_replacement() {
        let rules = rules();
        let established = rules
            .lines()
            .find(|line| line.contains("ct state established,related counter log group"))
            .expect("generic established-flow rule");

        assert!(!established.contains("198.51.100.20"));
        assert!(established.contains("ct mark 0x524f4b42"));
    }

    #[test]
    fn only_related_owned_icmp_errors_bypass_the_tcp_policy() {
        let rules = rules();
        let icmp = rules
            .lines()
            .find(|line| line.contains("meta l4proto icmp"))
            .expect("related ICMP rule");

        assert!(icmp.contains("ct mark 0x524f4b42 ct status dnat ct state related"));
        assert!(icmp.contains(
            "icmp type { destination-unreachable, time-exceeded, parameter-problem } counter accept"
        ));
        assert!(!icmp.contains(" log "));
        assert_eq!(rules.matches("meta l4proto icmp").count(), 1);
    }

    #[test]
    fn replacement_flushes_only_the_validated_owned_table() {
        let rules = render_replace(
            "0.0.0.0:3101".parse().expect("test bind address"),
            "198.51.100.20:3201".parse().expect("test upstream address"),
        );

        assert!(rules.starts_with("flush table ip rokbattles_nat_gateway\n"));
        assert!(!rules.contains("flush ruleset"));
    }

    #[test]
    fn wildcard_bind_selects_only_local_destinations_on_its_port() {
        let rules = render_create(
            "0.0.0.0:3200".parse().expect("bind"),
            "8.8.8.8:3101".parse().expect("upstream"),
        );
        let prerouting =
            rules.lines().find(|line| line.contains(" prerouting fib ")).expect("DNAT rule");
        assert!(prerouting.contains("fib daddr type local tcp dport 3200"));
        assert!(!prerouting.contains("ip daddr"));
        assert!(!prerouting.contains("0.0.0.0"));
    }
}
