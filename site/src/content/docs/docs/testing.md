---
title: Testing
description: Test views headless, click and type like a user, and compare pixels with snapshots.
---

Open UI apps can be tested without opening a window. A test mounts a view,
finds elements by text or id, clicks and types through real hit testing,
and compares the rendered frame with an accepted snapshot.

Enable the `testing` feature for tests only:

```toml
[dev-dependencies]
openui = { git = "https://github.com/zhuowcui/open-ui", features = ["testing"] }
```

```rust
use openui::prelude::*;
use openui::testing::TestApp;

#[test]
fn counts_clicks() {
    let count = create_signal(0);
    let mut app = TestApp::new(LogicalSize::new(200.0, 100.0)).mount(move || view! {
        <button on:click={move |_| count.set(count.get() + 1)}>"Add"</button>
    });
    app.click(&app.by_text("Add"));
    assert_eq!(count.get(), 1);
    openui::assert_snapshot!(app, "after-one-click");
}
```

## Snapshots

`assert_snapshot!` compares the frame, pixel for pixel, with
`tests/snapshots/<name>.<os>.png`. The first time, record it:

```sh
OPENUI_UPDATE_SNAPSHOTS=1 cargo test
```

Look at the image, commit it, and from then on `cargo test` fails on any
change and writes the new frame next to the snapshot as
`<name>.<os>.actual.png`, so you can see what moved. Snapshots are kept per
operating system because system fonts differ.

Apps made with `openui new` already have a test like this in `tests/ui.rs`.
