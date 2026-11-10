//! HTTP requests that never block the UI thread.
//!
//! [`Request::send`] runs the request on the blocking pool and resolves on
//! the task that awaits it, so a UI task can update signals with the
//! result. [`Request::send_blocking`] runs it on the calling thread, for
//! code already off the UI thread.
//!
//! TLS is rustls with the ring provider, and certificates are checked
//! against the operating system's roots: Windows' certificate store, and
//! the system bundle on Linux. Every status code is a [`Response`]; use
//! [`Response::error_for_status`] to treat 4xx and 5xx as errors.
//!
//! ```no_run
//! use openui::http;
//! use std::time::Duration;
//!
//! # async fn demo() -> Result<(), http::Error> {
//! let page = http::get("https://example.com").send().await?.text()?;
//!
//! let client = http::Client::builder()
//!     .timeout(Duration::from_secs(10))
//!     .header("Accept", "application/json")
//!     .build();
//! let response = client
//!     .post("https://example.com/notes")
//!     .json(&serde_json::json!({ "text": page.len() }))
//!     .send()
//!     .await?
//!     .error_for_status()?;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use ureq::http as h;

/// The largest response body read by default: 10 MiB.
pub const DEFAULT_MAX_BODY: u64 = 10 * 1024 * 1024;

/// A request to `url` with the shared default client.
pub fn get(url: impl Into<String>) -> Request {
    default_client().get(url)
}

/// A request to `url` with the shared default client.
pub fn post(url: impl Into<String>) -> Request {
    default_client().post(url)
}

/// A request to `url` with the shared default client.
pub fn put(url: impl Into<String>) -> Request {
    default_client().put(url)
}

/// A request to `url` with the shared default client.
pub fn patch(url: impl Into<String>) -> Request {
    default_client().patch(url)
}

/// A request to `url` with the shared default client.
pub fn delete(url: impl Into<String>) -> Request {
    default_client().delete(url)
}

fn default_client() -> Client {
    static DEFAULT: OnceLock<Client> = OnceLock::new();
    DEFAULT.get_or_init(|| Client::builder().build()).clone()
}

/// Shared connection settings. Cloning is cheap and clones share
/// connections.
#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    headers: Arc<[(String, String)]>,
    max_body: u64,
}

impl fmt::Debug for Client {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Client")
            .field("headers", &self.headers)
            .field("max_body", &self.max_body)
            .finish_non_exhaustive()
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// A request with `method`, such as `"GET"` or `"OPTIONS"`.
    pub fn request(&self, method: &str, url: impl Into<String>) -> Request {
        Request {
            client: self.clone(),
            method: method.to_owned(),
            url: url.into(),
            query: Vec::new(),
            headers: Vec::new(),
            body: Ok(None),
            timeout: None,
        }
    }

    pub fn get(&self, url: impl Into<String>) -> Request {
        self.request("GET", url)
    }

    pub fn post(&self, url: impl Into<String>) -> Request {
        self.request("POST", url)
    }

    pub fn put(&self, url: impl Into<String>) -> Request {
        self.request("PUT", url)
    }

    pub fn patch(&self, url: impl Into<String>) -> Request {
        self.request("PATCH", url)
    }

    pub fn delete(&self, url: impl Into<String>) -> Request {
        self.request("DELETE", url)
    }
}

/// Settings for a [`Client`].
#[derive(Debug, Clone)]
#[must_use]
pub struct ClientBuilder {
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    headers: Vec<(String, String)>,
    user_agent: Option<String>,
    max_redirects: u32,
    max_body: u64,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            timeout: None,
            connect_timeout: Some(Duration::from_secs(30)),
            headers: Vec::new(),
            user_agent: None,
            max_redirects: 10,
            max_body: DEFAULT_MAX_BODY,
        }
    }
}

impl ClientBuilder {
    /// The longest a whole request may take, from connecting to reading
    /// the body. Unlimited by default.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// The longest connecting may take. 30 seconds by default.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Send this header with every request.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// How many redirects to follow; 0 returns redirects as responses.
    pub fn max_redirects(mut self, redirects: u32) -> Self {
        self.max_redirects = redirects;
        self
    }

    /// The largest response body to read, in bytes. A larger body fails
    /// with [`Error::BodyTooLarge`].
    pub fn max_body(mut self, bytes: u64) -> Self {
        self.max_body = bytes;
        self
    }

    pub fn build(self) -> Client {
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::Rustls)
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .build();
        let mut config = ureq::Agent::config_builder()
            .tls_config(tls)
            .http_status_as_error(false)
            .timeout_global(self.timeout)
            .timeout_connect(self.connect_timeout)
            .max_redirects(self.max_redirects);
        if let Some(user_agent) = self.user_agent {
            config = config.user_agent(user_agent);
        }
        Client {
            agent: config.build().new_agent(),
            headers: self.headers.into(),
            max_body: self.max_body,
        }
    }
}

