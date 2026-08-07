//! DNS wire-format request handling for the configured target hostname.

use std::net::{Ipv4Addr, Ipv6Addr};

use hickory_proto::{
    op::{Message, MessageType, OpCode, ResponseCode},
    rr::{
        DNSClass, RData, Record, RecordType,
        rdata::{A, AAAA},
    },
    serialize::binary::DecodeError,
};

const DNS_TTL_SECONDS: u32 = 60;

/// A non-recursive resolver that synthesizes configured A and AAAA records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolver {
    target_hostname: String,
    relay_ipv4: Ipv4Addr,
    relay_ipv6: Option<Ipv6Addr>,
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

impl Resolver {
    /// Create a resolver for `target_hostname` that returns `relay_ipv4` and,
    /// when present, `relay_ipv6`.
    #[must_use]
    pub fn new(
        target_hostname: impl Into<String>,
        relay_ipv4: Ipv4Addr,
        relay_ipv6: Option<Ipv6Addr>,
    ) -> Self {
        let target_hostname = target_hostname.into().trim_end_matches('.').to_ascii_lowercase();
        Self { target_hostname, relay_ipv4, relay_ipv6 }
    }

    /// Resolve one DNS wire-format request into a DNS wire-format response.
    ///
    /// The resolver never performs recursion or forwards a request. It returns
    /// `REFUSED` for names other than its configured target hostname.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError::Decode`] for malformed requests and
    /// [`ResolveError::Encode`] if the response cannot be serialized.
    pub fn resolve(&self, wire_request: &[u8]) -> Result<Vec<u8>, ResolveError> {
        let request = Message::from_vec(wire_request)?;
        let mut response = response_for(&request);

        let [query] = request.queries.as_slice() else {
            response.metadata.response_code = ResponseCode::FormErr;
            return Ok(response.to_vec()?);
        };
        if request.metadata.message_type != MessageType::Query {
            response.metadata.response_code = ResponseCode::FormErr;
            return Ok(response.to_vec()?);
        }

        if request.metadata.op_code != OpCode::Query {
            response.metadata.response_code = ResponseCode::NotImp;
            return Ok(response.to_vec()?);
        }

        if query.query_class() != DNSClass::IN || !self.is_target(query.name()) {
            response.metadata.response_code = ResponseCode::Refused;
            return Ok(response.to_vec()?);
        }

        let answer = match query.query_type() {
            RecordType::A => Some(RData::A(A(self.relay_ipv4))),
            RecordType::AAAA => self.relay_ipv6.map(|address| RData::AAAA(AAAA(address))),
            _ => None,
        };
        if let Some(answer) = answer {
            response.add_answer(Record::from_rdata(query.name().clone(), DNS_TTL_SECONDS, answer));
        }

        Ok(response.to_vec()?)
    }

    fn is_target(&self, name: &hickory_proto::rr::Name) -> bool {
        name.to_ascii().trim_end_matches('.').eq_ignore_ascii_case(&self.target_hostname)
    }
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

    const TARGET_HOSTNAME: &str = "example.com";
    const RELAY_IPV4: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 10);
    const RELAY_IPV6: Ipv6Addr = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x10);

    fn resolver() -> Resolver {
        Resolver::new(TARGET_HOSTNAME, RELAY_IPV4, Some(RELAY_IPV6))
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

    fn resolve(resolver: Resolver, request: &Message) -> Message {
        let wire_request = request.to_vec().expect("query fixture should encode");
        let wire_response = resolver.resolve(&wire_request).expect("query should resolve");
        Message::from_vec(&wire_response).expect("response should decode")
    }

    #[test]
    fn a_query_should_return_configured_relay_address_and_ttl() {
        let message = resolve(resolver(), &query(TARGET_HOSTNAME, RecordType::A));
        let answer = message.answers.first().expect("A answer should be present");

        assert_eq!(
            (message.metadata.response_code, answer.ttl, &answer.data),
            (ResponseCode::NoError, DNS_TTL_SECONDS, &RData::A(A(RELAY_IPV4)))
        );
    }

    #[test]
    fn aaaa_query_should_return_configured_relay_address_and_ttl() {
        let message = resolve(resolver(), &query(TARGET_HOSTNAME, RecordType::AAAA));
        let answer = message.answers.first().expect("AAAA answer should be present");

        assert_eq!(
            (message.metadata.response_code, answer.ttl, &answer.data),
            (ResponseCode::NoError, DNS_TTL_SECONDS, &RData::AAAA(AAAA(RELAY_IPV6)))
        );
    }

    #[test]
    fn aaaa_query_should_return_nodata_without_configured_ipv6() {
        let resolver = Resolver::new(TARGET_HOSTNAME, RELAY_IPV4, None);
        let message = resolve(resolver, &query(TARGET_HOSTNAME, RecordType::AAAA));

        assert_eq!(
            (message.metadata.response_code, message.answers.is_empty()),
            (ResponseCode::NoError, true)
        );
    }

    #[test]
    fn unsupported_record_type_for_target_should_return_nodata() {
        let message = resolve(resolver(), &query(TARGET_HOSTNAME, RecordType::TXT));

        assert_eq!(
            (message.metadata.response_code, message.answers.is_empty()),
            (ResponseCode::NoError, true)
        );
    }

    #[test]
    fn target_name_matching_should_be_case_insensitive() {
        let message = resolve(resolver(), &query("ExAmPlE.CoM.", RecordType::A));

        assert_eq!(message.answers.len(), 1);
    }

    #[test]
    fn target_name_without_textual_trailing_dot_should_match() {
        let message = resolve(resolver(), &query(TARGET_HOSTNAME, RecordType::A));

        assert_eq!(message.answers.len(), 1);
    }

    #[test]
    fn unrelated_name_should_be_refused_without_recursion() {
        let message = resolve(resolver(), &query("example.net.", RecordType::A));

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
        let mut message = query(TARGET_HOSTNAME, RecordType::A);
        message.add_query(Query::query(
            Name::from_ascii("example.net.").expect("fixture should be valid"),
            RecordType::A,
        ));
        let response = resolve(resolver(), &message);

        assert_eq!(response.metadata.response_code, ResponseCode::FormErr);
    }

    #[test]
    fn unsupported_dns_opcode_should_return_notimp() {
        let mut message = Message::new(0x1234, MessageType::Query, OpCode::Notify);
        message.add_query(Query::query(
            Name::from_ascii(TARGET_HOSTNAME).expect("fixture should be valid"),
            RecordType::A,
        ));
        let response = resolve(resolver(), &message);

        assert_eq!(response.metadata.response_code, ResponseCode::NotImp);
    }
}
