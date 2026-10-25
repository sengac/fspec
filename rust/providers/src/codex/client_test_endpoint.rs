use super::rewrite_codex_url;

pub(super) fn url_for_request(original: &str, extensions: &http::Extensions) -> String {
    #[cfg(feature = "test-support")]
    if let Some(endpoint) = extensions.get::<LocalEndpoint>() {
        return endpoint.0.to_string();
    }
    #[cfg(not(feature = "test-support"))]
    let _ = extensions;
    rewrite_codex_url(original)
}

#[cfg(feature = "test-support")]
#[derive(Clone, Debug)]
pub(super) struct LocalEndpoint(http::Uri);

#[cfg(feature = "test-support")]
impl super::RefreshingCodexClient {
    pub fn with_test_endpoint(mut self, endpoint: &str) -> Result<Self, crate::ProviderError> {
        let invalid =
            || crate::ProviderError::config("codex", "test endpoint must be numeric loopback HTTP");
        let parsed = url::Url::parse(endpoint).map_err(|_| invalid())?;
        let loopback = match parsed.host() {
            Some(url::Host::Ipv4(address)) => address.is_loopback(),
            Some(url::Host::Ipv6(address)) => address.is_loopback(),
            _ => false,
        };
        if parsed.scheme() != "http"
            || !loopback
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.fragment().is_some()
        {
            return Err(invalid());
        }
        self.test_endpoint = Some(LocalEndpoint(endpoint.parse().map_err(|_| invalid())?));
        Ok(self)
    }

    pub(super) fn test_request<T>(&self, mut request: http::Request<T>) -> http::Request<T> {
        if let Some(endpoint) = &self.test_endpoint {
            request.extensions_mut().insert(endpoint.clone());
        }
        request
    }

    pub fn test_request_url(&self, original: &str) -> String {
        let request = self.test_request(http::Request::new(()));
        url_for_request(original, request.extensions())
    }
}
