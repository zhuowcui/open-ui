use openui::prelude::*;
use openui::testing::TestApp;

fn counter() -> ViewNode {
    let count = create_signal(0);
    view! {
        <button on:click={move |_| count.set(count.get() + 1)}>
            {format!("Clicked {} times", count.get())}
        </button>
    }
}

#[test]
fn clicking_counts() {
    let mut ui = TestApp::new(LogicalSize::new(400.0, 200.0)).mount(counter);
    let button = ui.by_text("Clicked 0 times");
    ui.click(&button);
    ui.click(&button);
    assert!(ui.find_text("Clicked 2 times").is_some());
}

#[test]
fn first_screen_matches_its_snapshot() {
    let ui = TestApp::new(LogicalSize::new(400.0, 200.0)).mount(counter);
    // Compares with tests/snapshots/first-screen.<os>.png.
    openui::assert_snapshot!(ui, "first-screen");
}
