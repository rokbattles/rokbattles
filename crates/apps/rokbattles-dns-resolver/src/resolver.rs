//! DNS wire-format request handling for `rocgate.lilithgame.com`.

use std::{
    collections::HashSet,
    net::Ipv4Addr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use hickory_proto::{
    op::{Message, MessageType, OpCode, ResponseCode},
    rr::{DNSClass, RData, Record, RecordType, rdata::A},
    serialize::binary::DecodeError,
};

use crate::MAX_DNS_MESSAGE_BYTES;

const DNS_TTL_SECONDS: u32 = 60;
// A compressed A answer uses a two-byte name pointer, ten bytes of record
// metadata, and four address bytes.
const COMPRESSED_A_ANSWER_BYTES: usize = 16;
/// The game hostname synthesized by every resolver instance.
pub const ROCGATE_HOSTNAME: &str = "rocgate.lilithgame.com";
/// The private suffix used for cache-resistant resolver checks.
pub const DNS_CHECK_DOMAIN: &str = "probe.rokbattles.com";

/// A non-recursive resolver that synthesizes configured gateway A records.
#[derive(Debug, Clone)]
pub struct Resolver {
    gateway_ipv4s: Arc<[Ipv4Addr]>,
    next_start: Arc<AtomicUsize>,
}

/// Invalid static gateway configuration supplied to [`Resolver::new`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ResolverConfigError {
    /// The finite gateway list could not be reserved safely.
    #[error("gateway list is too large for available memory")]
    Allocation,
    /// At least one gateway address is required.
    #[error("at least one gateway IPv4 address is required")]
    Empty,
    /// Returning one address twice would not add redundancy.
    #[error("duplicate gateway IPv4 address: {address}")]
    Duplicate { address: Ipv4Addr },
    /// Client-facing gateway answers must be publicly routable unicast.
    #[error("gateway IPv4 address is not public unicast: {address}")]
    NonPublic { address: Ipv4Addr },
}

