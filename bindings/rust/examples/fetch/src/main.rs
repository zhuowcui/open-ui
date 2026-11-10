//! HTTP requests from the UI.
//!
//! "Fetch" downloads a page on the blocking pool and shows its status, type,
//! size and opening text; the window stays responsive meanwhile. The URL is
//! `https://example.com/` unless `OPENUI_FETCH_URL` names another.
//!
//! Set `OUI_SMOKE_EXIT=1` to fetch once from a server inside this process
//! and exit, as CI does: no network is needed, and the app only exits if the
//! finished request wakes the event loop.

use openui::http;
use openui::prelude::*;
use openui::task::spawn_local;
use openui::window::WindowSpec;
use std::io::{Read, Write};
use std::time::Duration;

#[derive(Clone, PartialEq)]
enum State {
    Idle,
    Loading,
    Done(Result<Summary, String>),
}

#[derive(Clone, PartialEq)]
struct Summary {
    status: u16,
    content_type: String,
    bytes: usize,
    opening: String,
}

fn summarize(response: &http::Response) -> Summary {
    let text = String::from_utf8_lossy(response.bytes());
    Summary {
        status: response.status(),
        content_type: response.header("content-type").unwrap_or("").to_owned(),
        bytes: response.bytes().len(),
        opening: text.chars().take(160).collect(),
    }
}

/// Serve one canned response on a local port and return its URL.
fn local_server() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a local port");
    let url = format!("http://{}/", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut request = [0; 1024];
            let _ = stream.read(&mut request);
            let body = "<p>Served from inside the fetch example.</p>";
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    url
}

fn app_view() -> ViewNode {
    let smoke = std::env::var_os("OUI_SMOKE_EXIT").is_some();
    let url = if smoke {
        local_server()
    } else {
        std::env::var("OPENUI_FETCH_URL").unwrap_or_else(|_| "https://example.com/".into())
    };
    let state = create_signal(State::Idle);
    let fetch = {
        let url = url.clone();
        move || {
            state.set(State::Loading);
            let request = http::get(url.clone()).timeout(Duration::from_secs(30));
            spawn_local(async move {
                let result = request.send().await;
                let summary = result.as_ref().map(summarize).map_err(ToString::to_string);
                if smoke {
                    match &summary {
                        Ok(summary) => println!("fetch: {} {:?}", summary.status, summary.opening),
                        Err(error) => println!("fetch: failed: {error}"),
                    }
                    openui::app::exit();
                }
                state.set(State::Done(summary));
            })
            .detach();
        }
    };
    if smoke {
        fetch();
    }

    view! {
        <div style:font-family="sans-serif" style:padding="16px">
            <p>{url}</p>
            <button style:padding="6px 14px" on:click={move |_| fetch()}>"Fetch"</button>
            <p>{match state.get() {
                State::Idle => String::new(),
                State::Loading => "Loading...".to_owned(),
                State::Done(Ok(summary)) => format!(
                    "{} {}, {} bytes",
                    summary.status, summary.content_type, summary.bytes
                ),
                State::Done(Err(error)) => format!("Failed: {error}"),
            }}</p>
            <p style:color="#555" style:font-size="12px">{match state.get() {
                State::Done(Ok(summary)) => summary.opening,
                _ => String::new(),
            }}</p>
        </div>
    }
}

#[cfg(any(feature = "linux", feature = "windows"))]
fn main() -> Result<(), Error> {
    App::new("dev.openui.examples.fetch")
        .name("Fetch")
        .window(WindowSpec::new("main").size(520.0, 260.0), app_view)
        .run()
}

#[cfg(not(any(feature = "linux", feature = "windows")))]
fn main() {
    println!("fetch: build with --features windows or --features linux to open a window");
}
