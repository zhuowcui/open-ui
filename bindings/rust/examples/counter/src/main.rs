//! Reactive Counter — demonstrates signals, effects, and event handlers.
//!
//! Creates a count signal and three buttons (Increment, Decrement, Reset)
//! that modify it. The displayed count updates reactively via `{count.get()}`
//! dynamic expressions in the `view!` macro.

use openui::prelude::*;

fn main() {
    let mut app = App::new(800, 600);

    app.render(|| {
        // Reactive state: the counter value.
        let count = create_signal(0_i32);

        view! {
            <div
                style:display="flex"
                style:flex-direction="column"
                style:align-items="center"
                style:justify-content="center"
                style:min-height="100vh"
                style:background-color="#f7fafc"
                style:font-family="sans-serif"
            >
                <h1 style:font-size="24px" style:color="#2d3748" style:margin-bottom="16px">
                    "Reactive Counter"
                </h1>

                // Dynamic count display — re-renders when count changes.
                <div style:font-size="72px" style:font-weight="700"
                     style:color="#2b6cb0" style:margin="16px">
                    {count.get()}
                </div>

                // Button row
                <div style:display="flex" style:gap="12px" style:margin-top="16px">
                    <button
                        style:padding="10px 24px"
                        style:font-size="18px"
                        style:background-color="#e53e3e"
                        style:color="white"
                        style:border="none"
                        style:border-radius="6px"
                        style:cursor="pointer"
                        on:click={move |_| count.set(count.get() - 1)}
                    >
                        "- Decrement"
                    </button>

                    <button
                        style:padding="10px 24px"
                        style:font-size="18px"
                        style:background-color="#718096"
                        style:color="white"
                        style:border="none"
                        style:border-radius="6px"
                        style:cursor="pointer"
                        on:click={move |_| count.set(0)}
                    >
                        "Reset"
                    </button>

                    <button
                        style:padding="10px 24px"
                        style:font-size="18px"
                        style:background-color="#38a169"
                        style:color="white"
                        style:border="none"
                        style:border-radius="6px"
                        style:cursor="pointer"
                        on:click={move |_| count.set(count.get() + 1)}
                    >
                        "+ Increment"
                    </button>
                </div>

                // Status line using formatted dynamic text
                <p style:font-size="14px" style:color="#a0aec0" style:margin-top="24px">
                    {format!("Current value: {}", count.get())}
                </p>
            </div>
        }
    });

    // Render initial state (count = 0).
    app.run_frames(1).render_to_png("counter.png");

    println!("Rendered counter.png");
}
