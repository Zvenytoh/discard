//! Main-frame navigation policy; subresources remain under WebView2's control.

pub(crate) const HOME: &str = "https://discord.com/app";

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
