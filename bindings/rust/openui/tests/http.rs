//! HTTP requests against a local server: what is sent, what comes back, and
//! how failures are reported. One test reaches the internet when
//! `OPENUI_NETWORK_TESTS=1`, to check certificates against the system's
//! roots.
#![cfg(feature = "http")]

use openui::http::{self, Client, Error};
use openui::task::spawn_local;
use openui::{create_signal, HeadlessApp, ViewportMetrics};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// A request as the server read it.
#[derive(Debug)]
struct Seen {
    request_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// Serve one connection: read a request, send it to the test, wait
/// `delay`, then write `response` verbatim.
fn serve(response: &'static str, delay: Duration) -> (String, mpsc::Receiver<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        if let Some(seen) = read_request(&stream) {
            let _ = sender.send(seen);
        }
        thread::sleep(delay);
        let mut stream = stream;
        let _ = stream.write_all(response.as_bytes());
    });
    (address, receiver)
}

fn read_request(stream: &TcpStream) -> Option<Seen> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map_or(0, |(_, value)| value.parse().unwrap());
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some(Seen {
        request_line: request_line.trim_end().to_owned(),
        headers,
        body,
    })
}

const HELLO: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-Note: héllo\r\n\
Content-Length: 12\r\nConnection: close\r\n\r\nhello, world";

#[test]
fn a_get_sends_query_and_headers_and_reads_the_response() {
    let (address, seen) = serve(HELLO, Duration::ZERO);
    let client = Client::builder()
        .header("X-Client", "openui")
        .user_agent("openui-test")
        .build();
    let response = client
        .get(format!("{address}/path"))
        .query("q", "a b&c")
        .header("X-Request", "1")
        .send_blocking()
        .unwrap();
    let seen = seen.recv().unwrap();
    assert_eq!(seen.request_line, "GET /path?q=a%20b%26c HTTP/1.1");
    assert_eq!(seen.header("x-client"), Some("openui"));
    assert_eq!(seen.header("x-request"), Some("1"));
    assert_eq!(seen.header("user-agent"), Some("openui-test"));
    assert_eq!(response.status(), 200);
    assert!(response.is_success());
    assert_eq!(response.text().unwrap(), "hello, world");
    assert_eq!(response.header("content-type"), Some("text/plain"));
    assert_eq!(response.url(), format!("{address}/path?q=a%20b%26c"));
}

#[test]
fn json_bodies_round_trip() {
    let (address, seen) = serve(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\n\
Content-Length: 21\r\nConnection: close\r\n\r\n{\"id\":7,\"tags\":[\"a\"]}",
        Duration::ZERO,
    );
    let response = http::post(format!("{address}/notes"))
        .json(&serde_json::json!({ "text": "ünïcode" }))
        .send_blocking()
        .unwrap();
    let seen = seen.recv().unwrap();
    assert_eq!(seen.request_line, "POST /notes HTTP/1.1");
    assert_eq!(seen.header("content-type"), Some("application/json"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&seen.body).unwrap(),
        serde_json::json!({ "text": "ünïcode" })
    );
    assert_eq!(response.status(), 201);
    let value: serde_json::Value = response.json().unwrap();
    assert_eq!(value, serde_json::json!({ "id": 7, "tags": ["a"] }));
    assert!(matches!(response.json::<Vec<u8>>(), Err(Error::Decode(_))));
}

#[test]
fn error_statuses_are_responses_until_checked() {
    let (address, _seen) = serve(
        "HTTP/1.1 404 Not Found\r\nContent-Length: 4\r\nConnection: close\r\n\r\ngone",
        Duration::ZERO,
    );
    let response = http::get(&address).send_blocking().unwrap();
    assert_eq!(response.status(), 404);
    assert_eq!(response.text().unwrap(), "gone");
    assert_eq!(response.error_for_status(), Err(Error::Status(404)));
}

#[test]
fn timeouts_limits_and_refused_connections_are_errors() {
    let (address, _seen) = serve(HELLO, Duration::from_secs(5));
    let started = Instant::now();
    let error = http::get(&address)
        .timeout(Duration::from_millis(300))
        .send_blocking()
        .unwrap_err();
    assert_eq!(error, Error::Timeout);
    assert!(started.elapsed() < Duration::from_secs(4));

    let (address, _seen) = serve(HELLO, Duration::ZERO);
    let small = Client::builder().max_body(4).build();
    assert_eq!(
        small.get(&address).send_blocking().unwrap_err(),
        Error::BodyTooLarge(4)
    );

    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let error = http::get(&address).send_blocking().unwrap_err();
    assert!(
        matches!(error, Error::Connect(_) | Error::Io(_)),
        "{error:?}"
    );

    assert!(matches!(
        http::get("not a url").send_blocking(),
        Err(Error::InvalidUrl(_))
    ));
}

#[test]
fn https_runs_tls_and_rejects_a_server_that_does_not_speak_it() {
    let (address, _seen) = serve(HELLO, Duration::ZERO);
    let address = address.replace("http://", "https://");
    let error = http::get(&address)
        .timeout(Duration::from_secs(10))
        .send_blocking()
        .unwrap_err();
    assert!(matches!(error, Error::Tls(_)), "{error:?}");
}

#[test]
fn a_ui_task_awaits_a_request_without_blocking_and_sets_a_signal() {
    let (address, _seen) = serve(HELLO, Duration::from_millis(100));
    let mut app =
        HeadlessApp::new(ViewportMetrics::from_logical_size(100.0, 50.0, 1.0).unwrap()).unwrap();
    let text = create_signal(String::new());
    spawn_local(async move {
        let response = http::get(address).send().await.unwrap();
        text.set(response.text().unwrap());
    })
    .detach();
    assert_eq!(text.get(), "");
    app.run_until_idle().unwrap();
    assert_eq!(text.get(), "hello, world");
}

#[test]
fn https_certificates_are_checked_against_the_system_roots() {
    if std::env::var_os("OPENUI_NETWORK_TESTS").is_none() {
        eprintln!("skipped: set OPENUI_NETWORK_TESTS=1 to reach the internet");
        return;
    }
    let response = http::get("https://example.com/")
        .timeout(Duration::from_secs(30))
        .send_blocking()
        .unwrap();
    assert!(response.is_success(), "{}", response.status());
    let error = http::get("https://self-signed.badssl.com/")
        .timeout(Duration::from_secs(30))
        .send_blocking()
        .unwrap_err();
    assert!(matches!(error, Error::Tls(_)), "{error:?}");
}
