use openui::prelude::*;

fn main() -> Result<(), Error> {
    App::new("com.example.counter")
        .name("Counter")
        .window(
            WindowSpec::new("main").title("Counter").size(480.0, 320.0),
            counter,
        )
        .run()
}

fn counter() -> ViewNode {
    let count = create_signal(0);
    view! {
        <div style:padding="24px" style:font-family="sans-serif">
            <button
                style:padding="8px 16px"
                style:border="1px solid #d4d4d8"
                style:border-radius="8px"
                on:click={move |_| count.set(count.get() + 1)}
            >
                {format!("Clicked {} times", count.get())}
            </button>
        </div>
    }
}
