use std::time::Duration;

use openui::http;
use openui::prelude::*;
use openui::task::spawn_local;

fn latest_release() -> ViewNode {
    let status = create_signal(String::from("Checking…"));
    let request = http::get("https://api.github.com/repos/zhuowcui/open-ui/releases/latest")
        .header("accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20));
    spawn_local(async move {
        // send() runs the request off the UI thread; every status is a Response.
        let result = match request
            .send()
            .await
            .and_then(|response| response.error_for_status())
        {
            Ok(response) => match response.json::<serde_json::Value>() {
                Ok(json) => json["tag_name"].as_str().unwrap_or("unknown").to_string(),
                Err(error) => error.to_string(),
            },
            Err(error) => error.to_string(),
        };
        status.set(result);
    })
    .detach();
    view! { <p>{status.get()}</p> }
}

fn shared_client() -> http::Client {
    http::Client::builder()
        .user_agent("notes/1.0")
        .connect_timeout(Duration::from_secs(10))
        .build()
}
