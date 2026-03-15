mod response;
mod validation;

use reqwest::{
    Client, Request, Url,
    header::{ACCEPT, HeaderMap, HeaderName, HeaderValue},
};
use serde::{Serialize, de::DeserializeOwned};

use self::{
    response::{ApiEnvelope, parse_api_response},
    validation::{
        normalize_kingdom_member_request, validate_kingdom_list_request, validate_server_id,
    },
};
use crate::{
    config::RokGtConfig,
    error::RokGtError,
    models::{
        KingdomInformationResponse, KingdomListRequest, KingdomListResponse, KingdomMemberRequest,
        KingdomMemberResponse, LatestServerIdsResponse,
    },
};

const KINDOM_LIST_PATH: &str = "/api/kindomList";
const LATEST_SERVER_IDS_PATH: &str = "/api/latestServerIds";
const KINDOM_INFORMATION_PATH: &str = "/api/kindomInformation";
const KINDOM_MEMBER_PATH: &str = "/api/kindomMember";
const HEADER_LANG_NAME: &str = "lang";
const HEADER_PAUTH_NAME: &str = "pauthorization";
const HEADER_BAUTH_NAME: &str = "bauthorization";

/// Async client for the ROK Game Tools Global API.
#[derive(Debug, Clone)]
pub struct RokGtClient {
    http: Client,
    base_url: Url,
    platform_base_url: Url,
}

impl RokGtClient {
    /// Construct a client with shared headers and auth configuration.
    pub fn new(config: RokGtConfig) -> Result<Self, RokGtError> {
        validate_non_empty("p_authorization_token", &config.p_authorization_token)?;
        validate_non_empty("b_authorization_token", &config.b_authorization_token)?;
        validate_non_empty("lang", &config.lang)?;
        if config.timeout.is_zero() {
            return Err(RokGtError::InvalidConfig {
                field: "timeout",
                reason: "must be greater than zero",
            });
        }

        let base_url = Url::parse(config.base_url.trim()).map_err(|_| {
            RokGtError::InvalidConfig { field: "base_url", reason: "must be a valid absolute URL" }
        })?;
        let platform_base_url =
            Url::parse(config.platform_base_url.trim()).map_err(|_| RokGtError::InvalidConfig {
                field: "platform_base_url",
                reason: "must be a valid absolute URL",
            })?;
        let default_headers = build_default_headers(&config)?;

        let http = Client::builder()
            .timeout(config.timeout)
            .default_headers(default_headers)
            .build()
            .map_err(RokGtError::ClientBuild)?;

        Ok(Self { http, base_url, platform_base_url })
    }

    /// List kingdom leaderboard entries from `/api/kindomList`.
    pub async fn list_kingdoms(
        &self,
        request: KingdomListRequest,
    ) -> Result<KingdomListResponse, RokGtError> {
        let request = self.build_kingdom_list_http_request(&request)?;
        self.execute_and_parse(request).await
    }

    /// Read the latest server ids from `/api/latestServerIds`.
    pub async fn latest_server_ids(&self) -> Result<LatestServerIdsResponse, RokGtError> {
        let request = self.build_latest_server_ids_http_request()?;
        self.execute_and_parse(request).await
    }

    /// Read kingdom information from `/api/kindomInformation`.
    pub async fn kingdom_information(
        &self,
        server_id: impl AsRef<str>,
    ) -> Result<KingdomInformationResponse, RokGtError> {
        let request = self.build_kingdom_information_http_request(server_id.as_ref())?;
        self.execute_and_parse(request).await
    }

    /// Read kingdom member stats from `/api/kindomMember`.
    pub async fn kingdom_members(
        &self,
        request: KingdomMemberRequest,
    ) -> Result<KingdomMemberResponse, RokGtError> {
        let request = self.build_kingdom_member_http_request(&request)?;
        self.execute_and_parse(request).await
    }

    async fn execute_and_parse<T>(&self, request: Request) -> Result<T, RokGtError>
    where
        T: DeserializeOwned + ApiEnvelope,
    {
        let response = self.http.execute(request).await.map_err(RokGtError::Request)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(RokGtError::Request)?;
        parse_api_response(status, &bytes)
    }

    fn build_get_request(
        &self,
        base_url: &Url,
        base_field: &'static str,
        path: &str,
    ) -> Result<Request, RokGtError> {
        let url = base_url.join(path).map_err(|_| RokGtError::InvalidConfig {
            field: base_field,
            reason: "failed to compose endpoint URL",
        })?;

        self.http.get(url).build().map_err(RokGtError::Request)
    }