/// A request being built. Nothing is sent until [`Request::send`].
#[must_use]
pub struct Request {
    client: Client,
    method: String,
    url: String,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: Result<Option<Vec<u8>>, Error>,
    timeout: Option<Duration>,
}

impl fmt::Debug for Request {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Request")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("query", &self.query)
            .field("headers", &self.headers)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl Request {
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Add `name=value` to the URL's query, percent-encoded.
    pub fn query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((name.into(), value.into()));
        self
    }

    /// The longest this request may take, replacing the client's timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Send `body` as is.
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Ok(Some(body.into()));
        self
    }

    /// Send `text` as `text/plain; charset=utf-8`.
    pub fn text(self, text: impl Into<String>) -> Self {
        self.content_type("text/plain; charset=utf-8")
            .body(text.into())
    }

    /// Send `value` as JSON. A value that cannot be serialized makes
    /// [`send`](Self::send) fail with [`Error::Decode`].
    pub fn json<T: serde::Serialize + ?Sized>(mut self, value: &T) -> Self {
        self = self.content_type("application/json");
        self.body = serde_json::to_vec(value)
            .map(Some)
            .map_err(|error| Error::Decode(error.to_string()));
        self
    }

    fn content_type(self, value: &str) -> Self {
        let set = self
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("content-type"));
        if set {
            self
        } else {
            self.header("Content-Type", value)
        }
    }

    /// Send the request on the blocking pool and read the whole response.
    pub async fn send(self) -> Result<Response, Error> {
        crate::task::spawn_blocking(move || self.send_blocking()).await
    }

    /// Send the request on this thread and read the whole response. Do not
    /// call it on the UI thread.
    pub fn send_blocking(self) -> Result<Response, Error> {
        let Request {
            client,
            method,
            url,
            query,
            headers,
            body,
            timeout,
        } = self;
        let body = body?;
        let url = with_query(&url, &query);
        let mut builder = h::Request::builder().method(method.as_str()).uri(&url);
        for (name, value) in client.headers.iter().chain(&headers) {
            builder = builder.header(name.as_str(), value.as_str());
        }
        let result = match body {
            Some(body) => run(&client, builder.body(body), timeout),
            None => run(&client, builder.body(()), timeout),
        };
        let mut response = result?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();
        let body = response
            .body_mut()
            .with_config()
            .limit(client.max_body)
            .read_to_vec()
            .map_err(Error::from)?;
        Ok(Response {
            status,
            url,
            headers,
            body,
        })
    }
}

fn run<S: ureq::AsSendBody>(
    client: &Client,
    request: Result<h::Request<S>, h::Error>,
    timeout: Option<Duration>,
) -> Result<h::Response<ureq::Body>, Error> {
    let request = request.map_err(|error| Error::InvalidUrl(error.to_string()))?;
    let request = match timeout {
        Some(timeout) => client
            .agent
            .configure_request(request)
            .timeout_global(Some(timeout))
            .build(),
        None => request,
    };
    client.agent.run(request).map_err(Error::from)
}

/// Append `query` to `url`, percent-encoding each name and value.
fn with_query(url: &str, query: &[(String, String)]) -> String {
    if query.is_empty() {
        return url.to_owned();
    }
    let (base, fragment) = match url.split_once('#') {
        Some((base, fragment)) => (base, Some(fragment)),
        None => (url, None),
    };
    let mut result = base.to_owned();
    let mut separator = if base.contains('?') { '&' } else { '?' };
    if base.ends_with('?') || base.ends_with('&') {
        separator = '\0';
    }
    for (name, value) in query {
        if separator != '\0' {
            result.push(separator);
        }
        separator = '&';
        result.push_str(&percent_encode(name));
        result.push('=');
        result.push_str(&percent_encode(value));
    }
    if let Some(fragment) = fragment {
        result.push('#');
        result.push_str(fragment);
    }
    result
}

fn percent_encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// A whole response: status, headers and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    status: u16,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Whether the status is 2xx.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// This response, or [`Error::Status`] for a 4xx or 5xx status.
    pub fn error_for_status(self) -> Result<Self, Error> {
        if self.status >= 400 {
            Err(Error::Status(self.status))
        } else {
            Ok(self)
        }
    }

    /// The requested URL, with its query.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The first value of header `name`, ignoring case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Every header, in the order received; names are lowercase.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    pub fn bytes(&self) -> &[u8] {
        &self.body
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.body
    }

    /// The body as UTF-8 text.
    pub fn text(&self) -> Result<String, Error> {
        String::from_utf8(self.body.clone()).map_err(|error| Error::Decode(error.to_string()))
    }

    /// The body parsed as JSON.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, Error> {
        serde_json::from_slice(&self.body).map_err(|error| Error::Decode(error.to_string()))
    }
}

