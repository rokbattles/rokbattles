//! nftables rules for the dedicated IPv4 gateway node.

use std::net::SocketAddrV4;

/// Application-owned nftables table.
pub const TABLE_NAME: &str = "rokbattles_nat_gateway";
/// Conntrack mark reserved for flows selected by this gateway (`ROKB`).
pub const CT_MARK: u32 = 0x524f_4b42;
/// Netfilter log group consumed by the passive capture helper.
pub const NFLOG_GROUP: u16 = 3101;
/// Marker used by startup to reject an unknown or incompatible owned table.
pub const SCHEMA_IDENTITY: &str = "rokbattles-nat-gateway-v1";

/// Render one atomic nftables program that replaces the application-owned table.
///
/// Conntrack entries and their marks are kernel state outside this table. The
/// established-flow rules deliberately avoid the current upstream address, so
/// an atomic replacement can change the target for new connections while old
/// translations continue to drain.
#[must_use]
pub fn render_create(bind_addr: SocketAddrV4, upstream_addr: SocketAddrV4) -> String {
    render("add", bind_addr, upstream_addr)
}

/// Render an atomic replacement for an already validated application table.
#[must_use]
pub fn render_replace(bind_addr: SocketAddrV4, upstream_addr: SocketAddrV4) -> String {
    render("flush", bind_addr, upstream_addr)
}

fn render(action: &str, bind_addr: SocketAddrV4, upstream_addr: SocketAddrV4) -> String {
    let bind_port = bind_addr.port();
    let upstream_ip = upstream_addr.ip();
    let upstream_port = upstream_addr.port();
    let mark = format!("0x{CT_MARK:08x}");
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
}