    fn build_get_request_with_query<Q>(
        &self,
        base_url: &Url,
        base_field: &'static str,
        path: &str,
        query: &Q,
    ) -> Result<Request, RokGtError>
    where
        Q: Serialize + ?Sized,
    {
        let url = base_url.join(path).map_err(|_| RokGtError::InvalidConfig {
            field: base_field,
            reason: "failed to compose endpoint URL",
        })?;

        self.http.get(url).query(query).build().map_err(RokGtError::Request)
    }

    fn build_kingdom_list_http_request(
        &self,
        request: &KingdomListRequest,
    ) -> Result<Request, RokGtError> {
        validate_kingdom_list_request(request)?;

        #[derive(Serialize)]
        struct Query<'a> {
            page: u32,
            size: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            server_id: Option<&'a str>,
            order_by: crate::models::KingdomOrderBy,
        }

        let server_id = request.server_id.as_deref().map(validate_server_id).transpose()?;

        self.build_get_request_with_query(
            &self.base_url,
            "base_url",
            KINDOM_LIST_PATH,
            &Query {
                page: request.page,
                size: request.size,
                server_id,
                order_by: request.order_by,
            },
        )
    }

    fn build_latest_server_ids_http_request(&self) -> Result<Request, RokGtError> {
        self.build_get_request(&self.platform_base_url, "platform_base_url", LATEST_SERVER_IDS_PATH)
    }

    fn build_kingdom_information_http_request(
        &self,
        server_id: &str,
    ) -> Result<Request, RokGtError> {
        let server_id = validate_server_id(server_id)?;

        #[derive(Serialize)]
        struct Query<'a> {
            server_id: &'a str,
        }

        self.build_get_request_with_query(
            &self.platform_base_url,
            "platform_base_url",
            KINDOM_INFORMATION_PATH,
            &Query { server_id },
        )
    }

    fn build_kingdom_member_http_request(
        &self,
        request: &KingdomMemberRequest,
    ) -> Result<Request, RokGtError> {
        let query = normalize_kingdom_member_request(request)?;
        self.build_get_request_with_query(
            &self.platform_base_url,
            "platform_base_url",
            KINDOM_MEMBER_PATH,
            &query,
        )
    }
}

fn build_default_headers(config: &RokGtConfig) -> Result<HeaderMap, RokGtError> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(
        HeaderName::from_static(HEADER_LANG_NAME),
        HeaderValue::from_str(config.lang.trim())
            .map_err(|source| RokGtError::InvalidHeaderValue { header: "Lang", source })?,
    );
    headers.insert(
        HeaderName::from_static(HEADER_PAUTH_NAME),
        bearer_header_value(&config.p_authorization_token, "Pauthorization")?,
    );
    headers.insert(
        HeaderName::from_static(HEADER_BAUTH_NAME),
        bearer_header_value(&config.b_authorization_token, "Bauthorization")?,
    );
    Ok(headers)
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), RokGtError> {
    if value.trim().is_empty() {
        return Err(RokGtError::InvalidConfig { field, reason: "must not be empty" });
    }
    Ok(())
}