/// Why a request failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The URL, method or a header is not valid.
    InvalidUrl(String),
    /// The host name did not resolve.
    HostNotFound,
    /// No connection could be made.
    Connect(String),
    /// The request took longer than its timeout.
    Timeout,
    /// The TLS handshake failed, for example on an untrusted certificate.
    Tls(String),
    /// The server's response was not valid HTTP, or redirects failed.
    Protocol(String),
    /// The body was larger than the client's limit, in bytes.
    BodyTooLarge(u64),
    /// [`Response::error_for_status`] found a 4xx or 5xx status.
    Status(u16),
    /// The body was not valid UTF-8 or JSON, or a JSON body could not be
    /// serialized.
    Decode(String),
    /// Reading or writing the connection failed.
    Io(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(detail) => write!(formatter, "invalid request: {detail}"),
            Self::HostNotFound => formatter.write_str("host not found"),
            Self::Connect(detail) => write!(formatter, "could not connect: {detail}"),
            Self::Timeout => formatter.write_str("the request timed out"),
            Self::Tls(detail) => write!(formatter, "TLS failed: {detail}"),
            Self::Protocol(detail) => write!(formatter, "invalid response: {detail}"),
            Self::BodyTooLarge(limit) => {
                write!(formatter, "the response body is larger than {limit} bytes")
            }
            Self::Status(status) => write!(formatter, "the server answered {status}"),
            Self::Decode(detail) => write!(formatter, "could not decode the body: {detail}"),
            Self::Io(detail) => write!(formatter, "connection failed: {detail}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ureq::Error> for Error {
    fn from(error: ureq::Error) -> Self {
        match error {
            ureq::Error::StatusCode(status) => Self::Status(status),
            ureq::Error::Http(error) => Self::InvalidUrl(error.to_string()),
            ureq::Error::BadUri(detail) => Self::InvalidUrl(detail),
            ureq::Error::RequireHttpsOnly(detail) => Self::InvalidUrl(detail),
            ureq::Error::Timeout(_) => Self::Timeout,
            ureq::Error::HostNotFound => Self::HostNotFound,
            ureq::Error::ConnectionFailed => Self::Connect("connection failed".into()),
            ureq::Error::BodyExceedsLimit(limit) => Self::BodyTooLarge(limit),
            ureq::Error::Tls(detail) => Self::Tls(detail.into()),
            ureq::Error::Rustls(error) => Self::Tls(error.to_string()),
            ureq::Error::Pem(error) => Self::Tls(error.to_string()),
            ureq::Error::Io(error) => match error.kind() {
                std::io::ErrorKind::TimedOut => Self::Timeout,
                std::io::ErrorKind::ConnectionRefused => Self::Connect(error.to_string()),
                _ => match tls_error(&error) {
                    Some(detail) => Self::Tls(detail),
                    None => Self::Io(error.to_string()),
                },
            },
            error @ (ureq::Error::Protocol(_)
            | ureq::Error::RedirectFailed
            | ureq::Error::TooManyRedirects
            | ureq::Error::LargeResponseHeader(..)) => Self::Protocol(error.to_string()),
            error => Self::Io(error.to_string()),
        }
    }
}

/// The rustls error inside an I/O error, as a handshake failure reports it.
fn tls_error(error: &std::io::Error) -> Option<String> {
    let inner = error.get_ref()?;
    inner
        .downcast_ref::<rustls::Error>()
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_are_appended_and_encoded() {
        let query = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect::<Vec<_>>()
        };
        assert_eq!(with_query("http://a/b", &[]), "http://a/b");
        assert_eq!(
            with_query("http://a/b", &query(&[("q", "a b&c"), ("n", "é")])),
            "http://a/b?q=a%20b%26c&n=%C3%A9"
        );
        assert_eq!(
            with_query("http://a/b?x=1#top", &query(&[("y", "2")])),
            "http://a/b?x=1&y=2#top"
        );
        assert_eq!(
            with_query("http://a/b?", &query(&[("y", "~._-")])),
            "http://a/b?y=~._-"
        );
    }

    #[test]
    fn status_errors_are_opt_in() {
        let response = |status| Response {
            status,
            url: String::new(),
            headers: vec![("content-type".into(), "text/plain".into())],
            body: b"ok".to_vec(),
        };
        assert!(response(204).is_success());
        assert!(response(302).error_for_status().is_ok());
        assert_eq!(response(404).error_for_status(), Err(Error::Status(404)));
        assert_eq!(response(200).header("Content-Type"), Some("text/plain"));
        assert_eq!(response(200).text().unwrap(), "ok");
    }
}
