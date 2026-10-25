pub(crate) fn validate(provider: &str, endpoint: &str) -> Result<(), crate::ProviderError> {
    let invalid =
        || crate::ProviderError::config(provider, "test client requires numeric loopback HTTP");
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
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(invalid());
    }
    Ok(())
}