fn bearer_header_value(token: &str, header: &'static str) -> Result<HeaderValue, RokGtError> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(RokGtError::InvalidConfig {
            field: "authorization_token",
            reason: "must not be empty",
        });
    }

    let value = if trimmed.to_ascii_lowercase().starts_with("bearer ") {
        trimmed.to_owned()
    } else {
        format!("Bearer {trimmed}")
    };

    HeaderValue::from_str(&value)
        .map_err(|source| RokGtError::InvalidHeaderValue { header, source })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> RokGtClient {
        let config = RokGtConfig::new("p-token", "b-token")
            .with_base_url("https://rok-game-tools-global-api.lilith.com");
        RokGtClient::new(config).expect("build test client")
    }

    #[test]
    fn default_headers_include_auth_and_lang() {
        let config = RokGtConfig::new("p-token", "b-token");
        let headers = build_default_headers(&config).expect("headers");

        assert_eq!(headers.get("lang").and_then(|v| v.to_str().ok()), Some("en_US"));
        assert_eq!(
            headers.get("pauthorization").and_then(|v| v.to_str().ok()),
            Some("Bearer p-token")
        );
        assert_eq!(
            headers.get("bauthorization").and_then(|v| v.to_str().ok()),
            Some("Bearer b-token")
        );
        assert_eq!(headers.get("accept").and_then(|v| v.to_str().ok()), Some("application/json"));
    }

    #[test]
    fn build_request_contains_expected_path_and_query() {
        let client = test_client();
        let request = client
            .build_kingdom_list_http_request(&KingdomListRequest::default())
            .expect("build request");

        assert_eq!(request.url().path(), "/api/kindomList");
        let query = request.url().query().expect("query");
        assert!(query.contains("page=1"));
        assert!(query.contains("size=12"));
        assert!(query.contains("order_by=power"));
        assert!(!query.contains("server_id="));
    }

    #[test]
    fn build_request_supports_server_id_and_time_order() {
        let client = test_client();
        let request = client
            .build_kingdom_list_http_request(&KingdomListRequest {
                server_id: Some("2804".to_string()),
                order_by: crate::models::KingdomOrderBy::Time,
                ..Default::default()
            })
            .expect("build request");

        let query = request.url().query().expect("query");
        assert!(query.contains("server_id=2804"));
        assert!(query.contains("order_by=time"));
    }

    #[test]
    fn build_request_trims_server_id_for_kingdom_list() {
        let client = test_client();
        let request = client
            .build_kingdom_list_http_request(&KingdomListRequest {
                server_id: Some(" 2804 ".to_string()),
                ..Default::default()
            })
            .expect("build request");

        let query = request.url().query().expect("query");
        assert!(query.contains("server_id=2804"));
        assert!(!query.contains("%20"));
    }

    #[test]
    fn build_latest_server_ids_request_uses_platform_host_and_path() {
        let client = test_client();
        let request = client.build_latest_server_ids_http_request().expect("build request");
        assert_eq!(request.url().host_str(), Some("plat-rok-gametools-global-api.lilithgames.com"));
        assert_eq!(request.url().path(), "/api/latestServerIds");
        assert!(request.url().query().is_none());
    }

    #[test]
    fn build_kingdom_information_request_uses_platform_host_path_and_query() {
        let client = test_client();
        let request = client.build_kingdom_information_http_request("2804").expect("build request");
        assert_eq!(request.url().host_str(), Some("plat-rok-gametools-global-api.lilithgames.com"));
        assert_eq!(request.url().path(), "/api/kindomInformation");
        assert_eq!(request.url().query(), Some("server_id=2804"));
    }

    #[test]
    fn build_kingdom_member_request_uses_platform_host_path_and_query() {
        let client = test_client();
        let request = client
            .build_kingdom_member_http_request(&KingdomMemberRequest::new(
                "2026-02-17",
                "2026-02-17",
                "2804",
            ))
            .expect("build request");
        assert_eq!(request.url().host_str(), Some("plat-rok-gametools-global-api.lilithgames.com"));
        assert_eq!(request.url().path(), "/api/kindomMember");
        let query = request.url().query().expect("query");
        assert!(query.contains("start=2026-02-17"));
        assert!(query.contains("end=2026-02-17"));
        assert!(query.contains("server_id=2804"));
        assert!(query.contains("search="));
    }

    #[test]
    fn build_kingdom_member_request_supports_search_by_id() {
        let client = test_client();
        let request = client
            .build_kingdom_member_http_request(
                &KingdomMemberRequest::new("2026-02-17", "2026-02-17", "2804")
                    .with_search("71738515"),
            )
            .expect("build request");
        let query = request.url().query().expect("query");
        assert!(query.contains("search=71738515"));
    }

    #[test]
    fn build_kingdom_member_request_trims_whitespace_fields() {
        let client = test_client();
        let request = client
            .build_kingdom_member_http_request(
                &KingdomMemberRequest::new(" 2026-02-17 ", " 2026-02-17 ", " 2804 ")
                    .with_search(" 71738515 "),
            )
            .expect("build request");
        let query = request.url().query().expect("query");
        assert!(query.contains("start=2026-02-17"));
        assert!(query.contains("end=2026-02-17"));
        assert!(query.contains("server_id=2804"));
        assert!(query.contains("search=71738515"));
    }

    #[test]
    fn new_rejects_empty_tokens() {
        let err = RokGtClient::new(RokGtConfig::new("", "b-token"))
            .expect_err("empty p token should fail");
        assert!(matches!(err, RokGtError::InvalidConfig { field: "p_authorization_token", .. }));
    }
}
