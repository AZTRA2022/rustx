//! Which pages may be in the window, and who may talk to the host.
//!
//! The Rust code runs in the host process and the page in the webview's,
//! so a remote page cannot touch the host except through the one door the
//! host opens: `window.ipc.postMessage`. This is where that door is
//! checked. Two rules, kept apart because they answer different questions:
//!
//! - **Navigation**: the application's own origin and the origins it
//!   allow-listed load in the window. A link to any other http or https
//!   address opens in the system browser. Everything else is refused.
//! - **IPC**: only the application's own origin. An allow-listed remote
//!   page may be shown, and may not call anything.

/// What to do with an address the window is asked to go to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Navigation {
    /// Load it in the window.
    Stay,
    /// Give it to the system browser and leave the window where it is.
    OpenExternally,
    /// Do nothing, and say why.
    Refuse(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// The application's own origin, as the webview writes it.
    origin: String,
    /// Origins that may load in the window, without native access.
    allowed: Vec<String>,
}

impl Policy {
    pub fn new(origin: &str, allowed: &[&str]) -> Self {
        Self {
            origin: origin_of(origin).unwrap_or_else(|| origin.to_ascii_lowercase()),
            allowed: allowed.iter().filter_map(|each| origin_of(each)).collect(),
        }
    }

    /// The application's own origin.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn navigation(&self, url: &str) -> Navigation {
        let url = url.trim();
        // The blank page every frame starts from.
        if url.eq_ignore_ascii_case("about:blank") {
            return Navigation::Stay;
        }
        let Some(origin) = origin_of(url) else {
            return Navigation::Refuse(format!(
                "`{}` is not an address a rustx window opens",
                front(url)
            ));
        };
        if origin == self.origin || self.allowed.contains(&origin) {
            return Navigation::Stay;
        }
        match origin.starts_with("http://") || origin.starts_with("https://") {
            true => Navigation::OpenExternally,
            false => Navigation::Refuse(format!(
                "the origin `{origin}` is not the application's and is not allowed in the window"
            )),
        }
    }

    /// May a message sent from this page be acted on?
    pub fn ipc_allowed(&self, page_url: &str) -> bool {
        origin_of(page_url).is_some_and(|origin| origin == self.origin)
    }
}

/// `scheme://host[:port]`, lowercased, or `None` for what has no origin
/// (`javascript:`, `data:`, a bare word).
///
/// Written by hand because the one thing that must not happen is a parser
/// that disagrees with the browser's about where the host ends. A browser
/// reads `\` as `/` in these schemes and takes the host after the last `@`
/// of the authority; both are done here, so `http://app.localhost@evil.test`
/// is `evil.test`, and `http://app.localhost\@evil.test` is `app.localhost`
/// with a path, which is what the browser loads.
pub fn origin_of(url: &str) -> Option<String> {
    let url = url.trim();
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return None;
    }
    let end = rest
        .find(['/', '\\', '?', '#'])
        .unwrap_or(rest.len());
    let authority = &rest[..end];
    let host = authority.rsplit('@').next().unwrap_or(authority);
    if host.is_empty() {
        return None;
    }
    Some(format!(
        "{}://{}",
        scheme.to_ascii_lowercase(),
        host.to_ascii_lowercase()
    ))
}

/// The front of an address, for a message: all of it may be long and may
/// hold a token.
fn front(url: &str) -> String {
    url.chars().take(24).collect()
}
