//! Main-frame navigation policy; subresources remain under WebView2's control.

pub(crate) const HOME: &str = "https://discord.com/app";

#[cfg(windows)]
pub(crate) fn browser_url(input: &str) -> Option<url::Url> {
    let url = url::Url::parse(input).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then_some(url)
}

pub(crate) fn allowed(input: &str) -> bool {
    let Ok(url) = url::Url::parse(input) else {
        return false;
    };
    url.scheme() == "https"
        && url.host_str() == Some("discord.com")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}
