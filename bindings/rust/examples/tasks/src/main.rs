//! UI and background tasks.
//!
//! A clock ticks from a UI task, a button runs a computation on the
//! background pool, and a resource re-runs when its input changes. None of
//! it blocks input or rendering.
//!
//! Set `OUI_SMOKE_EXIT=1` to exit once the first computation and resource
//! fetch have finished, as CI does: the app only exits if wakeups from
//! background threads and timers reach the event loop.

use openui::prelude::*;
use openui::task::{spawn, spawn_local, time};
use openui::window::WindowSpec;
use openui::{Resource, create_resource};
use std::time::Duration;

/// Count primes below `limit`, slowly enough to show it runs off the UI thread.
fn count_primes(limit: u64) -> u64 {
    (2..limit)
        .filter(|n| (2..).take_while(|d| d * d <= *n).all(|d| n % d != 0))
        .count() as u64
}

fn app_view() -> ViewNode {
    let seconds = create_signal(0_u64);
    spawn_local(async move {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            seconds.update(|s| *s += 1);
        }
    })
    .detach();

    let primes = create_signal(None::<u64>);
    let limit = create_signal(200_000_u64);
    let compute = move || {
        primes.set(None);
        let limit = limit.get();
        spawn_local(async move {
            let count = spawn(async move { count_primes(limit) }).await;
            primes.set(Some(count));
        })
        .detach();
    };
    compute();

    let size = create_signal(3_u32);
    let square: Resource<u32, String> = create_resource(
        move || size.get(),
        |size| async move {
            time::sleep(Duration::from_millis(300)).await;
            Ok(size * size)
        },
    );

    if std::env::var_os("OUI_SMOKE_EXIT").is_some() {
        spawn_local(async move {
            while primes.get().is_none() || square.loading() {
                time::sleep(Duration::from_millis(20)).await;
            }
            println!("tasks: primes={:?} square={:?}", primes.get(), square.get());
            openui::app::exit();
        })
        .detach();
    }

    view! {
        <div style:font-family="sans-serif" style:padding="16px">
            <p>{format!("Running for {} s", seconds.get())}</p>
            <p>{match primes.get() {
                Some(count) => format!("{count} primes below {}", limit.get()),
                None => format!("Counting primes below {}...", limit.get()),
            }}</p>
            <button style:padding="6px 14px" style:margin="4px" on:click={move |_| {
                limit.update(|l| *l *= 2);
                compute();
            }}>"Count further"</button>
            <p>{match (square.loading(), square.get()) {
                (true, _) => format!("Squaring {}...", size.get()),
                (false, Some(Ok(value))) => format!("{} squared is {value}", size.get()),
                (false, _) => String::new(),
            }}</p>
            <button style:padding="6px 14px" style:margin="4px" on:click={move |_| {
                size.update(|s| *s += 1);
            }}>"Next square"</button>
        </div>
    }
}

#[cfg(any(feature = "linux", feature = "windows"))]
fn main() -> Result<(), Error> {
    App::new("dev.openui.examples.tasks")
        .name("Tasks")
        .window(WindowSpec::new("main").size(420.0, 260.0), app_view)
        .run()
}

#[cfg(not(any(feature = "linux", feature = "windows")))]
fn main() -> Result<(), Error> {
    let mut app = HeadlessApp::new(ViewportMetrics::from_logical_size(420.0, 260.0, 1.0)?)?;
    app.mount(app_view)?;
    // The clock never finishes, so run only until the other work is done.
    for _ in 0..50 {
        app.run_ready_tasks()?;
        std::thread::sleep(Duration::from_millis(20));
    }
    app.render_png_to(0.0, "tasks.png")?;
    println!("Rendered tasks.png");
    Ok(())
}