/// Failures decoding or encoding DNS wire-format messages.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// The request was not a valid DNS wire-format message.
    #[error("invalid DNS message: {0}")]
    Decode(#[from] DecodeError),
    /// A DNS response could not be encoded.
    #[error("failed to encode DNS response: {0}")]
    Encode(#[from] hickory_proto::ProtoError),
}

pub(crate) enum HttpResolution {
    Local(Vec<u8>),
    Probe { response: Vec<u8>, nonce: String },
    NonTarget,
}

enum Resolution {
    Local(Message),
    Probe { response: Message, nonce: String },
    NonTarget,
}

impl Resolver {
    /// Create a resolver from a finite, owned gateway list for [`ROCGATE_HOSTNAME`].
    ///
    /// Clones share one lightweight round-robin cursor so HTTP router state
    /// cloning does not restart answer rotation.
    ///
    /// # Errors
    ///
    /// Returns [`ResolverConfigError`] when the list is empty, contains a
    /// duplicate, cannot be reserved, or includes a non-public-unicast address.
    pub fn new(addresses: Vec<Ipv4Addr>) -> Result<Self, ResolverConfigError> {
        let mut seen = HashSet::new();
        seen.try_reserve(addresses.len()).map_err(|_| ResolverConfigError::Allocation)?;
        for &address in &addresses {
            if !is_public_unicast(address) {
                return Err(ResolverConfigError::NonPublic { address });
            }
            if !seen.insert(address) {
                return Err(ResolverConfigError::Duplicate { address });
            }
        }
        if addresses.is_empty() {
            return Err(ResolverConfigError::Empty);
        }

        Ok(Self { gateway_ipv4s: addresses.into(), next_start: Arc::new(AtomicUsize::new(0)) })
    }

    /// Resolve one DNS wire-format request into a DNS wire-format response.
    ///
    /// The resolver never performs recursion or forwards a request. It returns
    /// `REFUSED` for names other than [`ROCGATE_HOSTNAME`].
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError::Decode`] for malformed requests and
    /// [`ResolveError::Encode`] if the response cannot be serialized.
    pub fn resolve(&self, wire_request: &[u8]) -> Result<Vec<u8>, ResolveError> {
        match self.resolve_for_http(wire_request)? {
            HttpResolution::Local(response) | HttpResolution::Probe { response, .. } => {
                Ok(response)
            }
            HttpResolution::NonTarget => self.refused(wire_request),
        }
    }

    pub(crate) fn resolve_for_http(
        &self,
        wire_request: &[u8],
    ) -> Result<HttpResolution, ResolveError> {
        let request = Message::from_vec(wire_request)?;
        match self.resolve_message(&request, wire_request.len()) {
            Resolution::Local(response) => Ok(HttpResolution::Local(response.to_vec()?)),
            Resolution::Probe { response, nonce } => {
                Ok(HttpResolution::Probe { response: response.to_vec()?, nonce })
            }
            Resolution::NonTarget => Ok(HttpResolution::NonTarget),
        }
    }

    pub(crate) fn refused(&self, wire_request: &[u8]) -> Result<Vec<u8>, ResolveError> {
        let request = Message::from_vec(wire_request)?;
        let mut response = response_for(&request);
        response.metadata.response_code = ResponseCode::Refused;
        Ok(response.to_vec()?)
    }

    pub(crate) fn servfail(&self, wire_request: &[u8]) -> Result<Vec<u8>, ResolveError> {
        let request = Message::from_vec(wire_request)?;
        let mut response = response_for(&request);
        response.metadata.response_code = ResponseCode::ServFail;
        Ok(response.to_vec()?)
    }

    fn resolve_message(&self, request: &Message, request_wire_bytes: usize) -> Resolution {
        let mut response = response_for(request);

        if request.metadata.message_type != MessageType::Query || request.queries.len() != 1 {
            response.metadata.response_code = ResponseCode::FormErr;
            return Resolution::Local(response);
        }

        if request.metadata.op_code != OpCode::Query {
            response.metadata.response_code = ResponseCode::NotImp;
            return Resolution::Local(response);
        }

        let query = &request.queries[0];
        if query.query_class() != DNSClass::IN {
            return Resolution::NonTarget;
        }

        if let Some(nonce) = dns_check_nonce(query.name()) {
            response.metadata.response_code = ResponseCode::NXDomain;
            return Resolution::Probe { response, nonce };
        }

        if !self.is_target(query.name()) {
            return Resolution::NonTarget;
        }

        if query.query_type() == RecordType::A {
            let fleet_size = self.gateway_ipv4s.len();
            let answer_capacity = MAX_DNS_MESSAGE_BYTES.saturating_sub(request_wire_bytes)
                / COMPRESSED_A_ANSWER_BYTES;
            let answer_count = fleet_size.min(answer_capacity);
            let start = self.next_start.fetch_add(1, Ordering::Relaxed) % fleet_size;
            for offset in 0..answer_count {
                let address = self.gateway_ipv4s[(start + offset) % fleet_size];
                response.add_answer(Record::from_rdata(
                    query.name().clone(),
                    DNS_TTL_SECONDS,
                    RData::A(A(address)),
                ));
            }
        }

        Resolution::Local(response)
    }

    fn is_target(&self, name: &hickory_proto::rr::Name) -> bool {
        name.to_ascii().trim_end_matches('.').eq_ignore_ascii_case(ROCGATE_HOSTNAME)
    }
}

fn dns_check_nonce(name: &hickory_proto::rr::Name) -> Option<String> {
    let ascii = name.to_ascii();
    let normalized = ascii.trim_end_matches('.');
    let (nonce, domain) = normalized.split_once('.')?;
    if !domain.eq_ignore_ascii_case(DNS_CHECK_DOMAIN) {
        return None;
    }
    (nonce.len() == 32
        && nonce.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
    .then(|| nonce.to_string())
}

pub(crate) fn is_public_unicast(address: Ipv4Addr) -> bool {
    let value = u32::from(address);
    ![
        (u32::from(Ipv4Addr::new(0, 0, 0, 0)), 8),
        (u32::from(Ipv4Addr::new(10, 0, 0, 0)), 8),
        (u32::from(Ipv4Addr::new(100, 64, 0, 0)), 10),
        (u32::from(Ipv4Addr::new(127, 0, 0, 0)), 8),
        (u32::from(Ipv4Addr::new(169, 254, 0, 0)), 16),
        (u32::from(Ipv4Addr::new(172, 16, 0, 0)), 12),
        (u32::from(Ipv4Addr::new(192, 0, 0, 0)), 24),
        (u32::from(Ipv4Addr::new(192, 0, 2, 0)), 24),
        (u32::from(Ipv4Addr::new(192, 88, 99, 0)), 24),
        (u32::from(Ipv4Addr::new(192, 168, 0, 0)), 16),
        (u32::from(Ipv4Addr::new(198, 18, 0, 0)), 15),
        (u32::from(Ipv4Addr::new(198, 51, 100, 0)), 24),
        (u32::from(Ipv4Addr::new(203, 0, 113, 0)), 24),
        (u32::from(Ipv4Addr::new(224, 0, 0, 0)), 3),
    ]
    .iter()
    .any(|(network, prefix)| {
        let mask = u32::MAX.checked_shl(32_u32.saturating_sub(*prefix)).unwrap_or(0);
        value & mask == network & mask
    })
}

fn response_for(request: &Message) -> Message {
    let mut response = Message::response(request.metadata.id, request.metadata.op_code);
    response.add_queries(request.queries.iter().cloned());
    response.metadata.recursion_desired = request.metadata.recursion_desired;
    if let Some(edns) = request.edns.clone() {
        response.set_edns(edns);
    }
    response
}

#[cfg(test)]
mod tests {
    use hickory_proto::{
        op::{Query, ResponseCode},
        rr::{Name, RecordType},
    };

    use super::*;

    const GATEWAY_IPV4_A: Ipv4Addr = Ipv4Addr::new(93, 184, 216, 34);
    const GATEWAY_IPV4_B: Ipv4Addr = Ipv4Addr::new(1, 1, 1, 1);

    fn resolver() -> Resolver {
        Resolver::new(vec![GATEWAY_IPV4_A, GATEWAY_IPV4_B])
            .expect("resolver fixture should be valid")
    }

    fn query(name: &str, record_type: RecordType) -> Message {
        let mut message = Message::new(0x1234, MessageType::Query, OpCode::Query);
        message.metadata.recursion_desired = true;
        message.add_query(Query::query(
            Name::from_ascii(name).expect("query name fixture should be valid"),
            record_type,
        ));
        message
    }

    fn resolve(resolver: &Resolver, request: &Message) -> Message {
        let wire_request = request.to_vec().expect("query fixture should encode");
        let wire_response = resolver.resolve(&wire_request).expect("query should resolve");
        Message::from_vec(&wire_response).expect("response should decode")
    }

    #[test]
    fn a_query_should_return_all_gateway_addresses_and_ttl() {
        let message = resolve(&resolver(), &query(ROCGATE_HOSTNAME, RecordType::A));

        assert_eq!(
            (
                message.metadata.response_code,
                message.answers.iter().map(|answer| (answer.ttl, &answer.data)).collect::<Vec<_>>(),
            ),
            (
                ResponseCode::NoError,
                vec![
                    (DNS_TTL_SECONDS, &RData::A(A(GATEWAY_IPV4_A))),
                    (DNS_TTL_SECONDS, &RData::A(A(GATEWAY_IPV4_B))),
                ],
            )
        );
    }

    #[test]
    fn consecutive_a_queries_should_rotate_the_first_gateway() {
        let resolver = resolver();

        let first = resolve(&resolver, &query(ROCGATE_HOSTNAME, RecordType::A));
        let second = resolve(&resolver, &query(ROCGATE_HOSTNAME, RecordType::A));
        let third = resolve(&resolver, &query(ROCGATE_HOSTNAME, RecordType::A));

        let addresses = |message: &Message| {
            message.answers.iter().map(|answer| answer.data.clone()).collect::<Vec<_>>()
        };
        assert_eq!(
            (addresses(&first), addresses(&second), addresses(&third)),
            (
                vec![RData::A(A(GATEWAY_IPV4_A)), RData::A(A(GATEWAY_IPV4_B))],
                vec![RData::A(A(GATEWAY_IPV4_B)), RData::A(A(GATEWAY_IPV4_A))],
                vec![RData::A(A(GATEWAY_IPV4_A)), RData::A(A(GATEWAY_IPV4_B))],
            )
        );
    }

    #[test]
    fn cloned_resolver_should_share_answer_rotation() {
        let resolver = resolver();
        let clone = resolver.clone();

        let first = resolve(&resolver, &query(ROCGATE_HOSTNAME, RecordType::A));
        let second = resolve(&clone, &query(ROCGATE_HOSTNAME, RecordType::A));

        assert_eq!(first.answers[0].data, RData::A(A(GATEWAY_IPV4_A)));
        assert_eq!(second.answers[0].data, RData::A(A(GATEWAY_IPV4_B)));
    }

    #[test]
    fn non_a_queries_should_not_advance_answer_rotation() {
        let resolver = resolver();

        let first = resolve(&resolver, &query(ROCGATE_HOSTNAME, RecordType::A));
        let aaaa = resolve(&resolver, &query(ROCGATE_HOSTNAME, RecordType::AAAA));
        let second = resolve(&resolver, &query(ROCGATE_HOSTNAME, RecordType::A));

        assert_eq!(first.answers[0].data, RData::A(A(GATEWAY_IPV4_A)));
        assert!(aaaa.answers.is_empty());
        assert_eq!(second.answers[0].data, RData::A(A(GATEWAY_IPV4_B)));
    }

    #[test]
    fn invalid_gateway_lists_should_be_rejected() {
        assert_eq!(
            Resolver::new(vec![]).expect_err("empty list should be rejected"),
            ResolverConfigError::Empty
        );
        assert_eq!(
            Resolver::new(vec![GATEWAY_IPV4_A, GATEWAY_IPV4_A])
                .expect_err("duplicate should be rejected"),
            ResolverConfigError::Duplicate { address: GATEWAY_IPV4_A }
        );
        assert_eq!(
            Resolver::new(vec![Ipv4Addr::LOCALHOST])
                .expect_err("non-public address should be rejected"),
            ResolverConfigError::NonPublic { address: Ipv4Addr::LOCALHOST }
        );
    }

    #[test]
    fn resolver_should_not_have_an_application_node_count_limit() {
        let gateways = (11..=32).map(|first_octet| Ipv4Addr::new(first_octet, 0, 0, 1)).collect();
        let resolver = Resolver::new(gateways).expect("all public gateways should be accepted");
        let response = resolve(&resolver, &query(ROCGATE_HOSTNAME, RecordType::A));

        assert_eq!(response.answers.len(), 22);
    }

    #[test]
    fn oversized_fleet_should_rotate_through_protocol_sized_response_windows() {
        let gateways = (0..4_200)
            .map(|index| Ipv4Addr::new(11, ((index >> 8) & 0xff) as u8, (index & 0xff) as u8, 1))
            .collect();
        let resolver = Resolver::new(gateways).expect("all public gateways should be accepted");
        let request = query(ROCGATE_HOSTNAME, RecordType::A);
        let first = resolve(&resolver, &request);
        let second = resolve(&resolver, &request);

        assert_eq!(
            (
                first.answers.len() < 4_200,
                first.to_vec().expect("response should fit DNS wire format").len()
                    <= MAX_DNS_MESSAGE_BYTES,
                first.answers[1].data == second.answers[0].data,
            ),
            (true, true, true)
        );
    }

    #[test]
    fn aaaa_query_should_return_nodata() {
        let message = resolve(&resolver(), &query(ROCGATE_HOSTNAME, RecordType::AAAA));

        assert_eq!(
            (message.metadata.response_code, message.answers.is_empty()),
            (ResponseCode::NoError, true)
        );
    }

    #[test]
    fn unsupported_record_type_for_target_should_return_nodata() {
        let message = resolve(&resolver(), &query(ROCGATE_HOSTNAME, RecordType::TXT));

        assert_eq!(
            (message.metadata.response_code, message.answers.is_empty()),
            (ResponseCode::NoError, true)
        );
    }

    #[test]
    fn unique_dns_check_query_should_return_nxdomain_and_expose_nonce_to_http_layer() {
        let nonce = "0123456789abcdef0123456789abcdef";
        let request = query(&format!("{nonce}.{DNS_CHECK_DOMAIN}"), RecordType::A);
        let wire_request = request.to_vec().expect("query fixture should encode");
        let resolution =
            resolver().resolve_for_http(&wire_request).expect("probe query should resolve locally");

        let HttpResolution::Probe { response, nonce: reported_nonce } = resolution else {
            panic!("probe query should be classified separately");
        };
        let response = Message::from_vec(&response).expect("probe response should decode");
        assert_eq!(
            (response.metadata.response_code, response.answers.is_empty(), reported_nonce),
            (ResponseCode::NXDomain, true, nonce.to_string())
        );
    }

    #[test]
    fn malformed_dns_check_nonce_should_remain_a_non_target() {
        for nonce in
            ["short", "0123456789abcdef0123456789abcdeg", "0123456789ABCDEF0123456789ABCDEF"]
        {
            let message =
                resolve(&resolver(), &query(&format!("{nonce}.{DNS_CHECK_DOMAIN}"), RecordType::A));

            assert_eq!(message.metadata.response_code, ResponseCode::Refused);
        }
    }

    #[test]
    fn dns_check_domain_matching_should_be_case_insensitive() {
        let nonce = "0123456789abcdef0123456789abcdef";
        let request = query(&format!("{nonce}.PrObE.RoKbAtTlEs.CoM"), RecordType::AAAA);
        let wire_request = request.to_vec().expect("query fixture should encode");

        assert!(matches!(
            resolver().resolve_for_http(&wire_request),
            Ok(HttpResolution::Probe { .. })
        ));
    }

    #[test]
    fn target_name_matching_should_be_case_insensitive() {
        let message = resolve(&resolver(), &query("RoCgAtE.LiLiThGaMe.CoM.", RecordType::A));

        assert_eq!(message.answers.len(), 2);
    }

    #[test]
    fn target_name_without_textual_trailing_dot_should_match() {
        let message = resolve(&resolver(), &query(ROCGATE_HOSTNAME, RecordType::A));

        assert_eq!(message.answers.len(), 2);
    }

    #[test]
    fn unrelated_name_should_be_refused_without_recursion() {
        let message = resolve(&resolver(), &query("accounts.lilithgame.com.", RecordType::A));

        assert_eq!(
            (
                message.metadata.response_code,
                message.metadata.recursion_available,
                message.answers.is_empty(),
            ),
            (ResponseCode::Refused, false, true)
        );
    }

    #[test]
    fn multiple_dns_questions_should_return_formerr() {
        let mut message = query(ROCGATE_HOSTNAME, RecordType::A);
        message.add_query(Query::query(
            Name::from_ascii("accounts.lilithgame.com.").expect("fixture should be valid"),
            RecordType::A,
        ));
        let response = resolve(&resolver(), &message);

        assert_eq!(response.metadata.response_code, ResponseCode::FormErr);
    }

    #[test]
    fn unsupported_dns_opcode_should_return_notimp() {
        let mut message = Message::new(0x1234, MessageType::Query, OpCode::Notify);
        message.add_query(Query::query(
            Name::from_ascii(ROCGATE_HOSTNAME).expect("fixture should be valid"),
            RecordType::A,
        ));
        let response = resolve(&resolver(), &message);

        assert_eq!(response.metadata.response_code, ResponseCode::NotImp);
    }
}
